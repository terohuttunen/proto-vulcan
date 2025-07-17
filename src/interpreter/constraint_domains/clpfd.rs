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
use crate::interpreter::parser::meta_parser;
use crate::interpreter::InterpreterError;
use crate::lterm::LTerm;
use crate::operator::conj::Conj;
use crate::user::User;
use pest::Parser;
use pest_derive::Parser;
use std::rc::Rc;

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
        source_span: &super::super::parser::ast::Span,
    ) -> Result<Box<dyn DomainConstraints<U, E>>, InterpreterError> {
        // Trim the content to remove leading/trailing whitespace that might interfere
        let trimmed_content = body.raw_content.trim();

        let pairs = ClpfdParser::parse(Rule::constraints, trimmed_content).map_err(|e| {
            InterpreterError::InvalidConstraintSyntax {
                domain: "clpfd".to_string(),
                error: super::map_constraint_error_position(&e, body),
            }
        })?;

        let mut constraints = Vec::new();

        // Process all pairs to extract constraints
        for pair in pairs {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::constraint {
                    let constraint = Self::build_constraint(inner, source_span)?;
                    constraints.push(constraint);
                }
            }
        }

        Ok(Box::new(ClpfdConstraints {
            constraints,
            source_span: source_span.clone(),
        }))
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
            let dummy_span = super::super::parser::ast::Span::dummy();
            Self::build_constraint(pair, &dummy_span)
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
        source_span: &super::super::parser::ast::Span,
    ) -> Result<ClpfdConstraint<U, E>, InterpreterError> {
        let inner_pair = pair.into_inner().next().unwrap();
        match inner_pair.as_rule() {
            Rule::fresh_constraint => Self::build_fresh_constraint(inner_pair, source_span),
            Rule::domain_constraint => Self::build_domain_constraint(inner_pair, source_span),
            Rule::list_domain_constraint => {
                Self::build_list_domain_constraint(inner_pair, source_span)
            }
            Rule::distinct_constraint => Self::build_distinct_constraint(inner_pair),
            Rule::alldiff_constraint => Self::build_alldiff_constraint(inner_pair),
            Rule::arith_constraint => Self::build_arith_constraint(inner_pair, source_span),
            _ => unreachable!("Unexpected rule in constraint"),
        }
    }

    fn build_fresh_constraint<U: User, E: Engine<U>>(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Span,
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
                            constraints.push(Self::build_constraint(constraint_pair, source_span)?);
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
        source_span: &super::super::parser::ast::Span,
    ) -> Result<ClpfdConstraint<U, E>, InterpreterError> {
        let mut inner = pair.into_inner();
        let var_pair = inner.next().unwrap();
        let variable = match var_pair.as_rule() {
            Rule::variable => var_pair.as_str().to_string(),
            Rule::interpolation_expression => {
                // Store interpolated variables with special format for later evaluation
                let content = var_pair.into_inner().next().unwrap().as_str();
                format!("{{{}}}", content) // Store with braces to indicate it's an interpolation
            }
            _ => var_pair.as_str().to_string(),
        };
        let _in_kw = inner.next().unwrap(); // Skip the "in" keyword
        let range_spec_pair = inner.next().unwrap();
        let domain_spec = Self::build_domain_spec(range_spec_pair, source_span)?;

        Ok(ClpfdConstraint::Domain {
            variable,
            domain_spec,
            _phantom: std::marker::PhantomData,
        })
    }

    fn build_list_domain_constraint<U: User, E: Engine<U>>(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Span,
    ) -> Result<ClpfdConstraint<U, E>, InterpreterError> {
        let mut inner = pair.into_inner();
        let var_list_pair = inner.next().unwrap(); // var_list
        let variables = var_list_pair
            .into_inner()
            .map(|p| p.as_str().to_string())
            .collect();
        let _in_kw = inner.next().unwrap(); // Skip the "in" keyword
        let range_spec_pair = inner.next().unwrap();
        let domain_spec = Self::build_domain_spec(range_spec_pair, source_span)?;

        Ok(ClpfdConstraint::ListDomain {
            variables,
            domain_spec,
            _phantom: std::marker::PhantomData,
        })
    }

    fn build_domain_spec(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Span,
    ) -> Result<DomainSpec, InterpreterError> {
        let inner_pair = pair.into_inner().next().unwrap();
        match inner_pair.as_rule() {
            Rule::range_dotdot => {
                let mut inner = inner_pair.into_inner();
                let start_pair = inner.next().unwrap();
                let end_pair = inner.next().unwrap();

                let start_bound = Self::build_domain_bound(start_pair, source_span)?;
                let end_bound = Self::build_domain_bound(end_pair, source_span)?;

                // Check if both bounds are simple integers
                if let (DomainBound::Integer(start), DomainBound::Integer(end)) =
                    (&start_bound, &end_bound)
                {
                    Ok(DomainSpec::Range(*start, *end))
                } else {
                    Ok(DomainSpec::InterpolatedRange(start_bound, end_bound))
                }
            }
            Rule::range_set => {
                let mut bounds = Vec::new();
                let mut all_integers = true;

                for p in inner_pair.into_inner() {
                    let bound = Self::build_domain_bound(p, source_span)?;
                    if !matches!(bound, DomainBound::Integer(_)) {
                        all_integers = false;
                    }
                    bounds.push(bound);
                }

                if all_integers {
                    let values: Vec<i32> = bounds
                        .into_iter()
                        .map(|b| {
                            if let DomainBound::Integer(val) = b {
                                val
                            } else {
                                unreachable!()
                            }
                        })
                        .collect();
                    Ok(DomainSpec::Set(values))
                } else {
                    Ok(DomainSpec::InterpolatedSet(bounds))
                }
            }
            Rule::range_list => {
                // Handle [1, 2, {foo}] syntax - same as range_set but with square brackets
                let mut bounds = Vec::new();
                let mut all_integers = true;

                for element_pair in inner_pair.into_inner() {
                    let bound = Self::build_domain_bound(element_pair, source_span)?;
                    if !matches!(bound, DomainBound::Integer(_)) {
                        all_integers = false;
                    }
                    bounds.push(bound);
                }

                if all_integers {
                    let values: Vec<i32> = bounds
                        .into_iter()
                        .map(|b| {
                            if let DomainBound::Integer(val) = b {
                                val
                            } else {
                                unreachable!()
                            }
                        })
                        .collect();
                    Ok(DomainSpec::Set(values))
                } else {
                    Ok(DomainSpec::InterpolatedSet(bounds))
                }
            }
            Rule::range_single => {
                let bound =
                    Self::build_domain_bound(inner_pair.into_inner().next().unwrap(), source_span)?;

                if let DomainBound::Integer(value) = bound {
                    Ok(DomainSpec::Set(vec![value]))
                } else {
                    Ok(DomainSpec::InterpolatedSet(vec![bound]))
                }
            }
            _ => unreachable!("Unexpected rule in range_spec"),
        }
    }

    fn build_domain_bound(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Span,
    ) -> Result<DomainBound, InterpreterError> {
        match pair.as_rule() {
            Rule::integer => {
                let value = pair.as_str().parse().map_err(|_| {
                    InterpreterError::InvalidConstraintSyntax {
                        domain: "clpfd".to_string(),
                        error: format!("Invalid integer: {}", pair.as_str()),
                    }
                })?;
                Ok(DomainBound::Integer(value))
            }
            Rule::interpolation_expression => {
                let content = pair.into_inner().next().unwrap().as_str();
                let meta_expr =
                    meta_parser::parse_meta_expression(content, source_span).map_err(|_| {
                        InterpreterError::InvalidConstraintSyntax {
                            domain: "clpfd".to_string(),
                            error: format!("Invalid meta expression: {}", content),
                        }
                    })?;
                Ok(DomainBound::Interpolation(meta_expr))
            }
            _ => unreachable!("Unexpected rule in domain bound: {:?}", pair.as_rule()),
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
        source_span: &super::super::parser::ast::Span,
    ) -> Result<ClpfdConstraint<U, E>, InterpreterError> {
        let mut inner = pair.into_inner();
        let left = Self::build_arith_expr(inner.next().unwrap(), source_span);
        let op = Self::build_comp_op(inner.next().unwrap());
        let right = Self::build_arith_expr(inner.next().unwrap(), source_span);

        Ok(ClpfdConstraint::Expression {
            left,
            op,
            right,
            _phantom: std::marker::PhantomData,
        })
    }

    // This function uses the Pratt parser technique to handle operator precedence.
    fn build_arith_expr(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Span,
    ) -> ArithExpr {
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
                Rule::interpolation_expression => {
                    let content = primary.into_inner().next().unwrap().as_str();
                    let meta_expr = meta_parser::parse_meta_expression(content, source_span)
                        .unwrap_or_else(|_| {
                            // Fallback to a variable if parsing fails
                            crate::interpreter::metaprogramming::MetaExpression::Variable(
                                content.to_string(),
                                super::super::parser::ast::Span::dummy(),
                            )
                        });
                    ArithExpr::Interpolation(meta_expr)
                }
                Rule::arith_expr => Self::build_arith_expr(primary, source_span), // for parentheses
                Rule::factor => {
                    // Handle factor rule by extracting its inner content
                    let inner = primary.into_inner().next().unwrap();
                    match inner.as_rule() {
                        Rule::integer => ArithExpr::Integer(inner.as_str().parse().unwrap()),
                        Rule::variable => ArithExpr::Variable(inner.as_str().to_string()),
                        Rule::interpolation_expression => {
                            let content = inner.into_inner().next().unwrap().as_str();
                            let meta_expr = meta_parser::parse_meta_expression(
                                content,
                                source_span,
                            )
                            .unwrap_or_else(|_| {
                                crate::interpreter::metaprogramming::MetaExpression::Variable(
                                    content.to_string(),
                                    super::super::parser::ast::Span::dummy(),
                                )
                            });
                            ArithExpr::Interpolation(meta_expr)
                        }
                        Rule::arith_expr => Self::build_arith_expr(inner, source_span),
                        _ => unreachable!("Unexpected factor inner rule: {:?}", inner.as_rule()),
                    }
                }
                Rule::term => {
                    // Handle term = factor ~ (mul_op ~ factor)*
                    let mut inner = primary.into_inner();
                    let mut left = Self::build_arith_expr(inner.next().unwrap(), source_span);

                    // Process any multiplication operations
                    while let Some(op_pair) = inner.next() {
                        if op_pair.as_rule() == Rule::mul_op {
                            let op = match op_pair.as_str() {
                                "*" => ArithOp::Multiply,
                                "/" => ArithOp::Divide,
                                _ => unreachable!(),
                            };
                            let right = Self::build_arith_expr(inner.next().unwrap(), source_span);
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
    source_span: super::super::parser::ast::Span,
}

impl<U: User, E: Engine<U>> DomainConstraints<U, E> for ClpfdConstraints<U, E> {
    fn convert_to_goals(
        &self,
        execution_context: &mut ExecutionContext<U, E>,
    ) -> Result<Goal<U, E>, InterpreterError> {
        let goals: Result<Vec<_>, _> = self
            .constraints
            .iter()
            .map(|c| c.convert_to_goal(execution_context, &self.source_span))
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
    InterpolatedRange(DomainBound, DomainBound),
    InterpolatedSet(Vec<DomainBound>),
}

#[derive(Debug, Clone)]
pub enum DomainBound {
    Integer(i32),
    Interpolation(crate::interpreter::metaprogramming::MetaExpression),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArithExpr {
    Integer(i32),
    Variable(String),
    Interpolation(crate::interpreter::metaprogramming::MetaExpression),
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
        source_span: &super::super::parser::ast::Span,
    ) -> Result<Goal<U, E>, InterpreterError> {
        match self {
            ClpfdConstraint::Domain {
                variable,
                domain_spec,
                ..
            } => {
                let var_term = if variable.starts_with('{') && variable.ends_with('}') {
                    // This is an interpolated variable stored as "{content}"
                    let content = &variable[1..variable.len() - 1]; // Remove braces
                    let meta_expr = meta_parser::parse_meta_expression(content, source_span)
                        .map_err(|_| {
                            InterpreterError::RuntimeError(format!(
                                "Invalid meta expression in domain variable: {}",
                                content
                            ))
                        })?;

                    // Evaluate the meta expression to get the variable term
                    use crate::interpreter::metaprogramming::{
                        expand_term, TemplateExpansionContext,
                    };
                    use crate::interpreter::parser::ast::Term;

                    let context = TemplateExpansionContext::new(100);
                    let dummy_term = Term::Interpolation(meta_expr, source_span.clone());
                    let expanded_term = expand_term(&dummy_term, &context).map_err(|e| {
                        InterpreterError::RuntimeError(format!(
                            "Meta expression expansion error in domain variable: {}",
                            e
                        ))
                    })?;

                    execution_context.ast_term_to_runtime(&expanded_term)?
                } else {
                    execution_context.get_existing_variable(variable)?
                };
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
                    DomainSpec::InterpolatedRange(start, end) => {
                        let start_val = eval_domain_bound(start, execution_context, source_span)?;
                        let end_val = eval_domain_bound(end, execution_context, source_span)?;
                        use crate::relation::clpfd::infd::infdrange;
                        let range = start_val..=end_val;
                        Ok(infdrange(var_term, &range).cast_into())
                    }
                    DomainSpec::InterpolatedSet(values) => {
                        let domain_values: Result<Vec<isize>, InterpreterError> = values
                            .iter()
                            .map(|bound| eval_domain_bound(bound, execution_context, source_span))
                            .collect();
                        use crate::relation::clpfd::infd::infd;
                        Ok(infd(var_term, &domain_values?).cast_into())
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
                    DomainSpec::InterpolatedRange(start, end) => {
                        let start_val = eval_domain_bound(start, execution_context, source_span)?;
                        let end_val = eval_domain_bound(end, execution_context, source_span)?;
                        use crate::relation::clpfd::infd::infdrange;
                        let range = start_val..=end_val;
                        Ok(infdrange(var_list, &range).cast_into())
                    }
                    DomainSpec::InterpolatedSet(values) => {
                        let domain_values: Result<Vec<isize>, InterpreterError> = values
                            .iter()
                            .map(|bound| eval_domain_bound(bound, execution_context, source_span))
                            .collect();
                        use crate::relation::clpfd::infd::infd;
                        Ok(infd(var_list, &domain_values?).cast_into())
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
                    let fresh_var = execution_context.create_fresh_var();
                    execution_context.bind_var(var.clone(), fresh_var);
                }

                let mut goals = vec![];
                for constraint in constraints {
                    goals.push(constraint.convert_to_goal(execution_context, source_span)?);
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
                        ArithExpr::Interpolation(_) => {
                            // Interpolation expressions don't contribute to static variable extraction
                            // as they're evaluated at runtime
                        }
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
        ArithExpr::Interpolation(meta_expr) => {
            // For simple variable interpolations, directly access the execution context
            match meta_expr {
                crate::interpreter::metaprogramming::MetaExpression::Variable(var_name, _) => {
                    // Directly look up the variable in the execution context
                    execution_context
                        .get_existing_variable(var_name)
                        .map_err(|_| {
                            InterpreterError::RuntimeError(format!(
                                "Interpolation variable '{}' not found in constraint context",
                                var_name
                            ))
                        })
                }
                _ => {
                    // For complex expressions, use template expansion with execution context bindings
                    use crate::interpreter::metaprogramming::{
                        expand_term, MetaBindings, MetaValue, TemplateExpansionContext,
                    };
                    use crate::interpreter::parser::ast::Term;

                    // Create template context with current variable bindings from execution context
                    let mut bindings = MetaBindings::new();

                    // Get all variable bindings from execution context and convert them to meta values
                    let var_bindings = execution_context.get_variable_bindings();
                    for (var_name, var_term) in var_bindings {
                        if let Some(number) = var_term.get_number() {
                            bindings.insert(var_name, MetaValue::Integer(number as i64));
                        }
                        // Could add support for other types here in the future
                    }

                    let context = TemplateExpansionContext::with_bindings(bindings, 100);

                    // Create dummy term and expand it
                    let dummy_term = Term::Interpolation(meta_expr.clone(), Default::default());
                    let expanded_term = expand_term(&dummy_term, &context).map_err(|e| {
                        InterpreterError::RuntimeError(format!(
                            "Meta expression expansion error in arithmetic: {}",
                            e
                        ))
                    })?;

                    // Convert to runtime term
                    execution_context.ast_term_to_runtime(&expanded_term)
                }
            }
        }
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

fn eval_domain_bound<U: User, E: Engine<U>>(
    bound: &DomainBound,
    execution_context: &mut ExecutionContext<U, E>,
    source_span: &super::super::parser::ast::Span,
) -> Result<isize, InterpreterError> {
    match bound {
        DomainBound::Integer(val) => Ok(*val as isize),
        DomainBound::Interpolation(meta_expr) => {
            // For simple variable interpolations, directly access the execution context
            match meta_expr {
                crate::interpreter::metaprogramming::MetaExpression::Variable(var_name, _) => {
                    // Directly look up the variable in the execution context
                    let var_term =
                        execution_context
                            .get_existing_variable(var_name)
                            .map_err(|_| {
                                InterpreterError::RuntimeError(format!(
                                    "Interpolation variable '{}' not found in constraint context",
                                    var_name
                                ))
                            })?;

                    // Extract integer value using get_number()
                    var_term.get_number().ok_or_else(|| {
                        InterpreterError::RuntimeError(format!(
                            "Interpolation variable '{}' must evaluate to an integer",
                            var_name
                        ))
                    })
                }
                _ => {
                    // For complex expressions, use template expansion with execution context bindings
                    use crate::interpreter::metaprogramming::{
                        expand_term, MetaBindings, MetaValue, TemplateExpansionContext,
                    };
                    use crate::interpreter::parser::ast::Term;

                    // Create template context with current variable bindings from execution context
                    let mut bindings = MetaBindings::new();

                    // Get all variable bindings from execution context and convert them to meta values
                    let var_bindings = execution_context.get_variable_bindings();
                    for (var_name, var_term) in var_bindings {
                        if let Some(number) = var_term.get_number() {
                            bindings.insert(var_name, MetaValue::Integer(number as i64));
                        }
                        // Could add support for other types here in the future
                    }

                    let context = TemplateExpansionContext::with_bindings(bindings, 100);

                    // Create dummy term and expand it
                    let dummy_term = Term::Interpolation(meta_expr.clone(), Default::default());
                    let expanded_term = expand_term(&dummy_term, &context).map_err(|e| {
                        InterpreterError::RuntimeError(format!(
                            "Meta expression expansion error: {}",
                            e
                        ))
                    })?;

                    // Convert to runtime and extract integer
                    let runtime_term = execution_context.ast_term_to_runtime(&expanded_term)?;

                    // Extract integer value using get_number()
                    runtime_term.get_number().ok_or_else(|| {
                        InterpreterError::RuntimeError(
                            "Domain bound meta expression must evaluate to an integer".to_string(),
                        )
                    })
                }
            }
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
