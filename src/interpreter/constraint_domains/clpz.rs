//! CLPZ (Constraint Logic Programming over Integers) Implementation
//!
//! This domain provides integer constraint syntax like:
//! - x + y == z (arithmetic constraints)
//! - x * y == z (multiplication constraints)
//! - x < y (comparison constraints)

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
#[grammar = "interpreter/constraint_domains/grammars/clpz.pest"]
pub struct ClpzParser;

/// CLPZ constraint template that stores compiled constraint information
#[derive(Debug, Clone)]
pub struct ClpzTemplate {
    body: ConstraintBody,
    variables: std::collections::HashMap<String, super::VariableInfo>,
}

impl super::DomainConstraintTemplate for ClpzTemplate {
    fn execute(&self, execution_context: &mut super::super::execution::ExecutionContext<crate::user::DefaultUser, crate::engine::DefaultEngine<crate::user::DefaultUser>>) 
        -> Result<crate::goal::Goal<crate::user::DefaultUser, crate::engine::DefaultEngine<crate::user::DefaultUser>>, InterpreterError> {
        // For now, fall back to the original parsing approach
        // TODO: Implement proper template-based execution
        let domain = ClpzDomain::new();
        let parsed_constraints = domain.parse_constraints(&self.body, &super::super::parser::ast::Span::dummy())?;
        parsed_constraints.convert_to_goals(execution_context)
    }
}

/// CLPZ IR constraint template that stores compiled constraint information for IR system
#[derive(Debug, Clone)]
pub struct ClpzIrTemplate {
    body: ConstraintBody,
    variables: std::collections::HashMap<String, super::VariableInfo>,
}

impl super::IrDomainConstraintTemplate for ClpzIrTemplate {
    fn execute(&self, execution_context: &mut super::super::ir::context::ExecutionContext) 
        -> Result<crate::goal::Goal<crate::user::DefaultUser, crate::engine::DefaultEngine<crate::user::DefaultUser>>, InterpreterError> {
        // Create a temporary bridge between IR ExecutionContext and regular ExecutionContext
        // This is a temporary implementation until we have full IR-native constraint compilation
        use super::super::execution::ExecutionContext as RegularExecutionContext;
        use crate::user::DefaultUser;
        use crate::engine::DefaultEngine;
        
        // Create a temporary execution context for constraint compilation
        let mut temp_execution_context = RegularExecutionContext::<DefaultUser, DefaultEngine<DefaultUser>>::new(
            execution_context.environment().clone()
        );
        
        // For now, fall back to the original parsing approach with temporary context
        let domain = ClpzDomain::new();
        let parsed_constraints = domain.parse_constraints(&self.body, &super::super::parser::ast::Span::dummy())?;
        parsed_constraints.convert_to_goals(&mut temp_execution_context)
    }
}

/// CLPZ constraint domain
pub struct ClpzDomain;

impl ClpzDomain {
    pub fn new() -> Self {
        Self
    }
}

impl<U: User, E: Engine<U>> ConstraintDomain<U, E> for ClpzDomain {
    fn name(&self) -> &str {
        "clpz"
    }

    fn parse_constraints(
        &self,
        body: &ConstraintBody,
        source_span: &super::super::parser::ast::Span,
    ) -> Result<Box<dyn DomainConstraints<U, E>>, InterpreterError> {
        let trimmed_content = body.raw_content.trim();

        let pairs = ClpzParser::parse(Rule::constraints, trimmed_content).map_err(|e| {
            InterpreterError::InvalidConstraintSyntax {
                domain: "clpz".to_string(),
                error: super::map_constraint_error_position(&e, body),
            }
        })?;

        let mut constraints = Vec::new();

        for pair in pairs {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::constraint {
                    let constraint = Self::build_constraint(inner, source_span)?;
                    constraints.push(constraint);
                }
            }
        }

        Ok(Box::new(ClpzConstraints {
            constraints,
            source_span: source_span.clone(),
        }))
    }

    fn syntax_help(&self) -> &str {
        r#"CLPZ Syntax:
- Arithmetic: x + y == z, x - y == z, x * y == z
- Comparison: x < y, x <= y, x > y, x >= y, x != y, x == y
- Fresh: |x, y| { x + y == z }"#
    }

    fn get_unbound_variables(&self, body: &ConstraintBody) -> Result<Vec<String>, InterpreterError> {
        // For now, parse the constraint to extract variables using concrete types
        // TODO: Implement proper variable extraction without full parsing
        use crate::user::DefaultUser;
        use crate::engine::DefaultEngine;
        let parsed_constraints: Box<dyn DomainConstraints<DefaultUser, DefaultEngine<DefaultUser>>> = 
            self.parse_constraints(body, &super::super::parser::ast::Span::dummy())?;
        Ok(parsed_constraints.extract_variables())
    }

    fn compile_template(
        &self,
        body: &ConstraintBody,
        variables: std::collections::HashMap<String, super::VariableInfo>,
    ) -> Result<std::rc::Rc<dyn super::DomainConstraintTemplate>, InterpreterError> {
        // Create a CLPZ template with the constraint body and variable information
        let template = ClpzTemplate {
            body: body.clone(),
            variables,
        };
        Ok(std::rc::Rc::new(template))
    }

    fn compile_ir_template(
        &self,
        body: &ConstraintBody,
        variables: std::collections::HashMap<String, super::VariableInfo>,
    ) -> Result<std::rc::Rc<dyn super::IrDomainConstraintTemplate>, InterpreterError> {
        // Create a CLPZ IR template with the constraint body and variable information
        let template = ClpzIrTemplate {
            body: body.clone(),
            variables,
        };
        Ok(std::rc::Rc::new(template))
    }
}

impl ClpzDomain {
    fn build_constraint<U: User, E: Engine<U>>(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Span,
    ) -> Result<ClpzConstraint<U, E>, InterpreterError> {
        let constraint_pair = pair.into_inner().next().unwrap();

        match constraint_pair.as_rule() {
            Rule::fresh_constraint => Self::build_fresh_constraint(constraint_pair, source_span),
            Rule::arith_constraint => Self::build_arith_constraint(constraint_pair, source_span),
            _ => unreachable!(
                "Unexpected rule in constraint_expr: {:?}",
                constraint_pair.as_rule()
            ),
        }
    }

    fn build_fresh_constraint<U: User, E: Engine<U>>(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Span,
    ) -> Result<ClpzConstraint<U, E>, InterpreterError> {
        let mut inner = pair.into_inner();
        let mut vars = vec![];
        let mut constraints = vec![];

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

        Ok(ClpzConstraint::Fresh {
            vars,
            constraints,
            _phantom: std::marker::PhantomData,
        })
    }

    fn build_arith_constraint<U: User, E: Engine<U>>(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Span,
    ) -> Result<ClpzConstraint<U, E>, InterpreterError> {
        let mut inner = pair.into_inner();
        let left = Self::build_arith_expr(inner.next().unwrap(), source_span);
        let op = Self::build_comp_op(inner.next().unwrap());
        let right = Self::build_arith_expr(inner.next().unwrap(), source_span);

        Ok(ClpzConstraint::Expression {
            left,
            op,
            right,
            _phantom: std::marker::PhantomData,
        })
    }

    fn build_arith_expr(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Span,
    ) -> ArithExpr {
        match pair.as_rule() {
            Rule::integer => ArithExpr::Integer(pair.as_str().parse().unwrap()),
            Rule::variable => ArithExpr::Variable(pair.as_str().to_string()),
            Rule::interpolation_expression => {
                let content = pair.into_inner().next().unwrap().as_str();
                let meta_expr =
                    super::super::parser::meta_parser::parse_meta_expression(content, source_span)
                        .unwrap_or_else(|_| {
                            crate::interpreter::metaprogramming::MetaExpression::Variable(
                                content.to_string(),
                                super::super::parser::ast::Span::dummy(),
                            )
                        });

                match &meta_expr {
                    crate::interpreter::metaprogramming::MetaExpression::Variable(var_name, _) => {
                        ArithExpr::Variable(var_name.clone())
                    }
                    _ => ArithExpr::Interpolation(meta_expr),
                }
            }
            Rule::factor => {
                // A factor is either an integer, variable, interpolation expression, or parenthesized expression
                let inner = pair.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::integer => ArithExpr::Integer(inner.as_str().parse().unwrap()),
                    Rule::variable => ArithExpr::Variable(inner.as_str().to_string()),
                    Rule::interpolation_expression => {
                        let content = inner.into_inner().next().unwrap().as_str();
                        let meta_expr = super::super::parser::meta_parser::parse_meta_expression(
                            content,
                            source_span,
                        )
                        .unwrap_or_else(|_| {
                            crate::interpreter::metaprogramming::MetaExpression::Variable(
                                content.to_string(),
                                super::super::parser::ast::Span::dummy(),
                            )
                        });
                        match &meta_expr {
                            crate::interpreter::metaprogramming::MetaExpression::Variable(
                                var_name,
                                _,
                            ) => ArithExpr::Variable(var_name.clone()),
                            _ => ArithExpr::Interpolation(meta_expr),
                        }
                    }
                    Rule::arith_expr => Self::build_arith_expr(inner, source_span),
                    _ => unreachable!("Unexpected factor inner rule: {:?}", inner.as_rule()),
                }
            }
            Rule::term => {
                // A term is one or more factors separated by * or /
                let mut inner = pair.into_inner();
                let mut result = Self::build_arith_expr(inner.next().unwrap(), source_span);

                while let Some(op_pair) = inner.next() {
                    let op = match op_pair.as_str() {
                        "*" => ArithOp::Multiply,
                        "/" => ArithOp::Divide,
                        _ => continue, // Skip non-operator rules
                    };
                    if let Some(right_pair) = inner.next() {
                        let right = Self::build_arith_expr(right_pair, source_span);
                        result = ArithExpr::BinaryOp {
                            left: Box::new(result),
                            op,
                            right: Box::new(right),
                        };
                    }
                }
                result
            }
            Rule::arith_expr => {
                // An arith_expr is one or more terms separated by + or -
                let mut inner = pair.into_inner();
                let mut result = Self::build_arith_expr(inner.next().unwrap(), source_span);

                while let Some(op_pair) = inner.next() {
                    let op = match op_pair.as_str() {
                        "+" => ArithOp::Add,
                        "-" => ArithOp::Subtract,
                        _ => continue, // Skip non-operator rules
                    };
                    if let Some(right_pair) = inner.next() {
                        let right = Self::build_arith_expr(right_pair, source_span);
                        result = ArithExpr::BinaryOp {
                            left: Box::new(result),
                            op,
                            right: Box::new(right),
                        };
                    }
                }
                result
            }
            _ => unreachable!("Unexpected rule in arith_expr: {:?}", pair.as_rule()),
        }
    }

    fn build_comp_op(pair: pest::iterators::Pair<Rule>) -> CompOp {
        let op_str = pair.as_str();
        match op_str {
            "==" => CompOp::Equal,
            "!=" => CompOp::NotEqual,
            "<" => CompOp::LessThan,
            "<=" => CompOp::LessEqual,
            ">" => CompOp::GreaterThan,
            ">=" => CompOp::GreaterEqual,
            _ => unreachable!("Unexpected comparison operator: {}", op_str),
        }
    }
}

/// Parsed CLPZ constraints
struct ClpzConstraints<U: User, E: Engine<U>> {
    constraints: Vec<ClpzConstraint<U, E>>,
    source_span: super::super::parser::ast::Span,
}

impl<U: User, E: Engine<U>> DomainConstraints<U, E> for ClpzConstraints<U, E> {
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

// Data model for CLPZ constraints
#[derive(Debug, Clone)]
pub enum ClpzConstraint<U: User, E: Engine<U>> {
    Expression {
        left: ArithExpr,
        op: CompOp,
        right: ArithExpr,
        _phantom: std::marker::PhantomData<(U, E)>,
    },
    Fresh {
        vars: Vec<String>,
        constraints: Vec<ClpzConstraint<U, E>>,
        _phantom: std::marker::PhantomData<(U, E)>,
    },
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Copy)]
pub enum ArithOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, Copy)]
pub enum CompOp {
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
}

impl<U: User, E: Engine<U>> ClpzConstraint<U, E> {
    fn convert_to_goal(
        &self,
        execution_context: &mut ExecutionContext<U, E>,
        source_span: &super::super::parser::ast::Span,
    ) -> Result<Goal<U, E>, InterpreterError> {
        match self {
            ClpzConstraint::Expression {
                left, op, right, ..
            } => {
                let left_term = eval_arith_expr(left, execution_context)?;
                let right_term = eval_arith_expr(right, execution_context)?;
                build_comparison_goal(left_term, *op, right_term)
            }
            ClpzConstraint::Fresh {
                vars, constraints, ..
            } => {
                execution_context.push_scope();
                for var in vars {
                    let fresh_var = execution_context.create_fresh_var();
                    execution_context.bind_var(var.clone(), fresh_var);
                }

                let mut goals = vec![];
                for constraint in constraints {
                    goals.push(constraint.convert_to_goal(execution_context, source_span)?);
                }

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

    fn extract_variables(&self) -> Vec<String> {
        let mut vars = Vec::new();
        match self {
            ClpzConstraint::Expression { left, right, .. } => {
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
            ClpzConstraint::Fresh {
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

/// Evaluates an arithmetic expression using CLPZ relations
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
                            "Meta expression expansion error in CLPZ arithmetic: {}",
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
                    use crate::relation::clpz::plusz::plusz;
                    plusz(left_term, right_term, result_term.clone()).cast_into()
                }
                ArithOp::Subtract => {
                    use crate::relation::clpz::plusz::plusz;
                    // For x - y = z, we use x = y + z, so plusz(right_term, result_term, left_term)
                    plusz(right_term, result_term.clone(), left_term).cast_into()
                }
                ArithOp::Multiply => {
                    use crate::relation::clpz::timesz::timesz;
                    timesz(left_term, right_term, result_term.clone()).cast_into()
                }
                ArithOp::Divide => {
                    use crate::relation::clpz::timesz::timesz;
                    // For x / y = z, we use z * y = x
                    timesz(result_term.clone(), right_term, left_term).cast_into()
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
            use crate::relation::eq::eq;
            Ok(eq(left, right).cast_into())
        }
        CompOp::NotEqual => {
            use crate::relation::diseq::diseq;
            Ok(diseq(left, right).cast_into())
        }
        CompOp::LessThan => {
            use crate::relation::clpz::ltz::ltz;
            Ok(ltz(left, right).cast_into())
        }
        CompOp::LessEqual => {
            use crate::relation::clpz::ltez::ltez;
            Ok(ltez(left, right).cast_into())
        }
        CompOp::GreaterThan => {
            // a > b becomes b < a
            use crate::relation::clpz::ltz::ltz;
            Ok(ltz(right, left).cast_into())
        }
        CompOp::GreaterEqual => {
            // a >= b becomes b <= a
            use crate::relation::clpz::ltez::ltez;
            Ok(ltez(right, left).cast_into())
        }
    }
}
