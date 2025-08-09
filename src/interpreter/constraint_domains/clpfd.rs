//! CLPFD (Constraint Logic Programming over Finite Domains) Implementation
//!
//! This domain provides finite domain constraint syntax like:
//! - x in 1..10 (domain constraints)
//! - x + y == z (arithmetic constraints)  
//! - x < y (comparison constraints)
//! - distinct [x, y, z] (global constraints)

use super::{ConstraintDomain, DomainConstraintTemplate, FreshVariableContext, RuntimeValue, VariableType};
use crate::goal::{AnyGoal, Goal, GoalCast};
use crate::interpreter::parser::ast::ConstraintBody;
use crate::interpreter::parser::meta_parser;
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
#[grammar = "interpreter/constraint_domains/grammars/clpfd.pest"]
pub struct ClpfdParser;

/// CLPFD constraint template that stores compiled constraint information
#[derive(Debug, Clone)]
pub struct ClpfdTemplate {
    body: ConstraintBody,
    /// External relational variables referenced by this template
    external_relational_variables: Vec<String>,
    /// External meta variables referenced by this template
    external_meta_variables: Vec<String>,
    constraints: Vec<ClpfdConstraint>,
}


impl DomainConstraintTemplate for ClpfdTemplate {
    fn to_goal_with_context(
        &self,
        external_binder: &dyn Fn(&str) -> Option<RuntimeValue>,
        fresh_context: &mut FreshVariableContext,
    ) -> Result<Goal, InterpreterError> {
        // Start with succeed and chain all constraints
        let mut result = Goal::succeed();

        // Build conjunction chain using new context-based approach
        for constraint in &self.constraints {
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

/// CLPFD constraint domain
pub struct ClpfdDomain;

impl ClpfdDomain {
    pub fn new() -> Self {
        Self
    }
}

impl ConstraintDomain for ClpfdDomain {
    fn name(&self) -> &str {
        "clpfd"
    }

    fn syntax_help(&self) -> &str {
        r#"CLPFD Syntax:
- Domain constraints: x in 1..10, x in [1,2,3], [x, y, z] in 0..2
- Arithmetic: x + y == z, x - y == z, x * y == z
- Comparison: x < y, x <= y, x > y, x >= y, x != y
- Global: distinct [x, y, z], alldiff [x, y, z]
- Fresh: |x, y| { x < y, x in 1..10 }"#
    }

    fn compile(
        &self,
        body: &ConstraintBody,
        binder: &dyn Fn(&str) -> Option<VariableType>,
    ) -> Result<Rc<dyn DomainConstraintTemplate>, InterpreterError> {
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
        let template = ClpfdTemplate {
            body: body.clone(),
            external_relational_variables,
            external_meta_variables,
            constraints,
        };
        Ok(Rc::new(template))
    }
}

impl ClpfdDomain {
    /// Parse constraints with binder validation for the new compile API
    fn parse_constraints_with_binder(
        &self,
        content: &str,
        binder: &dyn Fn(&str) -> Option<VariableType>,
    ) -> Result<Vec<ClpfdConstraint>, InterpreterError> {
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

    /// Strip line comments (// ...) from constraint content
    fn strip_comments(&self, content: &str) -> String {
        content
            .lines()
            .map(|line| {
                // Find the position of "//" that's not inside a string
                if let Some(comment_start) = line.find("//") {
                    &line[..comment_start]
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Parse multiple constraints from constraint body (like "x in 1..2, y in 1..2")
    fn parse_constraints_block(
        &self,
        content: &str,
    ) -> Result<Vec<ClpfdConstraint>, InterpreterError> {
        // Strip comments from the content before parsing
        let cleaned_content = self.strip_comments(content);
        let pairs = ClpfdParser::parse(Rule::constraints, cleaned_content.trim()).map_err(|e| {
            InterpreterError::InvalidConstraintSyntax {
                domain: "clpfd".to_string(),
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

    fn parse_individual_constraint(
        &self,
        content: &str,
    ) -> Result<ClpfdConstraint, InterpreterError> {
        let pairs = ClpfdParser::parse(Rule::constraint, content.trim()).map_err(|e| {
            InterpreterError::InvalidConstraintSyntax {
                domain: "clpfd".to_string(),
                error: format!("Parse error: {}", e),
            }
        })?;

        // There should be exactly one pair, corresponding to the `constraint` rule
        if let Some(pair) = pairs.peek() {
            let dummy_span = super::super::parser::ast::Location::dummy();
            Self::build_constraint(pair, &dummy_span)
        } else {
            Err(InterpreterError::InvalidConstraintSyntax {
                domain: "clpfd".to_string(),
                error: format!("No valid constraint found in: {}", content),
            })
        }
    }

    // -- AST Builder Functions --

    fn build_constraint(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Location,
    ) -> Result<ClpfdConstraint, InterpreterError> {
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

    fn build_fresh_constraint(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Location,
    ) -> Result<ClpfdConstraint, InterpreterError> {
        let mut inner = pair.into_inner();
        let mut vars = vec![];
        let mut constraints = vec![];

        // The first part could be var_list or constraints
        while let Some(part) = inner.next() {
            match part.as_rule() {
                Rule::var_list => {
                    vars = part.into_inner().map(|v| v.as_str().to_string()).collect();
                }
                Rule::constraints | Rule::constraint_list => {
                    for constraint_pair in part.into_inner() {
                        if constraint_pair.as_rule() == Rule::constraint {
                            constraints.push(Self::build_constraint(constraint_pair, source_span)?);
                        }
                    }
                }
                _ => unreachable!("Unexpected rule in fresh_constraint"),
            }
        }

        Ok(ClpfdConstraint::Fresh { vars, constraints })
    }

    fn build_domain_constraint(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Location,
    ) -> Result<ClpfdConstraint, InterpreterError> {
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
        })
    }

    fn build_list_domain_constraint(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Location,
    ) -> Result<ClpfdConstraint, InterpreterError> {
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
        })
    }

    fn build_domain_spec(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Location,
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
        source_span: &super::super::parser::ast::Location,
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

    fn build_distinct_constraint(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClpfdConstraint, InterpreterError> {
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
        })
    }

    fn build_alldiff_constraint(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClpfdConstraint, InterpreterError> {
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
        })
    }

    fn build_arith_constraint(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Location,
    ) -> Result<ClpfdConstraint, InterpreterError> {
        let mut inner = pair.into_inner();
        let left = Self::build_arith_expr(inner.next().unwrap(), source_span);
        let op = Self::build_comp_op(inner.next().unwrap());
        let right = Self::build_arith_expr(inner.next().unwrap(), source_span);

        Ok(ClpfdConstraint::Expression { left, op, right })
    }

    // This function uses the Pratt parser technique to handle operator precedence.
    fn build_arith_expr(
        pair: pest::iterators::Pair<Rule>,
        source_span: &super::super::parser::ast::Location,
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
                                super::super::parser::ast::Location::dummy(),
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
                                    super::super::parser::ast::Location::dummy(),
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

// Data model for CLPFD constraints
#[derive(Debug, Clone)]
pub enum ClpfdConstraint {
    Domain {
        variable: String,
        domain_spec: DomainSpec,
    },
    ListDomain {
        variables: Vec<String>,
        domain_spec: DomainSpec,
    },
    Expression {
        left: ArithExpr,
        op: CompOp,
        right: ArithExpr,
    },
    Global {
        name: String,
        args: Vec<String>,
    },
    Fresh {
        vars: Vec<String>,
        constraints: Vec<ClpfdConstraint>,
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

impl ClpfdConstraint {
    /// Convert constraint to goal using external binder and fresh variable context
    fn to_goal_with_context(
        &self,
        external_binder: &dyn Fn(&str) -> Option<RuntimeValue>,
        fresh_context: &mut FreshVariableContext,
    ) -> Result<Goal, InterpreterError> {
        match self {
            ClpfdConstraint::Domain {
                variable,
                domain_spec,
            } => {
                // Resolve variable using external binder and fresh context
                let var_term = self.resolve_variable_to_lterm(variable, external_binder, fresh_context)?;

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
                    DomainSpec::InterpolatedRange(start_bound, end_bound) => {
                        // Evaluate interpolated bounds to integers using new context approach
                        let start = self.eval_domain_bound_with_context(start_bound, external_binder, fresh_context)?;
                        let end = self.eval_domain_bound_with_context(end_bound, external_binder, fresh_context)?;
                        
                        use crate::relation::clpfd::infd::infdrange;
                        let range = start..=end;
                        Ok(infdrange(var_term, &range).cast_into())
                    }
                    DomainSpec::InterpolatedSet(bounds) => {
                        // Evaluate all interpolated bounds to integers using new context approach
                        let mut values = Vec::new();
                        for bound in bounds {
                            let value = self.eval_domain_bound_with_context(bound, external_binder, fresh_context)?;
                            values.push(value);
                        }
                        
                        use crate::relation::clpfd::infd::infd;
                        Ok(infd(var_term, &values).cast_into())
                    }
                }
            }
            ClpfdConstraint::Global { name, args } => {
                // Handle global constraints like alldiff/distinct
                let mut resolved_args = Vec::new();
                
                for arg_name in args {
                    // Check if this is a variable or a constant
                    if let Ok(constant_value) = arg_name.parse::<isize>() {
                        // It's a constant value, convert to LTerm directly
                        resolved_args.push(LTerm::from(constant_value));
                    } else {
                        // It's a variable, resolve it using context
                        let var_term = self.resolve_variable_to_lterm(arg_name, external_binder, fresh_context)?;
                        resolved_args.push(var_term);
                    }
                }
                
                // Build the appropriate global constraint goal
                match name.as_str() {
                    "distinct" | "alldiff" => {
                        use crate::relation::clpfd::distinctfd::distinctfd;
                        // Convert vector of LTerms to a list LTerm
                        let mut list_term = LTerm::empty_list();
                        for arg in resolved_args.into_iter().rev() {
                            list_term = LTerm::cons(arg, list_term);
                        }
                        Ok(distinctfd(list_term).cast_into())
                    }
                    _ => Err(InterpreterError::RuntimeError(format!(
                        "Unknown global constraint: {}",
                        name
                    ))),
                }
            }
            ClpfdConstraint::ListDomain { variables: var_list, domain_spec } => {
                // Handle list domain constraints like [x, y, z] in 1..3
                // Apply the domain constraint to each variable in the list
                let mut goals = Vec::new();
                
                for var_name in var_list {
                    // Resolve variable using context
                    let var_term = self.resolve_variable_to_lterm(var_name, external_binder, fresh_context)?;
                    
                    // Apply domain constraint to this variable
                    let domain_goal = match domain_spec {
                        DomainSpec::Range(start, end) => {
                            use crate::relation::clpfd::infd::infdrange;
                            let range = (*start as isize)..=(*end as isize);
                            infdrange(var_term, &range).cast_into()
                        }
                        DomainSpec::Set(values) => {
                            use crate::relation::clpfd::infd::infd;
                            let domain_values: Vec<isize> = values.iter().map(|&v| v as isize).collect();
                            infd(var_term, &domain_values).cast_into()
                        }
                        DomainSpec::InterpolatedRange(start_bound, end_bound) => {
                            let start = self.eval_domain_bound_with_context(start_bound, external_binder, fresh_context)?;
                            let end = self.eval_domain_bound_with_context(end_bound, external_binder, fresh_context)?;
                            
                            use crate::relation::clpfd::infd::infdrange;
                            let range = start..=end;
                            infdrange(var_term, &range).cast_into()
                        }
                        DomainSpec::InterpolatedSet(bounds) => {
                            let mut values = Vec::new();
                            for bound in bounds {
                                let value = self.eval_domain_bound_with_context(bound, external_binder, fresh_context)?;
                                values.push(value);
                            }
                            
                            use crate::relation::clpfd::infd::infd;
                            infd(var_term, &values).cast_into()
                        }
                    };
                    
                    goals.push(domain_goal);
                }
                
                // Combine all domain goals into a conjunction
                let mut result = Goal::succeed();
                for goal in goals {
                    result = Conj::new(result, goal).cast_into();
                }
                Ok(result)
            }
            ClpfdConstraint::Expression { left, op, right } => {
                // Handle arithmetic expression constraints like x == 5, x < y, x + y == z, etc.
                // Pattern match for special arithmetic constraints first
                if matches!(op, CompOp::Equal) {
                    // Check for patterns like: x + y == z, x * y == z, x - y == z
                    match (left, right) {
                        // Pattern: (x + y) == z
                        (ArithExpr::BinaryOp { left: x, op: ArithOp::Add, right: y }, z) => {
                            let x_term = self.eval_arith_expr_with_context(x, external_binder, fresh_context)?;
                            let y_term = self.eval_arith_expr_with_context(y, external_binder, fresh_context)?;
                            let z_term = self.eval_arith_expr_with_context(z, external_binder, fresh_context)?;
                            
                            use crate::relation::clpfd::plusfd::plusfd;
                            Ok(plusfd(x_term, y_term, z_term).cast_into())
                        }
                        // Pattern: z == (x + y)
                        (z, ArithExpr::BinaryOp { left: x, op: ArithOp::Add, right: y }) => {
                            let x_term = self.eval_arith_expr_with_context(x, external_binder, fresh_context)?;
                            let y_term = self.eval_arith_expr_with_context(y, external_binder, fresh_context)?;
                            let z_term = self.eval_arith_expr_with_context(z, external_binder, fresh_context)?;
                            
                            use crate::relation::clpfd::plusfd::plusfd;
                            Ok(plusfd(x_term, y_term, z_term).cast_into())
                        }
                        // Pattern: (x * y) == z
                        (ArithExpr::BinaryOp { left: x, op: ArithOp::Multiply, right: y }, z) => {
                            let x_term = self.eval_arith_expr_with_context(x, external_binder, fresh_context)?;
                            let y_term = self.eval_arith_expr_with_context(y, external_binder, fresh_context)?;
                            let z_term = self.eval_arith_expr_with_context(z, external_binder, fresh_context)?;
                            
                            use crate::relation::clpfd::timesfd::timesfd;
                            Ok(timesfd(x_term, y_term, z_term).cast_into())
                        }
                        // Pattern: z == (x * y)
                        (z, ArithExpr::BinaryOp { left: x, op: ArithOp::Multiply, right: y }) => {
                            let x_term = self.eval_arith_expr_with_context(x, external_binder, fresh_context)?;
                            let y_term = self.eval_arith_expr_with_context(y, external_binder, fresh_context)?;
                            let z_term = self.eval_arith_expr_with_context(z, external_binder, fresh_context)?;
                            
                            use crate::relation::clpfd::timesfd::timesfd;
                            Ok(timesfd(x_term, y_term, z_term).cast_into())
                        }
                        // Pattern: (x - y) == z  =>  x == y + z (transform subtraction to addition)
                        (ArithExpr::BinaryOp { left: x, op: ArithOp::Subtract, right: y }, z) => {
                            let x_term = self.eval_arith_expr_with_context(x, external_binder, fresh_context)?;
                            let y_term = self.eval_arith_expr_with_context(y, external_binder, fresh_context)?;
                            let z_term = self.eval_arith_expr_with_context(z, external_binder, fresh_context)?;
                            
                            // x - y == z  is equivalent to  y + z == x
                            use crate::relation::clpfd::plusfd::plusfd;
                            Ok(plusfd(y_term, z_term, x_term).cast_into())
                        }
                        // Pattern: z == (x - y)  =>  y + z == x
                        (z, ArithExpr::BinaryOp { left: x, op: ArithOp::Subtract, right: y }) => {
                            let x_term = self.eval_arith_expr_with_context(x, external_binder, fresh_context)?;
                            let y_term = self.eval_arith_expr_with_context(y, external_binder, fresh_context)?;
                            let z_term = self.eval_arith_expr_with_context(z, external_binder, fresh_context)?;
                            
                            // z == x - y  is equivalent to  y + z == x
                            use crate::relation::clpfd::plusfd::plusfd;
                            Ok(plusfd(y_term, z_term, x_term).cast_into())
                        }
                        // General case: evaluate both sides and compare
                        _ => {
                            let left_term = self.eval_arith_expr_with_context(left, external_binder, fresh_context)?;
                            let right_term = self.eval_arith_expr_with_context(right, external_binder, fresh_context)?;
                            Ok(crate::relation::eq::eq(left_term, right_term).cast_into())
                        }
                    }
                } else {
                    // Non-equality operations - evaluate both sides and apply the comparison
                    let left_term = self.eval_arith_expr_with_context(left, external_binder, fresh_context)?;
                    let right_term = self.eval_arith_expr_with_context(right, external_binder, fresh_context)?;
                    
                    // Build the main comparison goal
                    let comparison_goal = match op {
                        CompOp::NotEqual => {
                            crate::relation::diseq::diseq(left_term, right_term).cast_into()
                        }
                        CompOp::LessThan => {
                            use crate::relation::clpfd::ltfd::ltfd;
                            ltfd(left_term, right_term).cast_into()
                        }
                        CompOp::LessEqual => {
                            use crate::relation::clpfd::ltefd::ltefd;
                            ltefd(left_term, right_term).cast_into()
                        }
                        CompOp::GreaterThan => {
                            use crate::relation::clpfd::ltfd::ltfd;
                            // x > y is equivalent to y < x
                            ltfd(right_term, left_term).cast_into()
                        }
                        CompOp::GreaterEqual => {
                            use crate::relation::clpfd::ltefd::ltefd;
                            // x >= y is equivalent to y <= x
                            ltefd(right_term, left_term).cast_into()
                        }
                        CompOp::Equal => unreachable!(), // Already handled above
                    };
                    
                    Ok(comparison_goal)
                }
            }
            _ => Err(InterpreterError::RuntimeError(
                "This constraint type is not yet implemented in IR mode".to_string(),
            )),
        }
    }

    /// Convert constraint to goal using pre-resolved variables (new API)
    fn convert_to_goal_with_resolved_vars(
        &self,
        resolved_variables: std::collections::HashMap<String, super::ResolvedValue>,
        _variables: &std::collections::HashMap<String, super::VariableInfo>,
    ) -> Result<Goal, InterpreterError> {
        match self {
            ClpfdConstraint::Domain {
                variable,
                domain_spec,
            } => {
                // Get pre-resolved variable value
                let resolved_value = resolved_variables
                    .get(variable)
                    .ok_or_else(|| InterpreterError::UnknownVariable(variable.clone()))?;

                // Extract LTerm based on resolved value type
                let var_term = match resolved_value {
                    super::ResolvedValue::Relational(lterm) => lterm.clone(),
                    super::ResolvedValue::Meta(_) => {
                        return Err(InterpreterError::RuntimeError(
                            "Meta variables not yet supported in domain constraints".to_string(),
                        ));
                    }
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
                    _ => Err(InterpreterError::RuntimeError(
                        "Interpolated domain specs not yet supported with resolved variables"
                            .to_string(),
                    )),
                }
            }
            ClpfdConstraint::Expression { left, op, right } => {
                // Handle arithmetic expressions specially for constraint programming
                match (left, op, right) {
                    // Handle binary arithmetic operations like x + y == z or x - y != w
                    (ArithExpr::BinaryOp { left: arith_left, op: arith_op, right: arith_right }, comp_op, right_expr) => {
                        // Evaluate the variables in the arithmetic expression
                        let left_term = self.eval_arith_expr_with_resolved_vars(arith_left, &resolved_variables)?;
                        let right_term = self.eval_arith_expr_with_resolved_vars(arith_right, &resolved_variables)?;
                        let result_term = self.eval_arith_expr_with_resolved_vars(right_expr, &resolved_variables)?;
                        
                        // Create the appropriate constraint goal based on the arithmetic operation
                        let arith_goal = match arith_op {
                            ArithOp::Add => {
                                use crate::relation::clpfd::plusfd::plusfd;
                                plusfd(left_term.clone(), right_term.clone(), result_term.clone()).cast_into()
                            }
                            ArithOp::Subtract => {
                                use crate::relation::clpfd::minusfd::minusfd;
                                minusfd(left_term.clone(), right_term.clone(), result_term.clone()).cast_into()
                            }
                            ArithOp::Multiply => {
                                use crate::relation::clpfd::timesfd::timesfd;
                                timesfd(left_term.clone(), right_term.clone(), result_term.clone()).cast_into()
                            }
                            ArithOp::Divide => {
                                // Division in CLP(FD) is more complex, fall back to general case
                                return Err(InterpreterError::RuntimeError(
                                    "Division constraints not yet supported".to_string()
                                ));
                            }
                        };

                        // For equality, just return the arithmetic constraint
                        // For inequality, we need to negate the result
                        match comp_op {
                            CompOp::Equal => Ok(arith_goal),
                            CompOp::NotEqual => {
                                use crate::operator::fresh::Fresh;
                                use crate::operator::conj::Conj;
                                use crate::relation::clpfd::diseqfd::diseqfd;
                                
                                // For x - y != z, create fresh variable for arithmetic result
                                // then constrain: arith_result == (x op y) AND arith_result != z
                                let arith_result = LTerm::var("_arith_temp");
                                let arith_constraint = match arith_op {
                                    ArithOp::Add => {
                                        use crate::relation::clpfd::plusfd::plusfd;
                                        plusfd(left_term.clone(), right_term.clone(), arith_result.clone()).cast_into()
                                    }
                                    ArithOp::Subtract => {
                                        use crate::relation::clpfd::minusfd::minusfd;
                                        minusfd(left_term.clone(), right_term.clone(), arith_result.clone()).cast_into()
                                    }
                                    ArithOp::Multiply => {
                                        use crate::relation::clpfd::timesfd::timesfd;
                                        timesfd(left_term.clone(), right_term.clone(), arith_result.clone()).cast_into()
                                    }
                                    ArithOp::Divide => unreachable!(), // Already handled above
                                };
                                
                                // Add disequality constraint: arith_result != result_term
                                let inequality = diseqfd(arith_result.clone(), result_term).cast_into();
                                
                                // Combine arithmetic constraint with inequality
                                let combined = Conj::new(arith_constraint, inequality).cast_into();
                                
                                // Wrap in Fresh to scope the temporary variable
                                Ok(Fresh::new(vec![arith_result], combined).cast_into())
                            }
                            _ => {
                                // For other comparison operations (LT, GT, etc.), fall back to general case
                                return Err(InterpreterError::RuntimeError(
                                    format!("Arithmetic expressions with {:?} not yet supported", comp_op)
                                ));
                            }
                        }
                    }
                    // Handle general case: evaluate both sides and compare
                    _ => {
                        let left_term =
                            self.eval_arith_expr_with_resolved_vars(left, &resolved_variables)?;
                        let right_term =
                            self.eval_arith_expr_with_resolved_vars(right, &resolved_variables)?;

                        // Build the appropriate comparison goal
                        match op {
                            CompOp::Equal => {
                                use crate::relation::eq::eq;
                                Ok(eq(left_term, right_term).cast_into())
                            }
                            CompOp::NotEqual => {
                                use crate::relation::clpfd::diseqfd::diseqfd;
                                Ok(diseqfd(left_term, right_term).cast_into())
                            }
                            CompOp::LessThan => {
                                use crate::relation::clpfd::ltfd::ltfd;
                                Ok(ltfd(left_term, right_term).cast_into())
                            }
                            CompOp::LessEqual => {
                                use crate::relation::clpfd::ltefd::ltefd;
                                Ok(ltefd(left_term, right_term).cast_into())
                            }
                            CompOp::GreaterThan => {
                                use crate::relation::clpfd::ltfd::ltfd;
                                Ok(ltfd(right_term, left_term).cast_into())
                            }
                            CompOp::GreaterEqual => {
                                use crate::relation::clpfd::ltefd::ltefd;
                                Ok(ltefd(right_term, left_term).cast_into())
                            }
                        }
                    }
                }
            }
            ClpfdConstraint::Global { name, args } => {
                // Resolve all argument variables to LTerms
                let mut resolved_args = Vec::new();
                for arg_name in args {
                    let resolved_value = resolved_variables
                        .get(arg_name)
                        .ok_or_else(|| InterpreterError::UnknownVariable(arg_name.clone()))?;

                    match resolved_value {
                        super::ResolvedValue::Relational(lterm) => {
                            resolved_args.push(lterm.clone())
                        }
                        super::ResolvedValue::Meta(_) => {
                            return Err(InterpreterError::RuntimeError(
                                "Meta variables not yet supported in global constraints"
                                    .to_string(),
                            ));
                        }
                    }
                }

                // Build the appropriate global constraint goal
                match name.as_str() {
                    "distinct" | "alldiff" => {
                        use crate::relation::clpfd::distinctfd::distinctfd;
                        // Convert vector of LTerms to a list LTerm
                        let mut list_term = LTerm::empty_list();
                        for arg in resolved_args.into_iter().rev() {
                            list_term = LTerm::cons(arg, list_term);
                        }
                        Ok(distinctfd(list_term).cast_into())
                    }
                    _ => Err(InterpreterError::RuntimeError(format!(
                        "Unknown global constraint: {}",
                        name
                    ))),
                }
            }
            ClpfdConstraint::ListDomain {
                variables,
                domain_spec,
            } => {
                // Handle list domain constraints like [x, y, z] in 1..3
                // Start with succeed and chain all variable domain constraints
                let mut result = Goal::succeed();

                // Helper function to create domain goal for a variable
                let create_domain_goal = |var_name: &String| -> Result<Goal, InterpreterError> {
                    // Get pre-resolved variable value
                    let resolved_value = resolved_variables
                        .get(var_name)
                        .ok_or_else(|| InterpreterError::UnknownVariable(var_name.clone()))?;

                    // Extract LTerm based on resolved value type
                    let var_term = match resolved_value {
                        super::ResolvedValue::Relational(lterm) => lterm.clone(),
                        super::ResolvedValue::Meta(_) => {
                            return Err(InterpreterError::RuntimeError(
                                "Meta variables not yet supported in list domain constraints".to_string(),
                            ));
                        }
                    };

                    // Create domain constraint for this variable
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
                        _ => {
                            Err(InterpreterError::RuntimeError(
                                "Interpolated domain specs not yet supported in list domain constraints"
                                    .to_string(),
                            ))
                        }
                    }
                };

                // Build conjunction chain directly
                for var_name in variables {
                    let goal = create_domain_goal(var_name)?;
                    result = Conj::new(result, goal).cast_into();
                }

                Ok(result)
            }
            ClpfdConstraint::Fresh { vars, constraints } => {
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
                    let goal = constraint.convert_to_goal_with_resolved_vars(
                        extended_resolved_vars.clone(),
                        _variables,
                    )?;
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
                        "Meta variables not yet supported in arithmetic expressions".to_string(),
                    )),
                }
            }
            ArithExpr::Interpolation(_) => Err(InterpreterError::RuntimeError(
                "Interpolation expressions not yet supported in resolved variables API".to_string(),
            )),
            ArithExpr::BinaryOp { left, op, right } => {
                // Handle simple constant arithmetic
                match (left.as_ref(), right.as_ref()) {
                    (ArithExpr::Integer(left_val), ArithExpr::Integer(right_val)) => {
                        let result = match op {
                            ArithOp::Add => left_val + right_val,
                            ArithOp::Subtract => left_val - right_val,
                            ArithOp::Multiply => left_val * right_val,
                            ArithOp::Divide => {
                                if *right_val == 0 {
                                    return Err(InterpreterError::RuntimeError("Division by zero".to_string()));
                                }
                                left_val / right_val
                            }
                        };
                        Ok(LTerm::from(result as isize))
                    }
                    // For cases involving variables, this should be handled at the constraint level
                    _ => Err(InterpreterError::RuntimeError(
                        "Arithmetic expressions with variables should be handled as constraints, not direct evaluation".to_string(),
                    ))
                }
            }
        }
    }

    /// Helper method to resolve a variable name to an LTerm using context
    fn resolve_variable_to_lterm(
        &self,
        var_name: &str,
        external_binder: &dyn Fn(&str) -> Option<RuntimeValue>,
        fresh_context: &mut FreshVariableContext,
    ) -> Result<LTerm, InterpreterError> {
        // First check if it's a fresh variable
        if let Some(runtime_value) = fresh_context.resolve_fresh_variable(var_name) {
            match runtime_value {
                RuntimeValue::Relational(lterm) => Ok(lterm),
                RuntimeValue::Meta(meta_value) => {
                    // Convert meta value to LTerm for relational context
                    Ok(meta_value.to_lterm())
                }
            }
        } else {
            // Check external binder
            let runtime_value = external_binder(var_name)
                .ok_or_else(|| InterpreterError::UnknownVariable(var_name.to_string()))?;

            match runtime_value {
                RuntimeValue::Relational(lterm) => Ok(lterm),
                RuntimeValue::Meta(meta_value) => {
                    // Convert meta value to LTerm for relational context
                    Ok(meta_value.to_lterm())
                }
            }
        }
    }

    /// Evaluate domain bound using external binder and fresh context
    fn eval_domain_bound_with_context(
        &self,
        bound: &DomainBound,
        _external_binder: &dyn Fn(&str) -> Option<RuntimeValue>,
        _fresh_context: &mut FreshVariableContext,
    ) -> Result<isize, InterpreterError> {
        match bound {
            DomainBound::Integer(i) => Ok(*i as isize),
            DomainBound::Interpolation(meta_expr) => {
                // Evaluate meta expression
                let meta_bindings = HashMap::new();
                let meta_value = evaluate_meta_expression(meta_expr, &meta_bindings)
                    .map_err(|e| InterpreterError::RuntimeError(format!(
                        "Meta expression evaluation error in domain bound: {}", e
                    )))?;
                
                match meta_value {
                    MetaValue::Integer(i) => Ok(i as isize),
                    _ => Err(InterpreterError::RuntimeError(
                        "Meta expression in domain bound must evaluate to integer".to_string()
                    ))
                }
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
                self.resolve_variable_to_lterm(name, external_binder, fresh_context)
            }
            ArithExpr::Interpolation(meta_expr) => {
                // Evaluate meta expression
                let meta_bindings = HashMap::new();
                let meta_value = evaluate_meta_expression(meta_expr, &meta_bindings)
                    .map_err(|e| InterpreterError::RuntimeError(format!(
                        "Meta expression evaluation error in CLPFD constraint: {}", e
                    )))?;
                
                Ok(meta_value.to_lterm())
            }
            ArithExpr::BinaryOp { left, op, right } => {
                // For CLP(FD), we still need to evaluate sub-expressions to create the proper terms
                // Even though constraint solving happens at a higher level
                match (left.as_ref(), right.as_ref()) {
                    // If both are simple terms (constants or variables), we can handle it
                    (ArithExpr::Integer(l_val), ArithExpr::Integer(r_val)) => {
                        // Both constants - we can compute directly for simple cases
                        match op {
                            ArithOp::Add => Ok(LTerm::from((*l_val + *r_val) as isize)),
                            ArithOp::Subtract => Ok(LTerm::from((*l_val - *r_val) as isize)),
                            ArithOp::Multiply => Ok(LTerm::from((*l_val * *r_val) as isize)),
                            _ => Err(InterpreterError::RuntimeError(format!(
                                "Arithmetic operation {:?} not supported", op
                            ))),
                        }
                    }
                    // For mixed variable/constant cases or complex expressions,
                    // the constraint will be posted at the higher level
                    _ => {
                        // Return the sub-expressions directly for constraint posting
                        // The actual constraint handling will be done by the pattern matching
                        // in the Expression constraint processing
                        let left_term = self.eval_arith_expr_with_context(left, external_binder, fresh_context)?;
                        let right_term = self.eval_arith_expr_with_context(right, external_binder, fresh_context)?;
                        
                        // For now, we can't directly compute the result, so we indicate
                        // this needs constraint processing at higher level
                        Err(InterpreterError::RuntimeError(
                            "Complex arithmetic expression requires constraint solving".to_string()
                        ))
                    }
                }
            }
        }
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
                // Only include args that are actual variables (not numeric constants)
                for arg in args {
                    if arg.parse::<i32>().is_err() {
                        vars.push(arg.clone());
                    }
                }
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

/// Evaluates an arithmetic expression with constraint generation for IR mode
fn eval_arith_expr_with_constraints_ir(
    expr: &ArithExpr,
    execution_context: &mut ExecutionContext,
    constraint_goals: &mut Vec<Goal>,
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
            // Handle meta expression interpolation
            match meta_expr {
                crate::interpreter::metaprogramming::MetaExpression::Variable(var_name, _) => {
                    let symbol =
                        crate::interpreter::symbol_table::InternedSymbol::from(var_name.clone());
                    if let Some(var_value) = execution_context.lookup_variable_value(&symbol) {
                        if let Some(meta_value) = var_value.as_meta() {
                            match meta_value {
                                crate::interpreter::compiler::ir::MetaValue::Integer(i) => {
                                    Ok(LTerm::from(*i as isize))
                                }
                                _ => Err(InterpreterError::RuntimeError(format!(
                                    "Interpolation variable '{}' must be an integer for arithmetic expressions",
                                    var_name
                                ))),
                            }
                        } else {
                            execution_context
                                .lookup_var(&symbol)
                                .ok_or_else(|| InterpreterError::UnknownVariable(var_name.clone()))
                        }
                    } else {
                        Err(InterpreterError::RuntimeError(format!(
                            "Interpolation variable '{}' not found in constraint context",
                            var_name
                        )))
                    }
                }
                _ => Err(InterpreterError::RuntimeError(
                    "Complex interpolation expressions not yet supported in IR mode".to_string(),
                )),
            }
        }
        ArithExpr::BinaryOp { left, op, right } => {
            // Recursively evaluate left and right operands
            let left_term = eval_arith_expr_with_constraints_ir(left, execution_context, constraint_goals)?;
            let right_term = eval_arith_expr_with_constraints_ir(right, execution_context, constraint_goals)?;
            
            // Create fresh variable for the result
            let result_var = execution_context.create_fresh_var();
            
            // Create the appropriate arithmetic constraint goal
            let constraint_goal = match op {
                ArithOp::Add => {
                    use crate::relation::clpfd::plusfd::plusfd;
                    // result = left + right, so plusfd(left, right, result)
                    plusfd(left_term, right_term, result_var.clone()).cast_into()
                }
                ArithOp::Subtract => {
                    use crate::relation::clpfd::plusfd::plusfd;
                    // result = left - right, equivalent to left = result + right
                    // So plusfd(result, right, left)
                    plusfd(result_var.clone(), right_term, left_term).cast_into()
                }
                ArithOp::Multiply => {
                    use crate::relation::clpfd::timesfd::timesfd;
                    // result = left * right, so timesfd(left, right, result)
                    timesfd(left_term, right_term, result_var.clone()).cast_into()
                }
                ArithOp::Divide => {
                    // Division is more complex and less commonly supported in CLPFD
                    // For now, return an error
                    return Err(InterpreterError::RuntimeError(
                        "Division not yet supported in CLPFD constraints".to_string(),
                    ));
                }
            };
            
            constraint_goals.push(constraint_goal);
            Ok(result_var)
        }
    }
}

/// Evaluates an arithmetic expression for IR mode, creating temporary variables for intermediate results.
fn eval_arith_expr_ir(
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
                    if let Some(var_value) = execution_context.lookup_variable_value(&symbol) {
                        if let Some(meta_value) = var_value.as_meta() {
                            match meta_value {
                                crate::interpreter::compiler::ir::MetaValue::Integer(i) => {
                                    Ok(LTerm::from(*i as isize))
                                }
                                _ => Err(InterpreterError::RuntimeError(format!(
                                    "Interpolation variable '{}' must be an integer for arithmetic expressions",
                                    var_name
                                ))),
                            }
                        } else {
                            // It's a relational variable, return it directly
                            execution_context
                                .lookup_var(&symbol)
                                .ok_or_else(|| InterpreterError::UnknownVariable(var_name.clone()))
                        }
                    } else {
                        Err(InterpreterError::RuntimeError(format!(
                            "Interpolation variable '{}' not found in constraint context",
                            var_name
                        )))
                    }
                }
                _ => Err(InterpreterError::RuntimeError(
                    "Complex interpolation expressions not yet supported in IR mode".to_string(),
                )),
            }
        }
        ArithExpr::BinaryOp { left, op, right } => {
            // For arithmetic expressions in constraints, we need to return a fresh variable
            // The actual constraint will be posted by the containing expression constraint
            let result_var = execution_context.create_fresh_var();
            Ok(result_var)
        }
    }
}

/// Evaluates an arithmetic expression, creating temporary variables for intermediate results.
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
                            "Meta expression expansion error in arithmetic: {}",
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
                    use crate::relation::clpfd::plusfd::plusfd;
                    plusfd::<Goal>(left_term, right_term, result_term.clone()).cast_into()
                }
                ArithOp::Subtract => {
                    use crate::relation::clpfd::minusfd::minusfd;
                    minusfd::<Goal>(left_term, right_term, result_term.clone()).cast_into()
                }
                ArithOp::Multiply => {
                    use crate::relation::clpfd::timesfd::timesfd;
                    timesfd::<Goal>(left_term, right_term, result_term.clone()).cast_into()
                }
                ArithOp::Divide => {
                    // Note: CLP(FD) division is often not a primitive.
                    // This will likely need a more complex implementation or might not be fully supported.
                    // For now, we create a multiplication goal: result * right == left
                    use crate::relation::clpfd::timesfd::timesfd;
                    timesfd::<Goal>(result_term.clone(), right_term, left_term).cast_into()
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

fn eval_domain_bound(
    bound: &DomainBound,
    execution_context: &mut ExecutionContext,
    _source_span: &super::super::parser::ast::Location,
) -> Result<isize, InterpreterError> {
    match bound {
        DomainBound::Integer(val) => Ok(*val as isize),
        DomainBound::Interpolation(meta_expr) => {
            // For simple variable interpolations, directly access the execution context
            match meta_expr {
                crate::interpreter::metaprogramming::MetaExpression::Variable(var_name, _) => {
                    // Directly look up the variable in the execution context
                    let symbol =
                        crate::interpreter::symbol_table::InternedSymbol::from(var_name.clone());
                    let var_term = execution_context.lookup_var(&symbol).ok_or_else(|| {
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
                    let _expanded_term = expand_term(&dummy_term, &context).map_err(|e| {
                        InterpreterError::RuntimeError(format!(
                            "Meta expression expansion error: {}",
                            e
                        ))
                    })?;

                    // Complex metaprogramming expansion should be handled in IR template system
                    Err(InterpreterError::RuntimeError(
                        "Complex metaprogramming interpolation in domain bounds not yet supported in IR mode".to_string()
                    ))
                }
            }
        }
    }
}

fn build_comparison_goal(left: LTerm, op: CompOp, right: LTerm) -> Result<Goal, InterpreterError> {
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

fn try_build_arithmetic_constraint(
    left: &ArithExpr,
    right: &ArithExpr,
    execution_context: &mut ExecutionContext,
) -> Result<Option<Goal>, InterpreterError> {
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
