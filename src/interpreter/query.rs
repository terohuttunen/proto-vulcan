use super::environment::Environment;
use super::execution::ExecutionContext;
use super::parser::ast::Goal;
use super::InterpreterError;
use crate::lresult::LResult;
use crate::lterm::LTerm;
// Removed unused imports for cleaner code
use crate::user::{DefaultUser, User};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Query execution configuration
#[derive(Debug, Clone)]
pub struct QueryConfig {
    /// Optional timeout in milliseconds
    pub timeout: Option<u64>,
    /// Optional trace configuration
    pub trace: Option<super::trace::TraceConfig>,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            timeout: None,
            trace: None,
        }
    }
}

impl QueryConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout = Some(timeout_ms);
        self
    }

    pub fn with_trace(mut self, trace_config: super::trace::TraceConfig) -> Self {
        self.trace = Some(trace_config);
        self
    }
}

/// Query result containing variable bindings
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub bindings: HashMap<String, LResult>,
}

impl QueryResult {
    /// Create a new empty query result
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Add a variable binding
    pub fn bind(&mut self, var_name: String, value: LResult) {
        self.bindings.insert(var_name, value);
    }

    /// Get a variable binding
    pub fn get(&self, var_name: &str) -> Option<&LResult> {
        self.bindings.get(var_name)
    }

    /// Check if the result has any bindings
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Create a new any LResult
    fn any() -> LResult {
        LResult(
            LTerm::any(),
            Rc::new(crate::state::constraint::store::ConstraintStore::new()),
        )
    }

    /// Create from LResult vector (for core query result compatibility)
    pub fn from_lresults(results: Vec<LResult>) -> Self {
        let mut bindings = HashMap::new();
        for (i, result) in results.into_iter().enumerate() {
            bindings.insert(format!("_{}", i), result);
        }
        Self { bindings }
    }
}

/// Execute a query against the environment with configuration
pub fn execute_query(
    environment: Rc<RefCell<Environment>>,
    query: Goal,
    config: QueryConfig,
) -> Result<Vec<QueryResult>, InterpreterError> {
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

    let mut reified_goal = AnyGoal::succeed();
    for goal in goals.into_iter().rev() {
        reified_goal = InferredConj::new(goal, reified_goal).cast_into();
    }

    // Create solver and initial state
    let user_state = DefaultUser::default();
    let user_globals = <DefaultUser as User>::UserContext::default();
    let mut solver = crate::solver::Solver::new(user_globals, false);

    // Set timeout if provided
    if let Some(timeout_ms) = config.timeout {
        let start_time = std::time::Instant::now();
        solver.set_timeout(start_time, timeout_ms);
    }

    // Initialize trace state if tracing is enabled
    let mut trace_state = config.trace.as_ref().map(|trace_config| {
        let mut state = super::trace::TraceState::new(trace_config.clone());
        state.print_search_header();
        println!("Executing query with {} variables", query_vars.len());
        state.enter_relation("query");
        state
    });

    let initial_state = crate::state::State::new(user_state);

    // Execute the goal and collect results
    let stream = solver.start(&reified_goal, initial_state);
    let mut results = Vec::new();

    // Create a mutable stream to iterate through
    let mut stream = stream;

    // Collect up to 100 results (to prevent infinite loops)
    let max_results = 100;
    let mut result_count = 0;

    while result_count < max_results {
        match solver.next(&mut stream) {
            crate::solver::SolverResult::Solution(state_box) => {
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

                // Trace the solution if tracing is enabled
                if let Some(ref mut trace) = trace_state {
                    let bindings: Vec<(String, String)> = query_result
                        .bindings
                        .iter()
                        .map(|(k, v)| (k.clone(), format!("{}", v.0)))
                        .collect();
                    trace.trace_solution(&bindings);
                }

                results.push(query_result);
                result_count += 1;
            }
            crate::solver::SolverResult::NoMoreSolutions => {
                // Natural completion (no more solutions)
                if let Some(ref mut trace) = trace_state {
                    trace.exit_relation("query", true);
                }
                break;
            }
            crate::solver::SolverResult::Timeout => {
                // Timeout occurred
                if let Some(ref mut trace) = trace_state {
                    trace.exit_relation("query", false);
                }
                return Err(InterpreterError::RuntimeError(
                    "Query execution timed out".to_string(),
                ));
            }
            crate::solver::SolverResult::Error(msg) => {
                // Error occurred during execution
                if let Some(ref mut trace) = trace_state {
                    trace.exit_relation("query", false);
                }
                return Err(InterpreterError::RuntimeError(msg));
            }
        }
    }

    // Print trace summary if tracing was enabled
    if let Some(ref trace) = trace_state {
        trace.print_summary();
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
                extract_variables_from_call_argument(arg, vars);
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
        Term::Variable(var_name) => {
            vars.push(var_name.to_string());
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
        Term::TupleStruct(compound, _) => {
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
        Term::EnumVariant(enum_variant, _) => {
            // Extract variables from enum variant construction
            match &enum_variant.kind {
                super::parser::ast::EnumVariantConstructionKind::Unit => {}
                super::parser::ast::EnumVariantConstructionKind::Tuple(args) => {
                    for arg in args {
                        extract_variables_from_term(arg, vars);
                    }
                }
                super::parser::ast::EnumVariantConstructionKind::Named(fields) => {
                    for field in fields {
                        extract_variables_from_term(&field.value, vars);
                    }
                }
            }
        }
    }
}

fn extract_variables_from_call_argument(
    arg: &super::parser::ast::CallArgument,
    vars: &mut Vec<String>,
) {
    use super::parser::ast::CallArgument;

    match arg {
        CallArgument::Term(term) => extract_variables_from_term(term, vars),
        CallArgument::MetaExpression(expr) => extract_variables_from_meta_expression(expr, vars),
    }
}

fn extract_variables_from_meta_expression(
    expr: &super::metaprogramming::MetaExpression,
    vars: &mut Vec<String>,
) {
    use super::metaprogramming::MetaExpression;

    match expr {
        MetaExpression::Variable(name, _) => {
            if !vars.contains(name) {
                vars.push(name.clone());
            }
        }
        MetaExpression::BinaryOp(_, left, right, _) => {
            extract_variables_from_meta_expression(left, vars);
            extract_variables_from_meta_expression(right, vars);
        }
        MetaExpression::Literal(_, _) => {
            // Literals don't contain variables
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

    #[test]
    fn test_query_result_creation() {
        let result: QueryResult = QueryResult::new();
        assert!(result.is_empty());
    }

    #[test]
    fn test_query_result_binding() {
        let mut result: QueryResult = QueryResult::new();
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
                assert!(matches!(left, super::super::parser::ast::Term::Variable(_)));
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
