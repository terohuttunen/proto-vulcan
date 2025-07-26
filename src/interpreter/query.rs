//! Query execution for the Proto-Vulcan interpreter
//!
//! This module handles parsing and executing queries in the Proto-Vulcan language.
//! It supports both the new IR-based execution and legacy AST-based execution.

use super::environment::Environment;
use super::InterpreterError;
use super::parser::ast::{Goal, Term};
use super::runtime::context::ExecutionContext;
use crate::lresult::LResult;
use crate::user::{DefaultUser, User};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Configuration for query execution
#[derive(Debug, Clone, Default)]
pub struct QueryConfig {
    /// Timeout in milliseconds
    pub timeout: Option<u64>,
    /// Trace configuration
    pub trace: Option<super::trace::TraceConfig>,
}

/// Result of a single query execution
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub bindings: HashMap<String, LResult>,
}

impl QueryResult {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }
}

/// Parse a query string into an AST goal
pub fn parse_query(query_str: &str) -> Result<Goal, InterpreterError> {
    let stripped = query_str.trim();
    if stripped.is_empty() {
        return Err(InterpreterError::ParseError(
            "Query cannot be empty".to_string(),
        ));
    }

    // For now, parse the query as a simple relation call or goal
    // TODO: Implement proper goal-only parsing
    // As a temporary solution, wrap it in a dummy relation and parse
    let dummy_program = format!("rel __query__() {{ {} }}", stripped);
    let program = super::parser::parse_str(&dummy_program)
        .map_err(|e| InterpreterError::ParseError(e.to_string()))?;
    
    // Extract the goal from the dummy relation
    if let Some(item) = program.items.first() {
        if let super::parser::ast::Item::Predicate(rel) = item {
            if let Some(goal) = rel.body.first() {
                return Ok(goal.clone());
            }
        }
    }
    
    Err(InterpreterError::ParseError("Failed to parse query".to_string()))
}

/// Execute a query using the IR-based execution system with compiler infrastructure
pub fn execute_query_ir(
    ir_program: Rc<super::compiler::ir::Program>,
    environment: Rc<RefCell<Environment>>,
    query: Goal,
    config: QueryConfig,
) -> Result<super::iterator::QueryResultIterator, InterpreterError> {
    // Convert QueryConfig to ExecutionConfig
    let debug_enabled = config.trace.is_some();
    let execution_config = super::ExecutionConfig {
        timeout: config.timeout,
        trace: config.trace,
        debug_enabled,
        ..Default::default()
    };
    
    // Use the new reification-aware iterator constructor
    super::iterator::QueryResultIterator::new_ir_reified(
        ir_program,
        environment,
        query,
        execution_config,
    )
}



/// Extract variable names from a goal AST
pub fn extract_variables_from_goal(goal: &Goal) -> Vec<String> {
    let mut vars = Vec::new();
    extract_variables_from_goal_recursive(goal, &mut vars);
    vars.sort();
    vars.dedup();
    vars
}

fn extract_variables_from_goal_recursive(goal: &Goal, vars: &mut Vec<String>) {
    use crate::interpreter::parser::ast::{Goal as AstGoal};

    match goal {
        AstGoal::Equality(left, right, _) => {
            extract_variables_from_term(left, vars);
            extract_variables_from_term(right, vars);
        }
        AstGoal::Disequality(left, right, _) => {
            extract_variables_from_term(left, vars);
            extract_variables_from_term(right, vars);
        }
        AstGoal::RelationCall(rel_call, _) => {
            for arg in &rel_call.args {
                match arg {
                    super::parser::ast::CallArgument::Term(term) => extract_variables_from_term(term, vars),
                    super::parser::ast::CallArgument::MetaExpression(_) => {}, // Skip meta expressions
                }
            }
        }
        AstGoal::Conjunction(goals, _) => {
            for goal in &goals.body {
                extract_variables_from_goal_recursive(goal, vars);
            }
        }
        AstGoal::Disjunction(goals, _) => {
            for goal in &goals.body {
                extract_variables_from_goal_recursive(goal, vars);
            }
        }
        AstGoal::Fresh(fresh_goal, _) => {
            for goal in &fresh_goal.body {
                extract_variables_from_goal_recursive(goal, vars);
            }
        }
        AstGoal::ConstraintBlock(_cb, _) => {
            // ConstraintBlocks have raw content, not parsed goals, so no variables to extract directly
        }
        AstGoal::PatternMatch(match_goal, _) => {
            extract_variables_from_term(&match_goal.term, vars);
            for clause in &match_goal.arms {
                for goal in &clause.body {
                    extract_variables_from_goal_recursive(goal, vars);
                }
            }
        }
        _ => {} // Other goal types don't contribute variables
    }
}

fn extract_variables_from_term(term: &Term, vars: &mut Vec<String>) {
    use crate::interpreter::parser::ast::Term;

    match term {
        Term::Variable(name) => {
            vars.push(name.to_string());
        }
        Term::NamedStruct(named_struct, _) => {
            for field in &named_struct.fields {
                extract_variables_from_term(&field.value, vars);
            }
        }
        Term::TupleStruct(tuple_struct, _) => {
            for field in &tuple_struct.args {
                extract_variables_from_term(field, vars);
            }
        }
        Term::EnumVariant(enum_variant, _) => {
            match &enum_variant.kind {
                super::parser::ast::EnumVariantConstructionKind::Unit => {},
                super::parser::ast::EnumVariantConstructionKind::Tuple(fields) => {
                    for field in fields {
                        extract_variables_from_term(field, vars);
                    }
                },
                super::parser::ast::EnumVariantConstructionKind::Named(fields) => {
                    for field in fields {
                        extract_variables_from_term(&field.value, vars);
                    }
                },
            }
        }
        Term::List(list_term, _) => {
            for element in &list_term.elements {
                extract_variables_from_term(element, vars);
            }
            if let Some(tail) = &list_term.tail {
                extract_variables_from_term(tail, vars);
            }
        }
        Term::Parenthesized(inner, _) => {
            extract_variables_from_term(inner, vars);
        }
        // Other term types (integers, strings, etc.) don't contain variables
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_query() {
        let query = parse_query("X == 5").unwrap();
        match query {
            Goal::Equality(_, _, _) => {} // Expected
            _ => panic!("Expected unification goal"),
        }
    }

    #[test]
    fn test_parse_relation_call_query() {
        let query = parse_query("parent(alice, bob)").unwrap();
        match query {
            Goal::RelationCall(call, _) => {
                assert_eq!(call.name.name(), "parent");
                assert_eq!(call.args.len(), 2);
            }
            _ => panic!("Expected relation call goal"),
        }
    }

    #[test]
    fn test_parse_invalid_query() {
        let result = parse_query("invalid syntax here");
        assert!(result.is_err());
    }
}