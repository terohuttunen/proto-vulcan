//! Streaming iterator for query results
//!
//! This module provides QueryResultIterator, which yields query results lazily
//! and supports infinite result sets with interruption handling.

use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::collections::HashMap;
use super::results::QueryResult;
use super::{ExecutionConfig, InterpreterError};
use crate::solver::{Solver, SolverResult};
use crate::state::State;
use crate::stream::Stream;
use crate::query::ResultIterator;
use crate::lterm::LTerm;

/// Iterator that yields query results lazily, supporting infinite result sets
///
/// This iterator wraps the proven ResultIterator from src/query.rs to leverage
/// its working result processing while maintaining the IR-based interpreter API.
pub struct QueryResultIterator {
    /// The inner ResultIterator that does the actual result processing
    inner: ResultIterator<QueryResult>,
    /// Variable names from the original query (e.g., ["q"] for member_test_2(q))
    variable_names: Vec<String>,
    /// Execution configuration controlling limits and behavior
    config: ExecutionConfig,
    /// Current number of results yielded
    result_count: usize,
    /// Start time for timeout checking
    start_time: std::time::Instant,
}

impl QueryResultIterator {
    /// Create a new query result iterator from ResultIterator components
    pub fn new(
        solver: Solver,
        variables: Vec<LTerm>,
        variable_names: Vec<String>,
        goal: crate::goal::Goal,
        initial_state: State,
        config: ExecutionConfig,
    ) -> Self {
        use crate::query::ResultIterator;
        
        let inner = ResultIterator::new(solver, variables, goal, initial_state);
        
        Self {
            inner,
            variable_names,
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
        use super::parser::ast;
        use super::compiler::Compiler;
        use super::runtime::context::{PredicateClosure, ArgumentValue, ExecutionContext};
        use crate::interpreter::symbol_table::InternedSymbol;
        use crate::user::{DefaultUser, User};

        // Step 1: Extract variables from the query for later result collection
        let query_vars = super::query::extract_variables_from_goal(&query);
        
        // Step 2: Add the query directly to the existing IR program without AST conversion
        // Use Rc clone for copy-on-write behavior (only clones if there are other references in add_query_to_program)
        let query_ir_program_rc = Compiler::add_query_to_program(
            ir_program.clone(),
            query.clone()
        ).map_err(|e| {
            InterpreterError::RuntimeError(format!("Query compilation failed: {:?}", e))
        })?;
        let query_predicate_id = super::compiler::ir::PredicateId::new("::__query__");
        let query_ir_predicate = query_ir_program_rc.registry.get_predicate(&query_predicate_id)
            .ok_or_else(|| InterpreterError::RuntimeError("Failed to find compiled query predicate".to_string()))?
            .clone();
        
        // Step 4: Create argument values for the query variables (all relational, no meta)
        let mut execution_context = ExecutionContext::new(query_ir_program_rc.clone(), environment.clone());
        
        // Debug: Show what predicates are available in the query IR program (disabled)
        // println!("DEBUG: Query IR program has {} items", query_ir_program_rc.registry.all_items().count());
        // for item in query_ir_program_rc.registry.all_items() {
        //     if let super::compiler::ir::Item::Predicate(predicate) = item {
        //         println!("DEBUG: Available predicate: {}", predicate.id.as_ref().path);
        //     }
        // }
        
        // The query_ir_program already contains the base program, so use it for predicate resolution
        execution_context.set_base_program(query_ir_program_rc.clone());
        let captured_args: Vec<ArgumentValue> = query_vars.iter()
            .map(|var_name| {
                let fresh_var = execution_context.create_named_fresh_var(var_name);
                let symbol = InternedSymbol::from(var_name.clone());
                execution_context.bind_var(symbol, fresh_var.clone());
                ArgumentValue::Relational(fresh_var)
            })
            .collect();
        
        // Step 5: Create predicate closure for the query
        let query_closure = PredicateClosure::new(
            std::rc::Rc::new(query_ir_predicate),
            captured_args,
            query_ir_program_rc.clone(),
            environment,
        );
        
        // Step 6: Create final goal from query closure
        // No need to add reification goals - the ResultIterator handles reification properly
        use crate::goal::Goal;
        let final_goal = Goal::LazyMacro(std::rc::Rc::new(query_closure));
        
        // Step 7: Collect variable LTerms for ResultIterator
        let variable_bindings = execution_context.get_all_variable_bindings();
        let mut variables = Vec::new();
        for var_name in &query_vars {
            let symbol = InternedSymbol::from(var_name.clone());
            if let Some(variable_value) = variable_bindings.get(&symbol) {
                if let super::runtime::context::VariableValue::Relational(var_term) = variable_value {
                    variables.push(var_term.clone());
                }
            }
        }
        
        // Step 8: Create solver and initial state
        let user_state = DefaultUser::default();
        let user_globals = <DefaultUser as User>::UserContext::default();
        let mut solver = crate::solver::Solver::new(user_globals, false);
        
        // Set timeout if provided
        if let Some(timeout_ms) = config.timeout {
            let start_time = std::time::Instant::now();
            solver.set_timeout(start_time, timeout_ms);
        }
        
        // Set the IR program in the solver for deferred relation calls
        solver.set_program(query_ir_program_rc.clone());
        
        let initial_state = crate::state::State::new(user_state);
        
        // Step 9: Create ResultIterator with the proven result processing logic
        let inner = ResultIterator::new(solver, variables, final_goal, initial_state);
        
        // Create wrapper iterator with variable names preserved
        Ok(Self {
            inner,
            variable_names: query_vars,
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

        // Delegate to the inner ResultIterator which has proven result processing logic
        match self.inner.next() {
            Some(result) => {
                self.result_count += 1;

                // Convert the generic "v0", "v1" variable names to actual query variable names
                let mut new_bindings = HashMap::new();
                for (index, var_name) in self.variable_names.iter().enumerate() {
                    let generic_name = format!("v{}", index);
                    if let Some(lresult) = result.bindings.get(&generic_name) {
                        new_bindings.insert(var_name.clone(), lresult.clone());
                    }
                }
                let remapped_result = QueryResult { bindings: new_bindings };

                // Debug output if enabled
                if self.config.debug_enabled {
                    println!(
                        "Solution {}: {} bindings",
                        self.result_count,
                        remapped_result.binding_count()
                    );
                }

                Some(Ok(remapped_result))
            }
            None => {
                if self.config.debug_enabled {
                    println!("No more solutions. Total: {}", self.result_count);
                }
                None
            }
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
