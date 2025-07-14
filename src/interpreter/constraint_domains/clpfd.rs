//! CLPFD (Constraint Logic Programming over Finite Domains) Implementation
//!
//! This domain provides finite domain constraint syntax like:
//! - x in 1..10 (domain constraints)
//! - x + y == z (arithmetic constraints)  
//! - x < y (comparison constraints)
//! - distinct [x, y, z] (global constraints)

use super::{ConstraintDomain, DomainConstraints};
use crate::engine::Engine;
use crate::goal::{AnyGoal, Goal, GoalCast};
use crate::interpreter::execution::ExecutionContext;
use crate::interpreter::parser::ast::ConstraintBody;
use crate::interpreter::InterpreterError;
use crate::lterm::LTerm;
use crate::operator::conj::Conj;
use crate::user::User;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "interpreter/constraint_domains/grammars/clpfd.pest"]
pub struct ClpfdParser;

/// CLPFD constraint domain
pub struct ClpfdDomain;

impl ClpfdDomain {
    pub fn new() -> Self {
        Self
    }
}

impl<U: User, E: Engine<U>> ConstraintDomain<U, E> for ClpfdDomain {
    fn name(&self) -> &str {
        "clpfd"
    }

    fn parse_constraints(
        &self,
        body: &ConstraintBody,
    ) -> Result<Box<dyn DomainConstraints<U, E>>, InterpreterError> {
        // Trim the content to remove leading/trailing whitespace that might interfere
        let trimmed_content = body.raw_content.trim();

        let pairs = ClpfdParser::parse(Rule::constraints, trimmed_content).map_err(|e| {
            InterpreterError::InvalidConstraintSyntax {
                domain: "clpfd".to_string(),
                error: format!("Parse error: {}", e),
            }
        })?;

        let mut constraints = Vec::new();

        // Process all pairs to extract constraints
        for pair in pairs {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::constraint {
                    let constraint = Self::build_constraint(inner)?;
                    constraints.push(constraint);
                }
            }
        }

        Ok(Box::new(ClpfdConstraints { constraints }))
    }

    fn syntax_help(&self) -> &str {
        r#"CLPFD Syntax:
- Domain constraints: x in 1..10, x in [1,2,3], [x, y, z] in 0..2
- Arithmetic: x + y == z, x - y == z, x * y == z
- Comparison: x < y, x <= y, x > y, x >= y, x != y
- Global: distinct [x, y, z], alldiff [x, y, z]
- Fresh: |x, y| { x < y, x in 1..10 }"#
    }
}

impl ClpfdDomain {
    fn parse_individual_constraint<U: User, E: Engine<U>>(
        &self,
        content: &str,
    ) -> Result<ClpfdConstraint<U, E>, InterpreterError> {
        let pairs = ClpfdParser::parse(Rule::constraint, content.trim()).map_err(|e| {
            InterpreterError::InvalidConstraintSyntax {
                domain: "clpfd".to_string(),
                error: format!("Parse error: {}", e),
            }
        })?;

        // There should be exactly one pair, corresponding to the `constraint` rule
        if let Some(pair) = pairs.peek() {
            Self::build_constraint(pair)
        } else {
            Err(InterpreterError::InvalidConstraintSyntax {
                domain: "clpfd".to_string(),
                error: format!("No valid constraint found in: {}", content),
            })
        }
    }

    // -- AST Builder Functions --

    fn build_constraint<U: User, E: Engine<U>>(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClpfdConstraint<U, E>, InterpreterError> {
        let inner_pair = pair.into_inner().next().unwrap();
        match inner_pair.as_rule() {
            Rule::fresh_constraint => Self::build_fresh_constraint(inner_pair),
            Rule::domain_constraint => Self::build_domain_constraint(inner_pair),
            Rule::list_domain_constraint => Self::build_list_domain_constraint(inner_pair),
            Rule::distinct_constraint => Self::build_distinct_constraint(inner_pair),
            Rule::alldiff_constraint => Self::build_alldiff_constraint(inner_pair),
            Rule::arith_constraint => Self::build_arith_constraint(inner_pair),
            _ => unreachable!("Unexpected rule in constraint"),
        }
    }

    fn build_fresh_constraint<U: User, E: Engine<U>>(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClpfdConstraint<U, E>, InterpreterError> {
        let mut inner = pair.into_inner();
        let mut vars = vec![];
        let mut constraints = vec![];

        // The first part could be var_list or constraints
        while let Some(part) = inner.next() {
            match part.as_rule() {
                Rule::var_list => {
                    vars = part.into_inner().map(|v| v.as_str().to_string()).collect();
                }
                Rule::constraints => {
                    for constraint_pair in part.into_inner() {
                        if constraint_pair.as_rule() == Rule::constraint {
                            constraints.push(Self::build_constraint(constraint_pair)?);
                        }
                    }
                }
                _ => unreachable!("Unexpected rule in fresh_constraint"),
            }
        }

        Ok(ClpfdConstraint::Fresh {
            vars,
            constraints,
            _phantom: std::marker::PhantomData,
        })
    }

    fn build_domain_constraint<U: User, E: Engine<U>>(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClpfdConstraint<U, E>, InterpreterError> {
        let mut inner = pair.into_inner();
        let variable = inner.next().unwrap().as_str().to_string();
        let _in_kw = inner.next().unwrap(); // Skip the "in" keyword
        let range_spec_pair = inner.next().unwrap();
        let domain_spec = Self::build_domain_spec(range_spec_pair)?;

        Ok(ClpfdConstraint::Domain {
            variable,
            domain_spec,
            _phantom: std::marker::PhantomData,
        })
    }

    fn build_list_domain_constraint<U: User, E: Engine<U>>(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClpfdConstraint<U, E>, InterpreterError> {
        let mut inner = pair.into_inner();
        let var_list_pair = inner.next().unwrap(); // var_list
        let variables = var_list_pair
            .into_inner()
            .map(|p| p.as_str().to_string())
            .collect();
        let _in_kw = inner.next().unwrap(); // Skip the "in" keyword
        let range_spec_pair = inner.next().unwrap();
        let domain_spec = Self::build_domain_spec(range_spec_pair)?;

        Ok(ClpfdConstraint::ListDomain {
            variables,
            domain_spec,
            _phantom: std::marker::PhantomData,
        })
    }

    fn build_domain_spec(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<DomainSpec, InterpreterError> {
        let inner_pair = pair.into_inner().next().unwrap();
        match inner_pair.as_rule() {
            Rule::range_dotdot => {
                let mut inner = inner_pair.into_inner();
                let start = inner.next().unwrap().as_str().parse().unwrap();
                let end = inner.next().unwrap().as_str().parse().unwrap();
                Ok(DomainSpec::Range(start, end))
            }
            Rule::range_set => {
                let values = inner_pair
                    .into_inner()
                    .map(|p| p.as_str().parse().unwrap())
                    .collect();
                Ok(DomainSpec::Set(values))
            }
            Rule::range_single => {
                let value = inner_pair
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str()
                    .parse()
                    .unwrap();
                Ok(DomainSpec::Set(vec![value]))
            }
            _ => unreachable!("Unexpected rule in range_spec"),
        }
    }

    fn build_distinct_constraint<U: User, E: Engine<U>>(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClpfdConstraint<U, E>, InterpreterError> {
        let mut inner = pair.into_inner();

        // The grammar now excludes the keyword with _{ "distinct" }, so var_list is first
        let var_list_pair = inner.next().unwrap();

        let args = var_list_pair
            .into_inner()
            .map(|p| p.as_str().to_string())
            .collect();

        Ok(ClpfdConstraint::Global {
            name: "distinct".to_string(),
            args,
            _phantom: std::marker::PhantomData,
        })
    }

    fn build_alldiff_constraint<U: User, E: Engine<U>>(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClpfdConstraint<U, E>, InterpreterError> {
        let mut inner = pair.into_inner();

        // The grammar now excludes the keyword with _alldiff_kw, so var_or_const_list is first
        let var_list_pair = inner.next().unwrap();

        let args = var_list_pair
            .into_inner()
            .map(|p| p.as_str().to_string())
            .collect();

        Ok(ClpfdConstraint::Global {
            name: "alldiff".to_string(),
            args,
            _phantom: std::marker::PhantomData,
        })
    }

    fn build_arith_constraint<U: User, E: Engine<U>>(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClpfdConstraint<U, E>, InterpreterError> {
        let mut inner = pair.into_inner();
        let left = Self::build_arith_expr(inner.next().unwrap());
        let op = Self::build_comp_op(inner.next().unwrap());
        let right = Self::build_arith_expr(inner.next().unwrap());

        Ok(ClpfdConstraint::Expression {
            left,
            op,
            right,
            _phantom: std::marker::PhantomData,
        })
    }

    // This function uses the Pratt parser technique to handle operator precedence.
    fn build_arith_expr(pair: pest::iterators::Pair<Rule>) -> ArithExpr {
        let pratt = pest::pratt_parser::PrattParser::new()
            .op(pest::pratt_parser::Op::infix(
                Rule::add_op,
                pest::pratt_parser::Assoc::Left,
            ))
            .op(pest::pratt_parser::Op::infix(
                Rule::mul_op,
                pest::pratt_parser::Assoc::Left,
            ));

        pratt
            .map_primary(|primary| match primary.as_rule() {
                Rule::integer => ArithExpr::Integer(primary.as_str().parse().unwrap()),
                Rule::variable => ArithExpr::Variable(primary.as_str().to_string()),
                Rule::arith_expr => Self::build_arith_expr(primary), // for parentheses
                Rule::factor => {
                    // Handle factor rule by extracting its inner content
                    let inner = primary.into_inner().next().unwrap();
                    match inner.as_rule() {
                        Rule::integer => ArithExpr::Integer(inner.as_str().parse().unwrap()),
                        Rule::variable => ArithExpr::Variable(inner.as_str().to_string()),
                        Rule::arith_expr => Self::build_arith_expr(inner),
                        _ => unreachable!("Unexpected factor inner rule: {:?}", inner.as_rule()),
                    }
                }
                Rule::term => {
                    // Handle term = factor ~ (mul_op ~ factor)*
                    let mut inner = primary.into_inner();
                    let mut left = Self::build_arith_expr(inner.next().unwrap());

                    // Process any multiplication operations
                    while let Some(op_pair) = inner.next() {
                        if op_pair.as_rule() == Rule::mul_op {
                            let op = match op_pair.as_str() {
                                "*" => ArithOp::Multiply,
                                "/" => ArithOp::Divide,
                                _ => unreachable!(),
                            };
                            let right = Self::build_arith_expr(inner.next().unwrap());
                            left = ArithExpr::BinaryOp {
                                left: Box::new(left),
                                op,
                                right: Box::new(right),
                            };
                        }
                    }
                    left
                }
                _ => unreachable!("Unexpected primary rule: {:?}", primary.as_rule()),
            })
            .map_infix(|lhs, op, rhs| {
                let op = match op.as_rule() {
                    Rule::add_op => match op.as_str() {
                        "+" => ArithOp::Add,
                        "-" => ArithOp::Subtract,
                        _ => unreachable!(),
                    },
                    Rule::mul_op => match op.as_str() {
                        "*" => ArithOp::Multiply,
                        "/" => ArithOp::Divide,
                        _ => unreachable!(),
                    },
                    _ => unreachable!("Unexpected infix operator rule: {:?}", op.as_rule()),
                };
                ArithExpr::BinaryOp {
                    left: Box::new(lhs),
                    op,
                    right: Box::new(rhs),
                }
            })
            .parse(pair.into_inner())
    }

    fn build_comp_op(pair: pest::iterators::Pair<Rule>) -> CompOp {
        match pair.as_str() {
            "==" => CompOp::Equal,
            "!=" => CompOp::NotEqual,
            "<" => CompOp::LessThan,
            "<=" => CompOp::LessEqual,
            ">" => CompOp::GreaterThan,
            ">=" => CompOp::GreaterEqual,
            _ => unreachable!("Unexpected comparison operator: {}", pair.as_str()),
        }
    }
}

/// Parsed CLPFD constraints
struct ClpfdConstraints<U: User, E: Engine<U>> {
    constraints: Vec<ClpfdConstraint<U, E>>,
}

impl<U: User, E: Engine<U>> DomainConstraints<U, E> for ClpfdConstraints<U, E> {
    fn convert_to_goals(
        &self,
        execution_context: &mut ExecutionContext<U, E>,
    ) -> Result<Goal<U, E>, InterpreterError> {
        let goals: Result<Vec<_>, _> = self
            .constraints
            .iter()
            .map(|c| c.convert_to_goal(execution_context))
            .collect();
        let goals = goals?;

        if goals.is_empty() {
            return Ok(Goal::succeed());
        }

        // Fold the goals into a single conjunction: goal1 & (goal2 & (goal3 & ...))
        let mut iter = goals.into_iter();
        let first = iter.next().unwrap();

        Ok(iter.fold(first, |acc, next_goal| Conj::new(acc, next_goal)))
    }

    fn extract_variables(&self) -> Vec<String> {
        let mut vars = Vec::new();
        for constraint in &self.constraints {
            vars.extend(constraint.extract_variables());
        }
        vars.sort();
        vars.dedup();
        vars
    }
}

// Data model for CLPFD constraints
#[derive(Debug, Clone)]
pub enum ClpfdConstraint<U: User, E: Engine<U>> {
    Domain {
        variable: String,
        domain_spec: DomainSpec,
        _phantom: std::marker::PhantomData<(U, E)>,
    },
    ListDomain {
        variables: Vec<String>,
        domain_spec: DomainSpec,
        _phantom: std::marker::PhantomData<(U, E)>,
    },
    Expression {
        left: ArithExpr,
        op: CompOp,
        right: ArithExpr,
        _phantom: std::marker::PhantomData<(U, E)>,
    },
    Global {
        name: String,
        args: Vec<String>,
        _phantom: std::marker::PhantomData<(U, E)>,
    },
    Fresh {
        vars: Vec<String>,
        constraints: Vec<ClpfdConstraint<U, E>>,
        _phantom: std::marker::PhantomData<(U, E)>,
    },
}

#[derive(Debug, Clone)]
pub enum DomainSpec {
    Range(i32, i32),
    Set(Vec<i32>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArithExpr {
    Integer(i32),
    Variable(String),
    BinaryOp {
        left: Box<ArithExpr>,
        op: ArithOp,
        right: Box<ArithExpr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArithOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompOp {
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
}

impl<U: User, E: Engine<U>> ClpfdConstraint<U, E> {
    /// Converts a single constraint into a goal.
    /// This will require accessing the execution context to resolve variables.
    fn convert_to_goal(
        &self,
        execution_context: &mut ExecutionContext<U, E>,
    ) -> Result<Goal<U, E>, InterpreterError> {
        match self {
            ClpfdConstraint::Domain {
                variable,
                domain_spec,
                ..
            } => {
                let var_term = execution_context.get_existing_variable(variable)?;
                match domain_spec {
                    DomainSpec::Range(start, end) => {
                        use crate::relation::clpfd::infd::infdrange;
                        let range = *start as isize..=*end as isize;
                        Ok(infdrange(var_term, &range).cast_into())
                    }
                    DomainSpec::Set(values) => {
                        use crate::relation::clpfd::infd::infd;
                        let domain_values: Vec<isize> =
                            values.iter().map(|&v| v as isize).collect();
                        Ok(infd(var_term, &domain_values).cast_into())
                    }
                }
            }
            ClpfdConstraint::ListDomain {
                variables,
                domain_spec,
                ..
            } => {
                // Convert variable names to LTerms
                let var_terms: Result<Vec<_>, _> = variables
                    .iter()
                    .map(|var| execution_context.get_existing_variable(var))
                    .collect();
                let var_terms = var_terms?;

                // Create a list from the variables
                let mut var_list = LTerm::empty_list();
                for var in var_terms.into_iter().rev() {
                    var_list = LTerm::cons(var, var_list);
                }

                // Use infdrange with the list (like the working macro approach)
                match domain_spec {
                    DomainSpec::Range(start, end) => {
                        use crate::relation::clpfd::infd::infdrange;
                        let range = *start as isize..=*end as isize;
                        Ok(infdrange(var_list, &range).cast_into())
                    }
                    DomainSpec::Set(values) => {
                        use crate::relation::clpfd::infd::infd;
                        let domain_values: Vec<isize> =
                            values.iter().map(|&v| v as isize).collect();
                        Ok(infd(var_list, &domain_values).cast_into())
                    }
                }
            }
            ClpfdConstraint::Expression {
                left, op, right, ..
            } => {
                // Check for arithmetic equality patterns that should use specialized CLPFD constraints
                if *op == CompOp::Equal {
                    // Try to detect patterns like: arith_expr == value or value == arith_expr
                    if let Some(goal) =
                        try_build_arithmetic_constraint(left, right, execution_context)?
                    {
                        return Ok(goal);
                    }
                    if let Some(goal) =
                        try_build_arithmetic_constraint(right, left, execution_context)?
                    {
                        return Ok(goal);
                    }
                }

                // Fall back to generic constraint handling
                let left_term = eval_arith_expr(left, execution_context)?;
                let right_term = eval_arith_expr(right, execution_context)?;
                build_comparison_goal(left_term, *op, right_term)
            }
            ClpfdConstraint::Global { name, args, .. } => {
                // Handle both variables and constants in the arguments
                let var_terms: Result<Vec<_>, _> = args
                    .iter()
                    .map(|arg| {
                        // Try to parse as integer first, then as variable
                        if let Ok(int_val) = arg.parse::<i32>() {
                            Ok(LTerm::from(int_val as isize))
                        } else {
                            execution_context.get_existing_variable(arg)
                        }
                    })
                    .collect();
                let var_terms = var_terms?;

                match name.as_str() {
                    "distinct" => {
                        use crate::relation::clpfd::distinctfd::distinctfd;
                        // Convert Vec<LTerm> to LTerm (list) in correct order
                        let mut list_term = LTerm::empty_list();
                        for var in var_terms.into_iter().rev() {
                            list_term = LTerm::cons(var, list_term);
                        }
                        Ok(distinctfd(list_term).cast_into())
                    }
                    "alldiff" => {
                        use crate::relation::clpfd::distinctfd::distinctfd;
                        // Convert Vec<LTerm> to LTerm (list) in correct order
                        let mut list_term = LTerm::empty_list();
                        for var in var_terms.into_iter().rev() {
                            list_term = LTerm::cons(var, list_term);
                        }
                        Ok(distinctfd(list_term).cast_into())
                    }
                    _ => Err(InterpreterError::UnknownGlobalConstraint(name.clone())),
                }
            }
            ClpfdConstraint::Fresh {
                vars, constraints, ..
            } => {
                // Create a new scope for the fresh variables
                execution_context.push_scope();
                for var in vars {
                    execution_context.get_or_create_variable(var)?;
                }

                let mut goals = vec![];
                for constraint in constraints {
                    goals.push(constraint.convert_to_goal(execution_context)?);
                }

                // Pop the scope after building the goals
                execution_context.pop_scope();

                if goals.is_empty() {
                    Ok(Goal::succeed())
                } else {
                    let mut iter = goals.into_iter();
                    let first = iter.next().unwrap();
                    Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
                }
            }
        }
    }

    /// Extracts all unique variable names used in the constraint.
    fn extract_variables(&self) -> Vec<String> {
        let mut vars = Vec::new();
        match self {
            ClpfdConstraint::Domain { variable, .. } => {
                vars.push(variable.clone());
            }
            ClpfdConstraint::ListDomain { variables, .. } => {
                vars.extend(variables.clone());
            }
            ClpfdConstraint::Expression { left, right, .. } => {
                fn collect_vars(expr: &ArithExpr, vars: &mut Vec<String>) {
                    match expr {
                        ArithExpr::Variable(name) => vars.push(name.clone()),
                        ArithExpr::Integer(_) => {}
                        ArithExpr::BinaryOp { left, right, .. } => {
                            collect_vars(left, vars);
                            collect_vars(right, vars);
                        }
                    }
                }
                collect_vars(left, &mut vars);
                collect_vars(right, &mut vars);
            }
            ClpfdConstraint::Global { args, .. } => {
                vars.extend(args.clone());
            }
            ClpfdConstraint::Fresh {
                vars: _,
                constraints,
                ..
            } => {
                for c in constraints {
                    vars.extend(c.extract_variables());
                }
            }
        }
        vars
    }
}

/// Evaluates an arithmetic expression, creating temporary variables for intermediate results.
fn eval_arith_expr<U: User, E: Engine<U>>(
    expr: &ArithExpr,
    execution_context: &mut ExecutionContext<U, E>,
) -> Result<LTerm<U, E>, InterpreterError> {
    match expr {
        ArithExpr::Integer(val) => Ok(LTerm::from(*val as isize)),
        ArithExpr::Variable(name) => execution_context.get_existing_variable(name),
        ArithExpr::BinaryOp { left, op, right } => {
            let left_term = eval_arith_expr(left, execution_context)?;
            let right_term = eval_arith_expr(right, execution_context)?;
            let result_term = execution_context.create_fresh_var();

            let goal = match op {
                ArithOp::Add => {
                    use crate::relation::clpfd::plusfd::plusfd;
                    plusfd(left_term, right_term, result_term.clone()).cast_into()
                }
                ArithOp::Subtract => {
                    use crate::relation::clpfd::minusfd::minusfd;
                    minusfd(left_term, right_term, result_term.clone()).cast_into()
                }
                ArithOp::Multiply => {
                    use crate::relation::clpfd::timesfd::timesfd;
                    timesfd(left_term, right_term, result_term.clone()).cast_into()
                }
                ArithOp::Divide => {
                    // Note: CLP(FD) division is often not a primitive.
                    // This will likely need a more complex implementation or might not be fully supported.
                    // For now, we create a multiplication goal: result * right == left
                    use crate::relation::clpfd::timesfd::timesfd;
                    timesfd(result_term.clone(), right_term, left_term).cast_into()
                }
            };

            execution_context.add_deferred_goal(goal);
            Ok(result_term)
        }
    }
}

fn build_comparison_goal<U: User, E: Engine<U>>(
    left: LTerm<U, E>,
    op: CompOp,
    right: LTerm<U, E>,
) -> Result<Goal<U, E>, InterpreterError> {
    match op {
        CompOp::Equal => {
            // Check if this is an arithmetic equality that should use specialized CLPFD constraints
            // Pattern: arith_expr == value  or  value == arith_expr
            // We need to detect patterns like x + y == z, x * y == z, etc.

            // For now, use generic equality - we'll need to enhance this to detect arithmetic patterns
            // TODO: Detect arithmetic expressions and convert to plusfd, timesfd, minusfd
            use crate::relation::eq::eq;
            Ok(eq(left, right).cast_into())
        }
        CompOp::NotEqual => {
            use crate::relation::clpfd::diseqfd::diseqfd;
            Ok(diseqfd(left, right).cast_into())
        }
        CompOp::LessThan => {
            use crate::relation::clpfd::ltfd::ltfd;
            Ok(ltfd(left, right).cast_into())
        }
        CompOp::LessEqual => {
            use crate::relation::clpfd::ltefd::ltefd;
            Ok(ltefd(left, right).cast_into())
        }
        CompOp::GreaterThan => {
            use crate::relation::clpfd::ltfd::ltfd;
            Ok(ltfd(right, left).cast_into())
        }
        CompOp::GreaterEqual => {
            use crate::relation::clpfd::ltefd::ltefd;
            Ok(ltefd(right, left).cast_into())
        }
    }
}

fn try_build_arithmetic_constraint<U: User, E: Engine<U>>(
    left: &ArithExpr,
    right: &ArithExpr,
    execution_context: &mut ExecutionContext<U, E>,
) -> Result<Option<Goal<U, E>>, InterpreterError> {
    // Detect patterns like: x * y == 6, x + y == z, etc.
    // Left side should be arithmetic expression, right side should be simple value or variable

    match (left, right) {
        // Pattern: x * y == value
        (
            ArithExpr::BinaryOp {
                left: x,
                op: ArithOp::Multiply,
                right: y,
            },
            ArithExpr::Integer(value),
        ) => {
            use crate::relation::clpfd::timesfd::timesfd;
            let x_term = eval_arith_expr(x, execution_context)?;
            let y_term = eval_arith_expr(y, execution_context)?;
            let value_term = LTerm::from(*value as isize);
            Ok(Some(timesfd(x_term, y_term, value_term).cast_into()))
        }
        // Pattern: x * y == variable
        (
            ArithExpr::BinaryOp {
                left: x,
                op: ArithOp::Multiply,
                right: y,
            },
            ArithExpr::Variable(_),
        ) => {
            use crate::relation::clpfd::timesfd::timesfd;
            let x_term = eval_arith_expr(x, execution_context)?;
            let y_term = eval_arith_expr(y, execution_context)?;
            let z_term = eval_arith_expr(right, execution_context)?;
            Ok(Some(timesfd(x_term, y_term, z_term).cast_into()))
        }
        // Pattern: x + y == value
        (
            ArithExpr::BinaryOp {
                left: x,
                op: ArithOp::Add,
                right: y,
            },
            ArithExpr::Integer(value),
        ) => {
            use crate::relation::clpfd::plusfd::plusfd;
            let x_term = eval_arith_expr(x, execution_context)?;
            let y_term = eval_arith_expr(y, execution_context)?;
            let value_term = LTerm::from(*value as isize);
            Ok(Some(plusfd(x_term, y_term, value_term).cast_into()))
        }
        // Pattern: x + y == variable
        (
            ArithExpr::BinaryOp {
                left: x,
                op: ArithOp::Add,
                right: y,
            },
            ArithExpr::Variable(_),
        ) => {
            use crate::relation::clpfd::plusfd::plusfd;
            let x_term = eval_arith_expr(x, execution_context)?;
            let y_term = eval_arith_expr(y, execution_context)?;
            let z_term = eval_arith_expr(right, execution_context)?;
            Ok(Some(plusfd(x_term, y_term, z_term).cast_into()))
        }
        // Pattern: x - y == value
        (
            ArithExpr::BinaryOp {
                left: x,
                op: ArithOp::Subtract,
                right: y,
            },
            ArithExpr::Integer(value),
        ) => {
            use crate::relation::clpfd::minusfd::minusfd;
            let x_term = eval_arith_expr(x, execution_context)?;
            let y_term = eval_arith_expr(y, execution_context)?;
            let value_term = LTerm::from(*value as isize);
            Ok(Some(minusfd(x_term, y_term, value_term).cast_into()))
        }
        // Pattern: x - y == variable
        (
            ArithExpr::BinaryOp {
                left: x,
                op: ArithOp::Subtract,
                right: y,
            },
            ArithExpr::Variable(_),
        ) => {
            use crate::relation::clpfd::minusfd::minusfd;
            let x_term = eval_arith_expr(x, execution_context)?;
            let y_term = eval_arith_expr(y, execution_context)?;
            let z_term = eval_arith_expr(right, execution_context)?;
            Ok(Some(minusfd(x_term, y_term, z_term).cast_into()))
        }
        _ => Ok(None),
    }
}
