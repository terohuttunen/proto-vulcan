use crate::query::EmptyUser;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Goal<U = EmptyUser>
where
    U: User
{
    Succeed,
    Fail,
    Inner(Rc<dyn Solver<U>>),
}

impl<U> Goal<U>
where
    U: User
{
    pub fn new<T: Solver<U> + 'static>(u: T) -> Goal<U> {
        Goal::Inner(Rc::new(u))
    }

    pub fn succeed() -> Goal<U> {
        Goal::Succeed
    }

    pub fn fail() -> Goal<U> {
        Goal::Fail
    }

    pub fn solve(&self, state: State<U>) -> Stream<U> {
        match self {
            Goal::Succeed => Stream::from(state),
            Goal::Fail => Stream::empty(),
            Goal::Inner(inner) => inner.solve(state),
        }
    }

    pub fn is_succeed(&self) -> bool {
        match self {
            Goal::Succeed => true,
            _ => false,
        }
    }

    pub fn is_fail(&self) -> bool {
        match self {
            Goal::Fail => true,
            _ => false,
        }
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
}
