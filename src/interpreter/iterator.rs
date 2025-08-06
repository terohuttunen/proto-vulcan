//! Streaming iterator for query results
//!
//! This module provides QueryResultIterator, which yields query results lazily
//! and supports infinite result sets with interruption handling.

use super::compiler;
use super::results::QueryResult;
use super::{ExecutionConfig, InterpreterError};
use crate::goal::Goal;
use crate::interpreter::runtime::context::{ArgumentValue, PredicateClosure};
use crate::lresult::LResult;
use crate::lterm::LTerm;
use crate::solver::{Solver, SolverResult};
use crate::state::State;
use crate::stream::Stream;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::Ordering;

/// Iterator that yields query results lazily, supporting infinite result sets
///
/// This iterator processes solver results directly to maintain proper variable
/// name mapping from the original query.
pub struct QueryResultIterator {
    /// The solver that processes the goal
    solver: Solver,
    /// The LTerm variables to extract from solutions
    variables: Vec<LTerm>,
    /// Variable names from the original query (e.g., ["x"] for member(x, [1, 2, 3]))
    variable_names: Vec<String>,
    /// The stream of solutions
    stream: Stream,
    /// Execution configuration controlling limits and behavior
    config: ExecutionConfig,
    /// Current number of results yielded
    result_count: usize,
    /// Start time for timeout checking
    start_time: std::time::Instant,
}

impl QueryResultIterator {
    /// Create a new query result iterator from solver components
    pub fn new(
        solver: Solver,
        variables: Vec<LTerm>,
        variable_names: Vec<String>,
        goal: crate::goal::Goal,
        initial_state: State,
        config: ExecutionConfig,
    ) -> Self {
        // Start the stream with the goal and initial state
        let stream = solver.start(&goal, initial_state);

        Self {
            solver,
            variables,
            variable_names,
            stream,
            config,
            result_count: 0,
            start_time: std::time::Instant::now(),
        }
    }

    /// Create a new query result iterator for IR-based queries with reification
    pub fn new_ir_reified(
        ir_program: std::rc::Rc<super::compiler::ir::Program>,
        environment: std::rc::Rc<std::cell::RefCell<super::environment::Environment>>,
        query: super::parser::ast::Goal,
        config: ExecutionConfig,
    ) -> Result<Self, InterpreterError> {
        use crate::user::{DefaultUser, User};
        let (ir_program, variable_names, query_predicate_id) =
            compiler::Compiler::add_query_to_program(ir_program.clone(), query)?;

        let query_vars: Vec<LTerm> = variable_names.iter().map(|v| LTerm::var(v)).collect();

        let query_args = query_vars
            .iter()
            .map(|v| ArgumentValue::Relational(v.clone()))
            .collect();

        let predicate_closure = PredicateClosure::new(
            query_predicate_id,
            query_args,
            ir_program.clone(),
            environment,
        );

        // Create solver and initial state
        let user_state = DefaultUser::default();
        let user_globals = <DefaultUser as User>::UserContext::default();
        let mut solver = crate::solver::Solver::new(user_globals, false);

        // Set timeout if provided
        if let Some(timeout_ms) = config.timeout {
            let start_time = std::time::Instant::now();
            solver.set_timeout(start_time, timeout_ms);
        }

        // Set the IR program in the solver for deferred relation calls
        solver.set_program(ir_program.clone());

        // Create the base query goal from predicate closure
        let base_query_goal = Goal::LazyMacro(Rc::new(predicate_closure));
        
        // Create reify goal for query variables (following macro pattern)
        let query_vars_term = LTerm::from_array(&query_vars);
        let reify_goal = crate::state::reify(query_vars_term);
        
        // Combine query goal and reification in a conjunction (like macro system does)
        let query_goal = crate::operator::conj::Conj::new(base_query_goal, reify_goal).into();

        let initial_state = crate::state::State::new(user_state);

        // Start the stream with the reified query goal and initial state
        let stream = solver.start(&query_goal, initial_state);

        // Create iterator with variable names preserved
        Ok(Self {
            solver,
            variables: query_vars,
            variable_names,
            stream,
            config,
            result_count: 0,
            start_time: std::time::Instant::now(),
        })
    }

    /// Get the current number of results yielded
    pub fn result_count(&self) -> usize {
        self.result_count
    }

    /// Get the elapsed execution time
    pub fn elapsed_time(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Check if execution should be interrupted
    fn should_interrupt(&self) -> bool {
        // Check interruption handler (e.g., Ctrl+C)
        if let Some(ref interrupted) = self.config.interruption_handler {
            if interrupted.load(Ordering::Relaxed) {
                return true;
            }
        }

        // Check result limit
        if let Some(limit) = self.config.result_limit {
            if self.result_count >= limit {
                return true;
            }
        }

        // Check timeout
        if let Some(timeout_ms) = self.config.timeout {
            if self.start_time.elapsed().as_millis() as u64 > timeout_ms {
                return true;
            }
        }

        false
    }

    /// Collect a limited number of results into a vector
    ///
    /// This is useful for scenarios where you want a bounded collection
    /// rather than streaming through all results.
    pub fn collect_limited(self, limit: usize) -> Result<Vec<QueryResult>, InterpreterError> {
        self.take(limit).collect()
    }

    /// Collect all results into a vector
    ///
    /// Warning: This will not terminate for infinite result sets!
    /// Use collect_limited() for safer collection.
    pub fn collect_all(self) -> Result<Vec<QueryResult>, InterpreterError> {
        self.collect()
    }

    /// Collect results with a timeout
    ///
    /// This will collect results until either the timeout is reached or
    /// no more solutions are available.
    pub fn collect_with_timeout(
        self,
        timeout_ms: u64,
    ) -> Result<Vec<QueryResult>, InterpreterError> {
        let start = std::time::Instant::now();
        let mut results = Vec::new();

        for result in self {
            if start.elapsed().as_millis() as u64 > timeout_ms {
                break;
            }
            results.push(result?);
        }

        Ok(results)
    }
}

impl Iterator for QueryResultIterator {
    type Item = Result<QueryResult, InterpreterError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Check if we should interrupt execution
        if self.should_interrupt() {
            return None;
        }

        // Process the next solution from the solver
        match self.solver.next(&mut self.stream) {
            SolverResult::Solution(state) => {
                // At this point the state has already gone through initial reification
                let smap = state.smap_ref();
                let purified_cstore = state.cstore_ref().clone().purify(smap).normalize();
                let reified_cstore = Rc::new(purified_cstore.walk_star(smap));
                
                // Create QueryResult with proper variable names
                let mut bindings = HashMap::new();
                for (var, name) in self.variables.iter().zip(&self.variable_names) {
                    let lresult = LResult(
                        smap.walk_star(var),
                        Rc::clone(&reified_cstore)
                    );
                    bindings.insert(name.clone(), lresult);
                }
                
                self.result_count += 1;
                Some(Ok(QueryResult { bindings }))
            }
            SolverResult::NoMoreSolutions => {
                if self.config.debug_enabled {
                    println!("No more solutions. Total: {}", self.result_count);
                }
                None
            }
            SolverResult::Timeout => Some(Err(crate::interpreter::InterpreterError::QueryExecutionError(
                "Query execution timed out".to_string()
            ))),
            SolverResult::Error(_) => None,
        }
    }
}

/// Convenience methods for common collection patterns
impl QueryResultIterator {
    /// Take the first result only
    pub fn first(mut self) -> Option<Result<QueryResult, InterpreterError>> {
        self.next()
    }

    /// Check if any solutions exist (consumes iterator)
    pub fn any_solutions(mut self) -> Result<bool, InterpreterError> {
        match self.next() {
            Some(Ok(_)) => Ok(true),
            Some(Err(e)) => Err(e),
            None => Ok(false),
        }
    }

    /// Count the total number of solutions (consumes iterator)
    ///
    /// Warning: This will not terminate for infinite result sets!
    pub fn count_solutions(self) -> Result<usize, InterpreterError> {
        let mut count = 0;
        for result in self {
            result?; // Check for errors
            count += 1;
        }
        Ok(count)
    }
}
