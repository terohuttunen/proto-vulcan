//! This module contains the implementation for deferred (lazy) goal execution,
//! which is crucial for handling recursive relations without causing a stack
//! overflow during the initial goal-to-AST conversion.

use super::environment::Environment;
use super::execution::ExecutionContext;
use super::parser::ast::RelationDefinition;
use crate::engine::Engine;
use crate::goal::{AnyGoal, Goal, GoalCast};
use crate::lterm::LTerm;
use crate::operator::conj::Conj;
use crate::relation::eq;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
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
pub struct DeferredRelationCall<U: User, E: Engine<U>> {
    pub environment: Rc<RefCell<Environment<U, E>>>,
    pub rel_def: Rc<RelationDefinition>,
    pub call_args: Vec<LTerm<U, E>>,
}

// Manual implementation of Debug to avoid issues with the recursive environment type.
impl<U: User, E: Engine<U>> std::fmt::Debug for DeferredRelationCall<U, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeferredRelationCall")
            .field("rel_def", &self.rel_def)
            .field("call_args", &self.call_args)
            .finish_non_exhaustive() // environment is not included
    }
}

impl<U: User, E: Engine<U>> DeferredRelationCall<U, E> {
    pub fn new(
        environment: Rc<RefCell<Environment<U, E>>>,
        rel_def: Rc<RelationDefinition>,
        call_args: Vec<LTerm<U, E>>,
    ) -> Self {
        Self {
            environment,
            rel_def,
            call_args,
        }
    }
}

impl<U: User, E: Engine<U>> Solve<U, E> for DeferredRelationCall<U, E> {
    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        // Create a temporary execution context to convert the relation body from AST to a runtime goal.
        let mut exec_context = ExecutionContext::new(self.environment.clone());

        // Push a new scope for the relation's parameters.
        exec_context.push_scope();

        // Create fresh variables for each parameter and bind them in the new scope.
        let mut param_terms = Vec::new();
        for param in &self.rel_def.parameters {
            let fresh_var = exec_context.create_fresh_var();
            exec_context.bind_var(param.name.clone(), fresh_var.clone());
            param_terms.push(fresh_var);
        }

        // Create unification goals to unify the call-site arguments with the fresh parameter variables.
        let mut unify_goals: Vec<Goal<U, E>> = self
            .call_args
            .iter()
            .zip(param_terms.iter())
            .map(|(arg, param)| eq(arg.clone(), param.clone()).cast_into())
            .collect();

        // Convert the relation's body (AST) into a runtime goal. This is the core of the lazy evaluation.
        let body_goals = self
            .rel_def
            .body
            .iter()
            .map(|g| exec_context.ast_goal_to_runtime(g))
            .collect::<Result<Vec<_>, _>>();

        // Pop the scope now that the body has been converted.
        exec_context.pop_scope();

        // Check if body conversion was successful.
        let mut body_goals = match body_goals {
            Ok(goals) => goals,
            Err(_) => return Stream::empty(), // If conversion fails, the goal fails.
        };

        // Combine unification goals with the body goals.
        let mut all_goals = unify_goals;
        all_goals.append(&mut body_goals);

        // Create a single conjunction goal and solve it.
        let mut final_goal = Goal::succeed();
        for goal in all_goals.into_iter().rev() {
            final_goal = Conj::new(goal, final_goal);
        }
        final_goal.solve(solver, state)
    }
}
