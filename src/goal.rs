use crate::query::EmptyUserState;
use crate::state::State;
use crate::stream::Stream;
use crate::user::UserState;
use std::fmt;

// A goal is a function which, given an input state, will give an output state (or infinite stream
// of output states). It encapsulates a logic query that is evaluated as infinite stream of
// states that solve the query at any given time.
pub trait Goal<U = EmptyUserState>: fmt::Debug
where
    U: UserState,
{
    /// Generate a stream of solutions to the goal by applying it to some initial state.
    fn apply(&self, state: State<U>) -> Stream<U>;

    /// A function that returns `true` only if the goal is `Succeed()`. This is used to
    /// prune the search tree.
    fn is_succeed(&self) -> bool {
        false
    }

    /// A function that returns `true` only if the goal is `Fail()`. This is used to
    /// prune the search tree.
    fn is_fail(&self) -> bool {
        false
    }
}
