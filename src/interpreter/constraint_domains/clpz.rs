//! CLPZ (Constraint Logic Programming over Integers) Implementation
//!
//! This domain provides integer constraint syntax like:
//! - x + y == z (arithmetic constraints)
//! - x * y == z (multiplication constraints)
//! - x < y (comparison constraints)

use super::{ConstraintCompiler, ConstraintTemplate, FreshVariableContext, RuntimeValue, VariableType};
use crate::goal::{AnyGoal, Goal, GoalCast};
use crate::interpreter::parser::ast::ConstraintBody;
use crate::interpreter::InterpreterError;
use crate::interpreter::metaprogramming::{MetaValue, evaluate_meta_expression};
use crate::interpreter::runtime::ExecutionContext;
use crate::lterm::LTerm;
use crate::operator::conj::Conj;
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Parser)]
#[grammar = "interpreter/constraint_domains/grammars/clpz.pest"]
pub struct ClpzParser;

/// CLPZ constraint template that stores compiled constraint information
#[derive(Debug, Clone)]
pub struct ClpzTemplate {
    body: ConstraintBody,
    /// External relational variables referenced by this template
    external_relational_variables: Vec<String>,
    /// External meta variables referenced by this template
    external_meta_variables: Vec<String>,
    constraints: Vec<ClpzConstraint>,
}


impl ConstraintTemplate for ClpzTemplate {
    fn to_goal_with_context(
        &self,
        external_binder: &dyn Fn(&str) -> Option<RuntimeValue>,
        fresh_context: &mut FreshVariableContext,
    ) -> Result<Goal, InterpreterError> {
        
        // Start with succeed and chain all constraints
        let mut result = Goal::succeed();

        // Build conjunction chain using new context-based approach
        for constraint in self.constraints.iter() {
            let goal = constraint.to_goal_with_context(external_binder, fresh_context)?;
            result = Conj::new(result, goal).cast_into();
        }

        Ok(result)
    }
    
    fn external_relational_variables(&self) -> &[String] {
        &self.external_relational_variables
    }
    
    fn external_meta_variables(&self) -> &[String] {
        &self.external_meta_variables
    }
}

/// CLPZ constraint compiler
#[derive(Debug)]
pub struct ClpzCompiler;

impl ClpzCompiler {
    pub fn new() -> Self {
        Self
    }
}

impl ConstraintCompiler for ClpzCompiler {
    fn name(&self) -> &str {
        "clpz"
    }

    fn syntax_help(&self) -> &str {
        r#"CLPZ Syntax:
- Arithmetic: x + y == z, x - y == z, x * y == z
- Comparison: x < y, x <= y, x > y, x >= y, x != y, x == y
- Fresh: |x, y| { x + y == z }"#
    }

    fn compile(
        &self,
        body: &ConstraintBody,
        binder: &dyn Fn(&str) -> Option<VariableType>,
    ) -> Result<Rc<dyn ConstraintTemplate>, InterpreterError> {
        // Parse constraints and validate variables using binder
        let constraints = self.parse_constraints_with_binder(&body.raw_content, binder)?;

        // Extract external variables and separate by type
        let mut external_relational_variables = Vec::new();
        let mut external_meta_variables = Vec::new();
        
        for constraint in &constraints {
            let variables = constraint.extract_variables();
            for var_name in variables {
                // Skip variables that look like constants (numeric)
                if var_name.parse::<i32>().is_ok() {
                    continue;
                }

                // CHANGED: Treat all variables as potentially external (relational)
                // The runtime external binder will handle the actual resolution
                // This fixes the timing issue where fresh variables don't exist at template creation time
                if !external_relational_variables.contains(&var_name) {
                    external_relational_variables.push(var_name);
                }
            }
        }
        
        // Sort for consistent ordering
        external_relational_variables.sort();
        external_meta_variables.sort();

        
        // Create template with parsed constraints and external variable tracking
        let template = ClpzTemplate {
            body: body.clone(),
            external_relational_variables,
            external_meta_variables,
            constraints,
        };
        Ok(Rc::new(template))
    }
}

impl ClpzCompiler {
    /// Parse constraints with binder validation for the new compile API
    fn parse_constraints_with_binder(
        &self,
        content: &str,
        binder: &dyn Fn(&str) -> Option<VariableType>,
    ) -> Result<Vec<ClpzConstraint>, InterpreterError> {
        // First parse constraints normally
        let constraints = self.parse_constraints_block(content)?;

        // Validate all variables using the binder
        for constraint in &constraints {
            let variables = constraint.extract_variables();
            for var_name in variables {
                // Skip variables that look like constants (numeric)
                if var_name.parse::<i32>().is_ok() {
                    continue;
                }

                // Check if variable can be bound
                binder(&var_name)
                    .ok_or_else(|| InterpreterError::UnknownVariable(var_name.clone()))?;
            }
        }

        Ok(constraints)
    }

    /// Parse multiple constraints from constraint body (like "x + y == z, x < y")
    fn parse_constraints_block(
        &self,
        content: &str,
    ) -> Result<Vec<ClpzConstraint>, InterpreterError> {
        let pairs = ClpzParser::parse(Rule::constraints, content.trim()).map_err(|e| {
            InterpreterError::InvalidConstraintSyntax {
                domain: "clpz".to_string(),
                error: format!("Parse error: {}", e),
            }
        })?;

        let mut constraints = Vec::new();
        let dummy_span = super::super::parser::ast::Location::dummy();

        for pair in pairs {
            if pair.as_rule() == Rule::constraints {
                // Parse each constraint in the constraints list
                for constraint_pair in pair.into_inner() {
                    if constraint_pair.as_rule() == Rule::constraint {
                        constraints.push(Self::build_constraint(constraint_pair, &dummy_span)?);
                    }
                }
            }
        }

        Ok(constraints)
    }

    fn build_constraint(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Location,
    ) -> Result<ClpzConstraint, InterpreterError> {
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

    fn build_fresh_constraint(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Location,
    ) -> Result<ClpzConstraint, InterpreterError> {
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

        Ok(ClpzConstraint::Fresh { vars, constraints })
    }

    fn build_arith_constraint(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Location,
    ) -> Result<ClpzConstraint, InterpreterError> {
        let mut inner = pair.into_inner();
        let left = Self::build_arith_expr(inner.next().unwrap(), source_span);
        let op = Self::build_comp_op(inner.next().unwrap());
        let right = Self::build_arith_expr(inner.next().unwrap(), source_span);

        Ok(ClpzConstraint::Expression { left, op, right })
    }

    fn build_arith_expr(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Location,
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
                                super::super::parser::ast::Location::dummy(),
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
                                super::super::parser::ast::Location::dummy(),
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

// Data model for CLPZ constraints
#[derive(Debug, Clone)]
pub enum ClpzConstraint {
    Expression {
        left: ArithExpr,
        op: CompOp,
        right: ArithExpr,
    },
    Fresh {
        vars: Vec<String>,
        constraints: Vec<ClpzConstraint>,
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

impl ClpzConstraint {
    /// Convert constraint to goal using external binder and fresh variable context
    fn to_goal_with_context(
        &self,
        external_binder: &dyn Fn(&str) -> Option<RuntimeValue>,
        fresh_context: &mut FreshVariableContext,
    ) -> Result<Goal, InterpreterError> {
        match self {
            ClpzConstraint::Expression { left, op, right } => {
                // Handle arithmetic expressions specially for equality constraints
                if matches!(op, CompOp::Equal) {
                    // Check for patterns like: x + y == z, x * y == z, etc.
                    match (left, right) {
                        // Pattern: (x + y) == z
                        (ArithExpr::BinaryOp { left: x, op: ArithOp::Add, right: y }, z) => {
                            let x_term = self.eval_arith_expr_with_context(x, external_binder, fresh_context)?;
                            let y_term = self.eval_arith_expr_with_context(y, external_binder, fresh_context)?;
                            let z_term = self.eval_arith_expr_with_context(z, external_binder, fresh_context)?;
                            
                            use crate::relation::clpz::plusz::plusz;
                            Ok(plusz(x_term, y_term, z_term).cast_into())
                        }
                        // Pattern: z == (x + y)
                        (z, ArithExpr::BinaryOp { left: x, op: ArithOp::Add, right: y }) => {
                            let x_term = self.eval_arith_expr_with_context(x, external_binder, fresh_context)?;
                            let y_term = self.eval_arith_expr_with_context(y, external_binder, fresh_context)?;
                            let z_term = self.eval_arith_expr_with_context(z, external_binder, fresh_context)?;
                            
                            use crate::relation::clpz::plusz::plusz;
                            Ok(plusz(x_term, y_term, z_term).cast_into())
                        }
                        // Pattern: (x * y) == z
                        (ArithExpr::BinaryOp { left: x, op: ArithOp::Multiply, right: y }, z) => {
                            let x_term = self.eval_arith_expr_with_context(x, external_binder, fresh_context)?;
                            let y_term = self.eval_arith_expr_with_context(y, external_binder, fresh_context)?;
                            let z_term = self.eval_arith_expr_with_context(z, external_binder, fresh_context)?;
                            
                            use crate::relation::clpz::timesz::timesz;
                            Ok(timesz(x_term, y_term, z_term).cast_into())
                        }
                        // Pattern: z == (x * y)
                        (z, ArithExpr::BinaryOp { left: x, op: ArithOp::Multiply, right: y }) => {
                            let x_term = self.eval_arith_expr_with_context(x, external_binder, fresh_context)?;
                            let y_term = self.eval_arith_expr_with_context(y, external_binder, fresh_context)?;
                            let z_term = self.eval_arith_expr_with_context(z, external_binder, fresh_context)?;
                            
                            use crate::relation::clpz::timesz::timesz;
                            Ok(timesz(x_term, y_term, z_term).cast_into())
                        }
                        // Pattern: (x - y) == z  =>  y + z == x
                        (ArithExpr::BinaryOp { left: x, op: ArithOp::Subtract, right: y }, z) => {
                            let x_term = self.eval_arith_expr_with_context(x, external_binder, fresh_context)?;
                            let y_term = self.eval_arith_expr_with_context(y, external_binder, fresh_context)?;
                            let z_term = self.eval_arith_expr_with_context(z, external_binder, fresh_context)?;
                            
                            use crate::relation::clpz::plusz::plusz;
                            Ok(plusz(y_term, z_term, x_term).cast_into())
                        }
                        // Pattern: z == (x - y)  =>  y + z == x  
                        (z, ArithExpr::BinaryOp { left: x, op: ArithOp::Subtract, right: y }) => {
                            let x_term = self.eval_arith_expr_with_context(x, external_binder, fresh_context)?;
                            let y_term = self.eval_arith_expr_with_context(y, external_binder, fresh_context)?;
                            let z_term = self.eval_arith_expr_with_context(z, external_binder, fresh_context)?;
                            
                            use crate::relation::clpz::plusz::plusz;
                            Ok(plusz(y_term, z_term, x_term).cast_into())
                        }
                        // General case: evaluate both sides and compare
                        _ => {
                            let left_term = self.eval_arith_expr_with_context(left, external_binder, fresh_context)?;
                            let right_term = self.eval_arith_expr_with_context(right, external_binder, fresh_context)?;
                            
                            use crate::relation::eq::eq;
                            Ok(eq(left_term, right_term).cast_into())
                        }
                    }
                } else {
                    // For non-equality operations, evaluate both sides and use generic comparison
                    let left_term = self.eval_arith_expr_with_context(left, external_binder, fresh_context)?;
                    let right_term = self.eval_arith_expr_with_context(right, external_binder, fresh_context)?;
                    
                    match op {
                        CompOp::NotEqual => {
                            // ClpZ doesn't have a direct neqz, use generic diseq
                            use crate::relation::diseq::diseq;
                            Ok(diseq(left_term, right_term).cast_into())
                        }
                        CompOp::LessThan => {
                            use crate::relation::clpz::ltz::ltz;
                            Ok(ltz(left_term, right_term).cast_into())
                        }
                        CompOp::LessEqual => {
                            use crate::relation::clpz::ltez::ltez;
                            Ok(ltez(left_term, right_term).cast_into())
                        }
                        CompOp::GreaterThan => {
                            use crate::relation::clpz::ltz::ltz;
                            Ok(ltz(right_term, left_term).cast_into()) // x > y ≡ y < x
                        }
                        CompOp::GreaterEqual => {
                            use crate::relation::clpz::ltez::ltez;
                            Ok(ltez(right_term, left_term).cast_into()) // x >= y ≡ y <= x
                        }
                        CompOp::Equal => unreachable!(), // Already handled above
                    }
                }
            }
            ClpzConstraint::Fresh { vars, constraints } => {
                
                // Push fresh variable scope with relational variables
                let fresh_relational_vars: Vec<String> = vars.clone();
                let fresh_meta_vars: Vec<(String, MetaValue)> = Vec::new(); // No meta vars in ClpZ fresh blocks
                
                let (fresh_lterms, _) = fresh_context.push_scope(&fresh_relational_vars, &fresh_meta_vars);

                // Convert all sub-constraints using the fresh context
                let mut body_goal = Goal::succeed();
                for constraint in constraints {
                    let goal = constraint.to_goal_with_context(external_binder, fresh_context)?;
                    body_goal = Conj::new(body_goal, goal).cast_into();
                }

                // Pop the fresh variable scope
                fresh_context.pop_scope();

                // Wrap the body in a Fresh operator
                use crate::operator::fresh::Fresh;
                Ok(Fresh::new(fresh_lterms, body_goal).cast_into())
            }
        }
    }

    /// Evaluate arithmetic expression using external binder and fresh context
    fn eval_arith_expr_with_context(
        &self,
        expr: &ArithExpr,
        external_binder: &dyn Fn(&str) -> Option<RuntimeValue>,
        fresh_context: &mut FreshVariableContext,
    ) -> Result<LTerm, InterpreterError> {
        match expr {
            ArithExpr::Integer(val) => Ok(LTerm::from(*val as isize)),
            ArithExpr::Variable(name) => {
                
                // First check if it's a fresh variable
                if let Some(runtime_value) = fresh_context.resolve_fresh_variable(name) {
                    match runtime_value {
                        RuntimeValue::Relational(lterm) => {
                            Ok(lterm)
                        },
                        RuntimeValue::Meta(meta_value) => {
                            // Convert meta value to LTerm for relational context
                            let lterm = meta_value.to_lterm();
                            Ok(lterm)
                        }
                    }
                } else {
                    // Check external binder
                    if let Some(runtime_value) = external_binder(name) {
                        match runtime_value {
                            RuntimeValue::Relational(lterm) => {
                                Ok(lterm)
                            },
                            RuntimeValue::Meta(meta_value) => {
                                // Convert meta value to LTerm for relational context
                                let lterm = meta_value.to_lterm();
                                Ok(lterm)
                            }
                        }
                    } else {
                        Err(InterpreterError::UnknownVariable(name.clone()))
                    }
                }
            }
            ArithExpr::Interpolation(meta_expr) => {
                // Evaluate meta expression using current context
                let meta_bindings = HashMap::new();
                
                // Get meta variables from external binder and fresh context
                // Note: For meta interpolation, we would need external meta variable values
                // For now, we'll handle simple variable interpolation
                
                let meta_value = evaluate_meta_expression(meta_expr, &meta_bindings)
                    .map_err(|e| InterpreterError::RuntimeError(format!(
                        "Meta expression evaluation error in CLPZ constraint: {}", e
                    )))?;
                
                Ok(meta_value.to_lterm())
            }
            ArithExpr::BinaryOp { left, op, right } => {
                // For binary operations in constraint expressions, we should create compound goals
                // However, since this is inside eval_arith_expr_with_context which expects a single LTerm result,
                // we need to handle this at the constraint level, not the expression level.
                // This suggests the constraint should be restructured to handle arithmetic directly.
                
                // For now, we'll return an error with better guidance
                Err(InterpreterError::RuntimeError(
                    format!("Complex arithmetic expressions like '{} {} {}' should be handled as top-level constraints, not as sub-expressions. Use constraints like 'x + y == z' directly.", 
                        format_arith_expr(left), format_arith_op(op), format_arith_expr(right))
                ))
            }
        }
    }

    /// Convert constraint to goal using pre-resolved variables (OLD API - to be removed)
    fn convert_to_goal_with_resolved_vars(
        &self,
        resolved_variables: std::collections::HashMap<String, super::ResolvedValue>,
    ) -> Result<Goal, InterpreterError> {
        match self {
            ClpzConstraint::Expression { left, op, right } => {
                // Handle arithmetic expressions specially for equality constraints
                if matches!(op, CompOp::Equal) {
                    // Check for patterns like: x + y == z, x * y == z, etc.
                    match (left, right) {
                        // Pattern: (x + y) == z
                        (ArithExpr::BinaryOp { left: x, op: ArithOp::Add, right: y }, z) => {
                            let x_term = self.eval_arith_expr_with_resolved_vars(x, &resolved_variables)?;
                            let y_term = self.eval_arith_expr_with_resolved_vars(y, &resolved_variables)?;
                            let z_term = self.eval_arith_expr_with_resolved_vars(z, &resolved_variables)?;
                            
                            use crate::relation::clpz::plusz::plusz;
                            Ok(plusz(x_term, y_term, z_term).cast_into())
                        }
                        // Pattern: z == (x + y)
                        (z, ArithExpr::BinaryOp { left: x, op: ArithOp::Add, right: y }) => {
                            let x_term = self.eval_arith_expr_with_resolved_vars(x, &resolved_variables)?;
                            let y_term = self.eval_arith_expr_with_resolved_vars(y, &resolved_variables)?;
                            let z_term = self.eval_arith_expr_with_resolved_vars(z, &resolved_variables)?;
                            
                            use crate::relation::clpz::plusz::plusz;
                            Ok(plusz(x_term, y_term, z_term).cast_into())
                        }
                        // Pattern: (x - y) == z
                        (ArithExpr::BinaryOp { left: x, op: ArithOp::Subtract, right: y }, z) => {
                            let x_term = self.eval_arith_expr_with_resolved_vars(x, &resolved_variables)?;
                            let y_term = self.eval_arith_expr_with_resolved_vars(y, &resolved_variables)?;
                            let z_term = self.eval_arith_expr_with_resolved_vars(z, &resolved_variables)?;
                            
                            // For x - y == z, we use x == y + z, so plusz(y, z, x)
                            use crate::relation::clpz::plusz::plusz;
                            Ok(plusz(y_term, z_term, x_term).cast_into())
                        }
                        // Pattern: z == (x - y)
                        (z, ArithExpr::BinaryOp { left: x, op: ArithOp::Subtract, right: y }) => {
                            let x_term = self.eval_arith_expr_with_resolved_vars(x, &resolved_variables)?;
                            let y_term = self.eval_arith_expr_with_resolved_vars(y, &resolved_variables)?;
                            let z_term = self.eval_arith_expr_with_resolved_vars(z, &resolved_variables)?;
                            
                            // For z == x - y, we use x == y + z, so plusz(y, z, x)
                            use crate::relation::clpz::plusz::plusz;
                            Ok(plusz(y_term, z_term, x_term).cast_into())
                        }
                        // Pattern: (x * y) == z
                        (ArithExpr::BinaryOp { left: x, op: ArithOp::Multiply, right: y }, z) => {
                            let x_term = self.eval_arith_expr_with_resolved_vars(x, &resolved_variables)?;
                            let y_term = self.eval_arith_expr_with_resolved_vars(y, &resolved_variables)?;
                            let z_term = self.eval_arith_expr_with_resolved_vars(z, &resolved_variables)?;
                            
                            use crate::relation::clpz::timesz::timesz;
                            Ok(timesz(x_term, y_term, z_term).cast_into())
                        }
                        // Pattern: z == (x * y)
                        (z, ArithExpr::BinaryOp { left: x, op: ArithOp::Multiply, right: y }) => {
                            let x_term = self.eval_arith_expr_with_resolved_vars(x, &resolved_variables)?;
                            let y_term = self.eval_arith_expr_with_resolved_vars(y, &resolved_variables)?;
                            let z_term = self.eval_arith_expr_with_resolved_vars(z, &resolved_variables)?;
                            
                            use crate::relation::clpz::timesz::timesz;
                            Ok(timesz(x_term, y_term, z_term).cast_into())
                        }
                        // Pattern: (x / y) == z
                        (ArithExpr::BinaryOp { left: x, op: ArithOp::Divide, right: y }, z) => {
                            let x_term = self.eval_arith_expr_with_resolved_vars(x, &resolved_variables)?;
                            let y_term = self.eval_arith_expr_with_resolved_vars(y, &resolved_variables)?;
                            let z_term = self.eval_arith_expr_with_resolved_vars(z, &resolved_variables)?;
                            
                            // For x / y == z, we use z * y == x
                            use crate::relation::clpz::timesz::timesz;
                            Ok(timesz(z_term, y_term, x_term).cast_into())
                        }
                        // Pattern: z == (x / y)
                        (z, ArithExpr::BinaryOp { left: x, op: ArithOp::Divide, right: y }) => {
                            let x_term = self.eval_arith_expr_with_resolved_vars(x, &resolved_variables)?;
                            let y_term = self.eval_arith_expr_with_resolved_vars(y, &resolved_variables)?;
                            let z_term = self.eval_arith_expr_with_resolved_vars(z, &resolved_variables)?;
                            
                            // For z == x / y, we use z * y == x
                            use crate::relation::clpz::timesz::timesz;
                            Ok(timesz(z_term, y_term, x_term).cast_into())
                        }
                        // Simple case: no arithmetic on either side
                        _ => {
                            let left_term = self.eval_arith_expr_with_resolved_vars(left, &resolved_variables)?;
                            let right_term = self.eval_arith_expr_with_resolved_vars(right, &resolved_variables)?;
                            
                            use crate::relation::eq::eq;
                            Ok(eq(left_term, right_term).cast_into())
                        }
                    }
                } else {
                    // For non-equality operators, evaluate both sides as simple terms
                    let left_term = self.eval_arith_expr_with_resolved_vars(left, &resolved_variables)?;
                    let right_term = self.eval_arith_expr_with_resolved_vars(right, &resolved_variables)?;

                    // Build the appropriate comparison goal
                    match op {
                        CompOp::Equal => unreachable!(), // Handled above
                        CompOp::NotEqual => {
                            use crate::relation::diseq::diseq;
                            Ok(diseq(left_term, right_term).cast_into())
                        }
                        CompOp::LessThan => {
                            use crate::relation::clpz::ltz::ltz;
                            Ok(ltz(left_term, right_term).cast_into())
                        }
                        CompOp::LessEqual => {
                            use crate::relation::clpz::ltez::ltez;
                            Ok(ltez(left_term, right_term).cast_into())
                        }
                        CompOp::GreaterThan => {
                            use crate::relation::clpz::ltz::ltz;
                            Ok(ltz(right_term, left_term).cast_into())
                        }
                        CompOp::GreaterEqual => {
                            use crate::relation::clpz::ltez::ltez;
                            Ok(ltez(right_term, left_term).cast_into())
                        }
                    }
                }
            }
            ClpzConstraint::Fresh { vars, constraints } => {
                // Create fresh variables using the Fresh operator
                // Convert variable names to LTerms
                let fresh_vars: Vec<LTerm> =
                    vars.iter().map(|var_name| LTerm::var(var_name)).collect();

                // Create a new resolved variables map that includes the fresh variables
                let mut extended_resolved_vars = resolved_variables.clone();
                for var_name in vars {
                    // Add the fresh variables to the resolved map as relational variables
                    extended_resolved_vars.insert(
                        var_name.clone(),
                        super::ResolvedValue::Relational(LTerm::var(var_name)),
                    );
                }

                // Convert all sub-constraints using the extended variable map
                // Start with succeed and chain all constraints
                let mut body_goal = Goal::succeed();

                // Build conjunction chain directly
                for constraint in constraints {
                    let goal = constraint
                        .convert_to_goal_with_resolved_vars(extended_resolved_vars.clone())?;
                    body_goal = Conj::new(body_goal, goal).cast_into();
                }

                // Wrap the body in a Fresh operator
                use crate::operator::fresh::Fresh;
                Ok(Fresh::new(fresh_vars, body_goal).cast_into())
            }
        }
    }

    /// Evaluate arithmetic expression using resolved variables (new API)
    fn eval_arith_expr_with_resolved_vars(
        &self,
        expr: &ArithExpr,
        resolved_variables: &std::collections::HashMap<String, super::ResolvedValue>,
    ) -> Result<LTerm, InterpreterError> {
        match expr {
            ArithExpr::Integer(val) => Ok(LTerm::from(*val as isize)),
            ArithExpr::Variable(name) => {
                let resolved_value = resolved_variables
                    .get(name)
                    .ok_or_else(|| InterpreterError::UnknownVariable(name.clone()))?;

                match resolved_value {
                    super::ResolvedValue::Relational(lterm) => Ok(lterm.clone()),
                    super::ResolvedValue::Meta(_) => Err(InterpreterError::RuntimeError(
                        "Meta variables not yet supported in ClpZ arithmetic expressions"
                            .to_string(),
                    )),
                }
            }
            ArithExpr::Interpolation(_) => Err(InterpreterError::RuntimeError(
                "Interpolation expressions not yet supported in ClpZ resolved variables API"
                    .to_string(),
            )),
            ArithExpr::BinaryOp { left, op, right } => {
                // For binary operations in constraint expressions, we should create compound goals
                // However, since this is inside eval_arith_expr_with_resolved_vars which expects a single LTerm result,
                // we need to handle this at the constraint level, not the expression level.
                // This suggests the constraint should be restructured to handle arithmetic directly.
                
                // For now, we'll return an error with better guidance
                Err(InterpreterError::RuntimeError(
                    format!("Complex arithmetic expressions like '{} {} {}' should be handled as top-level constraints, not as sub-expressions. Use constraints like 'x + y == z' directly.", 
                        format_arith_expr(left), format_arith_op(op), format_arith_expr(right))
                ))
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
            } => {
                for c in constraints {
                    vars.extend(c.extract_variables());
                }
            }
        }
        vars
    }

    /// Convert constraint to goal using IR execution context (OLD API - to be removed)
    fn convert_to_goal_ir(
        &self,
        _execution_context: &mut crate::interpreter::runtime::context::ExecutionContext,
        _variables: &HashMap<String, super::VariableInfo>,
    ) -> Result<Goal, InterpreterError> {
        // This is the old API that should be removed once migration is complete
        Err(InterpreterError::RuntimeError(
            "convert_to_goal_ir is deprecated - use to_goal_with_context instead".to_string()
        ))
    }
}

/// Helper function to format arithmetic expressions for error messages
fn format_arith_expr(expr: &ArithExpr) -> String {
    match expr {
        ArithExpr::Integer(val) => val.to_string(),
        ArithExpr::Variable(name) => name.clone(),
        ArithExpr::Interpolation(_) => "{...}".to_string(),
        ArithExpr::BinaryOp { left, op, right } => {
            format!("({} {} {})", format_arith_expr(left), format_arith_op(op), format_arith_expr(right))
        }
    }
}

/// Helper function to format arithmetic operators for error messages
fn format_arith_op(op: &ArithOp) -> &'static str {
    match op {
        ArithOp::Add => "+",
        ArithOp::Subtract => "-",
        ArithOp::Multiply => "*",
        ArithOp::Divide => "/",
    }
}

/// Evaluates an arithmetic expression using CLPZ relations
fn eval_arith_expr(
    expr: &ArithExpr,
    execution_context: &mut ExecutionContext,
) -> Result<LTerm, InterpreterError> {
    match expr {
        ArithExpr::Integer(val) => Ok(LTerm::from(*val as isize)),
        ArithExpr::Variable(name) => {
            let symbol = crate::interpreter::symbol_table::InternedSymbol::from(name.clone());
            execution_context
                .lookup_var(&symbol)
                .ok_or_else(|| InterpreterError::UnknownVariable(name.clone()))
        }
        ArithExpr::Interpolation(meta_expr) => {
            // For simple variable interpolations, directly access the execution context
            match meta_expr {
                crate::interpreter::metaprogramming::MetaExpression::Variable(var_name, _) => {
                    // Directly look up the variable in the execution context
                    let symbol =
                        crate::interpreter::symbol_table::InternedSymbol::from(var_name.clone());
                    execution_context.lookup_var(&symbol).ok_or_else(|| {
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
                    let _expanded_term = expand_term(&dummy_term, &context).map_err(|e| {
                        InterpreterError::RuntimeError(format!(
                            "Meta expression expansion error in CLPZ arithmetic: {}",
                            e
                        ))
                    })?;

                    // Complex metaprogramming expansion should be handled in IR template system
                    Err(InterpreterError::RuntimeError(
                        "Complex metaprogramming interpolation in constraint arithmetic not yet supported in IR mode".to_string()
                    ))
                }
            }
        }
        ArithExpr::BinaryOp { left, op, right } => {
            let left_term = eval_arith_expr(left, execution_context)?;
            let right_term = eval_arith_expr(right, execution_context)?;
            let result_term = execution_context.create_fresh_var();

            let _goal: Goal = match op {
                ArithOp::Add => {
                    use crate::relation::clpz::plusz::plusz;
                    plusz::<Goal>(left_term, right_term, result_term.clone()).cast_into()
                }
                ArithOp::Subtract => {
                    use crate::relation::clpz::plusz::plusz;
                    // For x - y = z, we use x = y + z, so plusz(right_term, result_term, left_term)
                    plusz::<Goal>(right_term, result_term.clone(), left_term).cast_into()
                }
                ArithOp::Multiply => {
                    use crate::relation::clpz::timesz::timesz;
                    timesz::<Goal>(left_term, right_term, result_term.clone()).cast_into()
                }
                ArithOp::Divide => {
                    use crate::relation::clpz::timesz::timesz;
                    // For x / y = z, we use z * y = x
                    timesz::<Goal>(result_term.clone(), right_term, left_term).cast_into()
                }
            };

            // In IR execution, we don't defer goals - they should be composed into the main goal
            // For now, we'll have to restructure this to return the goal along with the term
            // This is a limitation of the current arithmetic evaluation design
            Err(InterpreterError::RuntimeError(
                "Complex arithmetic expressions with constraints not yet supported in IR mode"
                    .to_string(),
            ))
        }
    }
}

fn build_comparison_goal(left: LTerm, op: CompOp, right: LTerm) -> Result<Goal, InterpreterError> {
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
