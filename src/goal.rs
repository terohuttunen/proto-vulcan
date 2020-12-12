use crate::query::EmptyUser;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Goal<U = EmptyUser>
where
    U: User
{
    inner: Rc<dyn Solver<U>>,
}

impl<U> Goal<U>
where
    U: User
{
    pub fn new<T: Solver<U> + 'static>(u: T) -> Goal<U> {
        Goal {
            inner: Rc::new(u),
        }
    }

    pub fn solve(&self, state: State<U>) -> Stream<U> {
        self.inner.solve(state)
    }

    pub fn is_succeed(&self) -> bool {
        self.inner.is_succeed()
    }

    pub fn is_fail(&self) -> bool {
        self.inner.is_fail()
    }
}

// A goal is a function which, given an input state, will give an output state (or infinite stream
// of output states). It encapsulates a logic query that is evaluated as infinite stream of
// states that solve the query at any given time.
pub trait Solver<U = EmptyUser>: fmt::Debug
where
    U: User,
{
    /// Generate a stream of solutions to the goal by applying it to some initial state.
    fn solve(&self, state: State<U>) -> Stream<U>;

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
