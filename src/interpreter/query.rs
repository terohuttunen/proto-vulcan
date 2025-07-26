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
) -> Result<Vec<QueryResult>, InterpreterError> {
    use super::parser::ast;
    use super::compiler::Compiler;
    use super::runtime::context::{PredicateClosure, ArgumentValue};
    use crate::interpreter::symbol_table::InternedSymbol;
    
    // Step 1: Extract variables from the query for later result collection
    let query_vars = extract_variables_from_goal(&query);
    
    // Step 2: Create a temporary AST program with the query as a predicate body
    // Note: We need to import the existing program so the query can reference its predicates
    let query_predicate_name = InternedSymbol::from("__query__".to_string());
    let query_parameters: Vec<ast::Parameter> = query_vars.iter()
        .map(|var_name| ast::Parameter {
            name: InternedSymbol::from(var_name.clone()),
            type_annotation: None,
        })
        .collect();
    
    let query_predicate = ast::PredicateDefinition {
        visibility: ast::Visibility::Private,
        predicate_kind: ast::PredicateKind::Relation,
        attributes: vec![],
        name: query_predicate_name.clone(),
        parameters: query_parameters,
        search_strategy: None,
        body: vec![query.clone()],
        span: ast::Location::dummy(),
    };
    
    // Step 3: Add the query directly to the existing IR program without AST conversion
    let query_ir_program = Compiler::add_query_to_program(
        (*ir_program).clone(), // Clone the base program  
        query.clone()
    ).map_err(|e| {
        InterpreterError::RuntimeError(format!("Query compilation failed: {:?}", e))
    })?;
    
    // Step 4: Extract the query predicate from the compiled IR program
    let query_predicate_id = super::compiler::ir::PredicateId::new("::__query__");
    let query_ir_predicate = query_ir_program.registry.get_predicate(&query_predicate_id)
        .ok_or_else(|| InterpreterError::RuntimeError("Failed to find compiled query predicate".to_string()))?
        .clone();
    
    // Step 5: Create argument values for the query variables (all relational, no meta)
    let mut execution_context = ExecutionContext::new(Rc::new(query_ir_program), environment.clone());
    
    // Provide base program access for predicate resolution during closure execution
    execution_context.set_base_program(ir_program.clone());
    let captured_args: Vec<ArgumentValue> = query_vars.iter()
        .map(|var_name| {
            let fresh_var = execution_context.create_fresh_var();
            let symbol = InternedSymbol::from(var_name.clone());
            execution_context.bind_var(symbol, fresh_var.clone());
            ArgumentValue::Relational(fresh_var)
        })
        .collect();
    
    // Step 6: Create predicate closure for the query
    // Use the original IR program as context for predicate resolution
    let query_closure = PredicateClosure::new(
        Rc::new(query_ir_predicate),
        captured_args,
        ir_program.clone(),
        environment,
    );
    
    // Step 7: Create solver and execute the query closure
    let user_state = DefaultUser::default();
    let user_globals = <DefaultUser as User>::UserContext::default();
    let mut solver = crate::solver::Solver::new(user_globals, false);
    
    // Set timeout if provided
    if let Some(timeout_ms) = config.timeout {
        let start_time = std::time::Instant::now();
        solver.set_timeout(start_time, timeout_ms);
    }
    
    // Set the IR program in the solver for deferred relation calls
    solver.set_program(ir_program);
    
    let initial_state = crate::state::State::new(user_state);
    
    // Get variable bindings for reification
    let variable_bindings = execution_context.get_variable_bindings();
    
    // Execute the query closure with reification (like macro does)
    // First get the original query stream
    let original_stream = query_closure.expand_and_solve(&solver, initial_state);
    
    // For now, we'll implement a simpler approach by adding reification to each solution
    // rather than trying to modify the goal structure before execution
    // TODO: This can be optimized later by creating a compound goal
    let mut results = Vec::new();
    let mut stream = original_stream;
    
    // Collect up to 100 results (to prevent infinite loops)
    let max_results = 100;
    let mut result_count = 0;
    
    while result_count < max_results {
        match solver.next(&mut stream) {
            crate::solver::SolverResult::Solution(state_box) => {
                let state = &*state_box;
                
                // Apply reification to each query variable (like macro does)
                use crate::state::reify;
                use crate::goal::AnyGoal;
                
                for (var_name, var_term) in &variable_bindings {
                    // Create reification goal for this variable
                    let reify_goal = reify(var_term.clone());
                    let reified_stream = reify_goal.solve(&solver, (*state_box).clone());
                    
                    // Process reified solutions for this variable
                    let mut reified_stream = reified_stream;
                    
                    // Collect reified solutions for this variable
                    while let Some(reified_state_box) = {
                        match solver.next(&mut reified_stream) {
                            crate::solver::SolverResult::Solution(s) => Some(s),
                            _ => None,
                        }
                    } {
                        let reified_state = &*reified_state_box;
                        let reified_smap = reified_state.smap_ref();
                        let reified_resolved_term = reified_smap.walk_star(var_term);
                        let reified_purified_cstore = reified_state.cstore_ref().clone().purify(reified_smap);
                        let reified_reified_cstore = Rc::new(reified_purified_cstore.walk_star(reified_smap));
                        let reified_result_with_constraints = LResult(reified_resolved_term, Rc::clone(&reified_reified_cstore));
                        
                        // Create a result for each reified solution
                        let mut variable_query_result = QueryResult::new();
                        variable_query_result.bindings.insert(var_name.clone(), reified_result_with_constraints);
                        results.push(variable_query_result);
                        result_count += 1;
                        
                        if result_count >= max_results {
                            break;
                        }
                    }
                    
                    if result_count >= max_results {
                        break;
                    }
                }
            }
            crate::solver::SolverResult::NoMoreSolutions => break,
            crate::solver::SolverResult::Timeout => break,
            crate::solver::SolverResult::Error(_) => break,
        }
    }
    
    Ok(results)
}



/// Extract variable names from a goal AST
fn extract_variables_from_goal(goal: &Goal) -> Vec<String> {
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