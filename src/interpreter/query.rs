use super::environment::Environment;
use super::execution::ExecutionContext;
use super::parser::ast::Goal;
use super::InterpreterError;
use crate::engine::Engine;
use crate::lresult::LResult;
use crate::lterm::LTerm;
// Removed unused imports for cleaner code
use crate::user::User;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Query result containing variable bindings
#[derive(Debug, Clone)]
pub struct QueryResult<U: User, E: Engine<U>> {
    pub bindings: HashMap<String, LResult<U, E>>,
}

impl<U: User, E: Engine<U>> QueryResult<U, E> {
    /// Create a new empty query result
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Add a variable binding
    pub fn bind(&mut self, var_name: String, value: LResult<U, E>) {
        self.bindings.insert(var_name, value);
    }

    /// Get a variable binding
    pub fn get(&self, var_name: &str) -> Option<&LResult<U, E>> {
        self.bindings.get(var_name)
    }

    /// Check if the result has any bindings
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Create a new any LResult
    fn any() -> LResult<U, E> {
        LResult(
            LTerm::any(),
            Rc::new(crate::state::constraint::store::ConstraintStore::new()),
        )
    }

    /// Create from LResult vector (for core query result compatibility)
    pub fn from_lresults(results: Vec<LResult<U, E>>) -> Self {
        let mut bindings = HashMap::new();
        for (i, result) in results.into_iter().enumerate() {
            bindings.insert(format!("_{}", i), result);
        }
        Self { bindings }
    }
}

/// Execute a query against the environment
pub fn execute_query<U: User, E: Engine<U>>(
    environment: Rc<RefCell<Environment<U, E>>>,
    query: Goal,
) -> Result<Vec<QueryResult<U, E>>, InterpreterError>
where
    U::UserContext: Default,
{
    // Pre-populate the execution context with variables from the query
    let mut execution_context = ExecutionContext::new(environment);
    let query_vars = extract_variables_from_goal(&query);
    for var_name in &query_vars {
        let fresh_var = execution_context.create_fresh_var();
        execution_context.bind_var(var_name.clone(), fresh_var);
    }

    // Convert the AST query to a runtime goal
    let runtime_goal = execution_context.ast_goal_to_runtime(&query)?;

    // Get the actual variable bindings used during goal conversion
    let variable_bindings = execution_context.get_variable_bindings();

    // Add reification goals for each user-visible variable.  Reifying each variable
    // individually mirrors the code emitted by the procedural macros and guarantees
    // that every binding is walked and materialised in the final substitution map.
    use crate::goal::{AnyGoal, GoalCast};
    use crate::operator::conj::InferredConj;
    use crate::state::reify;

    let mut goals = vec![runtime_goal.clone()];

    for var_term in variable_bindings.values() {
        goals.push(reify(var_term.clone()).cast_into());
    }

    let mut reified_goal = AnyGoal::<U, E>::succeed();
    for goal in goals.into_iter().rev() {
        reified_goal = InferredConj::new(goal, reified_goal).cast_into();
    }

    // Create solver and initial state
    let user_state = U::default();
    let user_globals = U::UserContext::default();
    let solver = crate::solver::Solver::new(user_globals, false);
    let initial_state = crate::state::State::new(user_state);

    // Execute the goal and collect results
    let stream = solver.start(&reified_goal, initial_state);
    let mut results = Vec::new();

    // Create a mutable stream to iterate through
    let mut stream = stream;
    let mut solver = solver;

    // Collect up to 100 results (to prevent infinite loops)
    let max_results = 100;
    let mut result_count = 0;

    while result_count < max_results {
        match solver.next(&mut stream) {
            Some(state_box) => {
                let state = &*state_box;
                let mut query_result = QueryResult::new();

                // Finalize the state by processing the constraint store, same as in ResultIterator
                let smap = state.smap_ref();
                let purified_cstore = state.cstore_ref().clone().purify(smap);
                let reified_cstore = Rc::new(purified_cstore.walk_star(smap));

                // The key fix: use the reified substitution map to get resolved values
                // After reification, the substitution map contains the resolved values
                for (var_name, var_term) in &variable_bindings {
                    // Walk the variable term to get its value in the reified state
                    let resolved_term = smap.walk_star(var_term);
                    let result_with_constraints =
                        LResult(resolved_term, Rc::clone(&reified_cstore));
                    query_result.bind(var_name.clone(), result_with_constraints);
                }

                results.push(query_result);
                result_count += 1;
            }
            None => break,
        }
    }

    Ok(results)
}

/// Execute a query against the environment with tracing enabled
pub fn execute_query_with_trace<U: User, E: Engine<U>>(
    environment: Rc<RefCell<Environment<U, E>>>,
    query: Goal,
    trace_config: &mut super::trace::TraceConfig,
) -> Result<Vec<QueryResult<U, E>>, InterpreterError>
where
    U::UserContext: Default,
{
    use super::parser::ast::Goal;

    // Start tracing
    if trace_config.enabled {
        let goal_name = match &query {
            Goal::RelationCall(call, _) => call.name.clone(),
            Goal::Equality(_, _, _) => "equality".to_string(),
            Goal::Disequality(_, _, _) => "disequality".to_string(),
            _ => "query".to_string(),
        };
        trace_config.enter_relation(&goal_name);
    }

    // Pre-populate the execution context with variables from the query
    let mut execution_context = ExecutionContext::new(environment);
    let query_vars = extract_variables_from_goal(&query);
    for var_name in &query_vars {
        let fresh_var = execution_context.create_fresh_var();
        execution_context.bind_var(var_name.clone(), fresh_var);
    }

    // Convert the AST query to a runtime goal
    let runtime_goal = execution_context.ast_goal_to_runtime(&query)?;

    // Get the actual variable bindings used during goal conversion
    let variable_bindings = execution_context.get_variable_bindings();

    // Add reification goals for each user-visible variable
    use crate::goal::{AnyGoal, GoalCast};
    use crate::operator::conj::InferredConj;
    use crate::state::reify;

    let mut goals = vec![runtime_goal.clone()];

    for var_term in variable_bindings.values() {
        goals.push(reify(var_term.clone()).cast_into());
    }

    let mut reified_goal = AnyGoal::<U, E>::succeed();
    for goal in goals.into_iter().rev() {
        reified_goal = InferredConj::new(goal, reified_goal).cast_into();
    }

    // Create solver and initial state
    let user_state = U::default();
    let user_globals = U::UserContext::default();
    let solver = crate::solver::Solver::new(user_globals, false);
    let initial_state = crate::state::State::new(user_state);

    // Execute the goal and collect results with tracing
    let stream = solver.start(&reified_goal, initial_state);
    let mut results = Vec::new();

    // Create a mutable stream to iterate through
    let mut stream = stream;
    let mut solver = solver;

    // Collect up to 100 results (to prevent infinite loops)
    let max_results = 100;
    let mut result_count = 0;

    while result_count < max_results {
        match solver.next(&mut stream) {
            Some(state_box) => {
                let state = &*state_box;
                let mut query_result = QueryResult::new();

                // Finalize the state by processing the constraint store
                let smap = state.smap_ref();
                let purified_cstore = state.cstore_ref().clone().purify(smap);
                let reified_cstore = Rc::new(purified_cstore.walk_star(smap));

                // Convert bindings for tracing
                let mut trace_bindings = Vec::new();
                for (var_name, var_term) in &variable_bindings {
                    // Walk the variable term to get its value in the reified state
                    let resolved_term = smap.walk_star(var_term);
                    let result_with_constraints =
                        LResult(resolved_term.clone(), Rc::clone(&reified_cstore));
                    query_result.bind(var_name.clone(), result_with_constraints);

                    // Prepare for tracing
                    trace_bindings.push((var_name.clone(), format!("{}", resolved_term)));
                }

                // Trace the solution
                if trace_config.enabled {
                    trace_config.trace_solution(&trace_bindings);
                }

                results.push(query_result);
                result_count += 1;
            }
            None => break,
        }
    }

    // End tracing
    if trace_config.enabled {
        trace_config.exit_relation(
            &match &query {
                Goal::RelationCall(call, _) => call.name.clone(),
                Goal::Equality(_, _, _) => "equality".to_string(),
                Goal::Disequality(_, _, _) => "disequality".to_string(),
                _ => "query".to_string(),
            },
            true,
        );
        trace_config.print_summary();
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
    use super::parser::ast::*;

    match goal {
        Goal::Equality(left, right, _) => {
            extract_variables_from_term(left, vars);
            extract_variables_from_term(right, vars);
        }
        Goal::Disequality(left, right, _) => {
            extract_variables_from_term(left, vars);
            extract_variables_from_term(right, vars);
        }
        Goal::Conjunction(conj, _) => {
            for goal in &conj.body {
                extract_variables_from_goal_recursive(goal, vars);
            }
        }
        Goal::Disjunction(disj, _) => {
            for goal in &disj.body {
                extract_variables_from_goal_recursive(goal, vars);
            }
        }
        Goal::Fresh(fresh, _) => {
            // Don't extract fresh variables as they are locally scoped
            for goal in &fresh.body {
                extract_variables_from_goal_recursive(goal, vars);
            }
        }
        Goal::RelationCall(call, _) => {
            for arg in &call.args {
                extract_variables_from_term(arg, vars);
            }
        }
        Goal::MethodCall(call, _) => {
            extract_variables_from_term(&call.receiver, vars);
            for arg in &call.args {
                extract_variables_from_term(arg, vars);
            }
        }
        Goal::Let(let_decl, _) => {
            if let Some(value) = &let_decl.value {
                extract_variables_from_term(value, vars);
            }
        }
        Goal::Parenthesized(body, _) => {
            for goal in body {
                extract_variables_from_goal_recursive(goal, vars);
            }
        }
        Goal::PatternMatch(pattern_match, _) => {
            extract_variables_from_term(&pattern_match.term, vars);
            for arm in &pattern_match.arms {
                for goal in &arm.body {
                    extract_variables_from_goal_recursive(goal, vars);
                }
            }
        }
        Goal::BooleanLiteral(..) => {
            // Boolean literals don't contain variables
        }
        Goal::ConstraintBlock(..) => {
            // TODO: Extract variables from constraint blocks
        }
        Goal::MetaStatement(..) => {
            // TODO: Implement meta statement variable extraction
            // For now, do nothing as meta statements don't introduce variables
        }
    }
}

fn extract_variables_from_term(term: &super::parser::ast::Term, vars: &mut Vec<String>) {
    use super::parser::ast::*;

    match term {
        Term::Variable(var_name, _) => {
            vars.push(var_name.clone());
        }
        Term::Wildcard(_) => {
            // Wildcards don't contain variables to extract
        }
        Term::List(list_construction, _) => {
            for element in &list_construction.elements {
                extract_variables_from_term(element, vars);
            }
            if let Some(tail) = &list_construction.tail {
                extract_variables_from_term(tail, vars);
            }
        }
        Term::NamedStruct(named_struct, _) => {
            for field in &named_struct.fields {
                extract_variables_from_term(&field.value, vars);
            }
        }
        Term::Compound(compound, _) => {
            for arg in &compound.args {
                extract_variables_from_term(arg, vars);
            }
        }
        Term::Literal(..) => {
            // Literals don't contain variables
        }
        Term::Parenthesized(inner, _) => {
            extract_variables_from_term(inner, vars);
        }
        Term::Interpolation(..) => {
            // TODO: Extract variables from meta expressions in interpolations
            // For now, do nothing
        }
    }
}

/// Parse a query string using the existing parser
pub fn parse_query(query_str: &str) -> Result<Goal, InterpreterError> {
    use super::parser::{build_goal, Rule, VulcanParser};
    use pest::Parser;

    let pair = VulcanParser::parse(Rule::goal, query_str)
        .map_err(|e| InterpreterError::ParseError(e.to_string()))?
        .next()
        .ok_or_else(|| InterpreterError::ParseError("Empty query".to_string()))?;

    build_goal(pair).map_err(|e| InterpreterError::ParseError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::DefaultEngine;
    use crate::user::DefaultUser;

    #[test]
    fn test_query_result_creation() {
        let result: QueryResult<DefaultUser, DefaultEngine<DefaultUser>> = QueryResult::new();
        assert!(result.is_empty());
    }

    #[test]
    fn test_query_result_binding() {
        let mut result: QueryResult<DefaultUser, DefaultEngine<DefaultUser>> = QueryResult::new();
        let term = LTerm::from(42);
        let lresult = LResult(
            term.clone(),
            Rc::new(crate::state::constraint::store::ConstraintStore::new()),
        );
        result.bind("x".to_string(), lresult.clone());

        assert!(!result.is_empty());
        assert_eq!(result.get("x"), Some(&lresult));
    }

    #[test]
    fn test_parse_equality_query() {
        let query = parse_query("x == 42").unwrap();
        match query {
            Goal::Equality(left, right, _) => {
                assert!(matches!(
                    left,
                    super::super::parser::ast::Term::Variable(_, _)
                ));
                assert!(matches!(
                    right,
                    super::super::parser::ast::Term::Literal(..)
                ));
            }
            _ => panic!("Expected equality goal"),
        }
    }

    #[test]
    fn test_parse_relation_call_query() {
        let query = parse_query("parent(alice, bob)").unwrap();
        match query {
            Goal::RelationCall(call, _) => {
                assert_eq!(call.name, "parent");
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
