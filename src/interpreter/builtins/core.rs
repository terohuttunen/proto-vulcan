//! Core language builtin predicates

use super::macros::extract_two_relational;
use super::ArgumentValue;
use crate::goal::{AnyGoal, Goal};
use crate::lterm::{LTerm, LTermInner, LValue};
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use std::rc::Rc;

/// Builtin length predicate - efficiently calculates list length
pub fn length_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (list, length) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct BuiltinLengthGoal {
        list: LTerm,
        length: LTerm,
    }

    impl Solve for BuiltinLengthGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            // Walk the substitution map to get resolved values (same as assertions)
            let list_walked = state.smap_ref().walk(&self.list).clone();
            let length_walked = state.smap_ref().walk(&self.length).clone();

            // Check if list is now a concrete list
            if list_walked.is_list() {
                let count = list_walked.iter().count();
                let count_term: LTerm =
                    LTerm::from(LTermInner::Val(LValue::Number(count as isize)));

                // Use the constraint system's unification (same as assertions)
                match state.unify(&length_walked, &count_term) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                // If list is still a variable, we can't compute length yet
                // A more sophisticated implementation would add length constraints
                Stream::empty()
            }
        }
    }

    // Return the goal using the same pattern as assertions
    Goal::dynamic(Rc::new(BuiltinLengthGoal { list, length }))
}