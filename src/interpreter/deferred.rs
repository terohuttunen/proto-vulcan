//! This module contains the implementation for deferred (lazy) goal execution,
//! which is crucial for handling recursive relations without causing a stack
//! overflow during the initial goal-to-AST conversion.

use super::environment::Environment;
use super::runtime::context::ExecutionContext;
use super::parser::ast::{PredicateDefinition, SearchStrategy};
use crate::goal::{AnyGoal, Goal};
use crate::lterm::LTerm;
use crate::operator::conj::Conj;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use std::cell::RefCell;
use std::rc::Rc;

/// A goal that represents a relation call that is evaluated lazily.
///
/// This struct holds all the necessary information to execute a relation call
/// at solve-time rather than at "compile-time" (AST-to-Goal conversion).
/// It stores a reference to the shared environment to look up definitions,
/// the specific relation definition to execute, and the arguments (`LTerm`s)
/// that were passed to the call.
#[derive(Clone)]
pub struct DeferredRelationCall {
    pub environment: Rc<RefCell<Environment>>,
    pub rel_def: Rc<PredicateDefinition>,
    pub call_args: Vec<LTerm>,
    /// The search strategy context from the calling site
    pub parent_search_strategy: SearchStrategy,
}

// Manual implementation of Debug to avoid issues with the recursive environment type.
impl std::fmt::Debug for DeferredRelationCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeferredRelationCall")
            .field("rel_def", &self.rel_def)
            .field("call_args", &self.call_args)
            .finish_non_exhaustive() // environment is not included
    }
}

impl DeferredRelationCall {
    pub fn new(
        environment: Rc<RefCell<Environment>>,
        rel_def: Rc<PredicateDefinition>,
        call_args: Vec<LTerm>,
        parent_search_strategy: SearchStrategy,
    ) -> Self {
        Self {
            environment,
            rel_def,
            call_args,
            parent_search_strategy,
        }
    }
}

impl Solve for DeferredRelationCall {
    fn solve(&self, solver: &Solver, state: State) -> Stream {
        // Get the IR program from the solver (if available)
        let program = match solver.program() {
            Some(program) => program,
            None => {
                return Stream::error("No IR program available in solver for deferred relation call execution".to_string());
            }
        };
        
        let mut exec_context = ExecutionContext::new(program.clone(), self.environment.clone());

        // Set up the search strategy context
        // If the relation has its own @dfs/@bfs strategy, use that; otherwise inherit from parent
        let relation_strategy = self
            .rel_def
            .search_strategy
            .unwrap_or(self.parent_search_strategy);

        // For explicit strategy annotations, check for potentially problematic combinations
        // but allow them (this is based on logic programming theory - explicit annotations override)
        if let Some(explicit_strategy) = self.rel_def.search_strategy {
            match (self.parent_search_strategy, explicit_strategy) {
                (SearchStrategy::Dfs, SearchStrategy::Bfs) => {
                    // Theoretically problematic: BFS in DFS breaks deterministic exploration
                    // But explicit @bfs annotation overrides this - relation author knows best
                    eprintln!(
                        "Warning: Relation '{}' uses @bfs within DFS context - may break deterministic exploration",
                        self.rel_def.name
                    );
                }
                _ => {
                    // DFS in BFS is fine (maintains overall fairness)
                    // BFS in BFS and DFS in DFS are also fine
                }
            }
        }

        // Push the relation's search strategy onto the context
        exec_context.push_search_strategy(relation_strategy);

        // Push a new scope for the relation's parameters.
        exec_context.push_scope();

        // Arity validation is performed at call site, so arguments should always match parameters
        assert_eq!(
            self.call_args.len(),
            self.rel_def.parameters.len(),
            "Argument count should match parameter count (validation should occur at call site)"
        );

        // Directly bind call arguments to parameter names (no fresh variables needed)
        // This preserves variable identity and eliminates coordination issues
        for (param, arg) in self.rel_def.parameters.iter().zip(self.call_args.iter()) {
            exec_context.bind_var(param.name.clone(), arg.clone());
        }

        // Convert the relation's body (AST) into a runtime goal. This is the core of the lazy evaluation.

        // Note: Macro predicates should never reach this point as they are eagerly expanded
        // in ast_relation_call_to_runtime(). This code only handles regular relations.

        // In the IR-based system, deferred relation calls should not be executing AST bodies
        // This is a legacy pattern that needs to be replaced with IR-based deferred execution
        let body_goals_result: Result<Vec<crate::goal::Goal>, crate::interpreter::InterpreterError> = 
            Err(crate::interpreter::InterpreterError::RuntimeError(
                "Deferred relation calls with AST bodies not supported in IR-based execution. Relations should be compiled to IR first.".to_string()
            ));

        // Pop the scope now that the body has been converted.
        exec_context.pop_scope();

        // Check if body conversion was successful.
        let all_goals = match body_goals_result {
            Ok(goals) => goals,
            Err(err) => {
                return Stream::error(format!(
                    "Failed to convert relation '{}' body to runtime goals: {}",
                    self.rel_def.name, err
                ));
            }
        };

        // Create a single conjunction goal and solve it.
        let mut final_goal = Goal::succeed();
        for goal in all_goals.into_iter().rev() {
            final_goal = Conj::new(goal, final_goal);
        }
        final_goal.solve(solver, state)
    }
}
