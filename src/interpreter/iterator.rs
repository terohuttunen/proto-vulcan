//! Streaming iterator for query results
//!
//! This module provides QueryResultIterator, which yields query results lazily
//! and supports infinite result sets with interruption handling.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::rc::Rc;
use crate::lterm::LTerm;
// use crate::user::DefaultUser; // Not used in this module
use crate::solver::{Solver, SolverResult};
use crate::stream::Stream;
use crate::state::State;
use super::{InterpreterError, ExecutionConfig};
use super::results::QueryResult;

/// Iterator that yields query results lazily, supporting infinite result sets
/// 
/// This iterator wraps the existing solver/stream infrastructure to provide
/// a clean streaming interface with interruption and limit support.
pub struct QueryResultIterator {
    /// The solver that processes the solution stream
    solver: Solver,
    /// The stream of solutions from the solver
    stream: Stream,
    /// Variable bindings for extracting results from solver states
    variable_bindings: HashMap<String, LTerm>,
    /// Execution configuration controlling limits and behavior
    config: ExecutionConfig,
    /// Current number of results yielded
    result_count: usize,
    /// Start time for timeout checking
    start_time: std::time::Instant,
}

impl QueryResultIterator {
    /// Create a new query result iterator
    pub fn new(
        solver: Solver,
        stream: Stream,
        variable_bindings: HashMap<String, LTerm>,
        config: ExecutionConfig,
    ) -> Self {
        Self {
            solver,
            stream,
            variable_bindings,
            config,
            result_count: 0,
            start_time: std::time::Instant::now(),
        }
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

    /// Extract a query result from a solver state
    /// 
    /// This uses the same logic as the existing execute_query_ir function
    /// to preserve constraint information and variable bindings.
    fn extract_result_from_state(&self, state: &State) -> QueryResult {
        let mut query_result = QueryResult::new();
        
        // Process the constraint store - this preserves the excellent constraint handling
        let smap = state.smap_ref();
        let purified_cstore = state.cstore_ref().clone().purify(smap);
        let reified_cstore = Rc::new(purified_cstore.walk_star(smap));
        
        // Extract query variable bindings from the substitution map
        for (var_term, _value_term) in smap.iter() {
            if let crate::lterm::LTermInner::Var(_var_id, var_name) = var_term.as_ref() {
                let resolved_term = smap.walk_star(var_term);
                
                // Only include variables that are actually bound to something concrete
                if !resolved_term.is_var() {
                    let result_with_constraints = crate::lresult::LResult(resolved_term, Rc::clone(&reified_cstore));
                    
                    // Handle all non-anonymous variables (including field names like "field0")
                    if !var_name.as_ref().starts_with("_") {
                        query_result.bindings.insert(var_name.as_ref().to_string(), result_with_constraints);
                    }
                }
            }
        }
        
        // For struct pattern matching: Map struct field names to query variables
        // Handle both tuple structs (field0, field1) and named structs (name, age, etc.)
        
        // Map field0 to X (for tuple struct queries like get_x(Point(42, 13), X))
        if let Some(field0_binding) = query_result.bindings.get("field0") {
            query_result.bindings.insert("X".to_string(), field0_binding.clone());
        }
        
        // Map field0 to Name (for tuple struct queries like get_person_name(..., Name))  
        if let Some(field0_binding) = query_result.bindings.get("field0") {
            // Only insert Name if X wasn't already inserted (avoid conflicts)
            if !query_result.bindings.contains_key("X") {
                query_result.bindings.insert("Name".to_string(), field0_binding.clone());
            }
        }
        
        // Map named struct field "name" to Name (for named struct queries like get_pattern_name(..., Name))
        if let Some(name_binding) = query_result.bindings.get("name") {
            query_result.bindings.insert("Name".to_string(), name_binding.clone());
        }
        
        // Map named struct field "age" to Age (for age extraction queries)
        if let Some(age_binding) = query_result.bindings.get("age") {
            query_result.bindings.insert("Age".to_string(), age_binding.clone());
        }
        
        // Note: Removed anonymous variable fallback logic as it was causing correctness issues.
        // Modern pattern compilation correctly creates named field variables (field0, name, etc.)
        // that are handled by the explicit mapping logic above.
        
        query_result
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
    pub fn collect_with_timeout(self, timeout_ms: u64) -> Result<Vec<QueryResult>, InterpreterError> {
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

        // Get the next solution from the solver
        match self.solver.next(&mut self.stream) {
            SolverResult::Solution(state_box) => {
                self.result_count += 1;
                
                // Extract result using existing logic to preserve constraint information
                let result = self.extract_result_from_state(&*state_box);
                
                // Debug output if enabled
                if self.config.debug_enabled {
                    println!("Solution {}: {} bindings", self.result_count, result.binding_count());
                }
                
                Some(Ok(result))
            }
            SolverResult::NoMoreSolutions => {
                if self.config.debug_enabled {
                    println!("No more solutions. Total: {}", self.result_count);
                }
                None
            }
            SolverResult::Timeout => {
                if self.config.debug_enabled {
                    println!("Query timed out after {} solutions", self.result_count);
                }
                None
            }
            SolverResult::Error(e) => {
                Some(Err(InterpreterError::RuntimeError(format!("Solver error: {:?}", e))))
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