use crate::engine::{DefaultEngine, Engine};
use crate::state::State;
use crate::user::{EmptyUser, User};
use std::fmt;
use std::rc::Rc;

#[derive(Debug)]
pub enum Goal<U = EmptyUser, E = DefaultEngine<U>>
where
    U: User,
    E: Engine<U>,
{
    Succeed,
    Fail,
    Inner(Rc<dyn Solve<U, E>>),
}

impl<U, E> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new<T: Solve<U, E> + 'static>(u: T) -> Goal<U, E> {
        Goal::Inner(Rc::new(u))
    }

    pub fn succeed() -> Goal<U, E> {
        Goal::Succeed
    }

    pub fn fail() -> Goal<U, E> {
        Goal::Fail
    }

    pub fn solve(&self, engine: &E, state: State<U>) -> E::Stream {
        match self {
            Goal::Succeed => engine.munit(state),
            Goal::Fail => engine.mzero(),
            Goal::Inner(inner) => inner.solve(engine, state),
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

impl<U: User, E: Engine<U>> Clone for Goal<U, E> {
    fn clone(&self) -> Goal<U, E> {
        match self {
            Goal::Succeed => Goal::Succeed,
            Goal::Fail => Goal::Fail,
            Goal::Inner(inner) => Goal::Inner(Rc::clone(inner)),
        }
    }
}

// A goal is a function which, given an input state, will give an output state (or infinite stream
// of output states). It encapsulates a logic query that is evaluated as infinite stream of
// states that solve the query at any given time.
pub trait Solve<U, E>: fmt::Debug
where
    U: User,
    E: Engine<U>,
{
    /// Generate a stream of solutions to the goal by applying it to some initial state.
    fn solve(&self, engine: &E, state: State<U>) -> E::Stream;
}

#[cfg(test)]
mod test {
    use super::Solve;
    use crate::engine::Engine;
    use crate::prelude::*;
    use crate::state::State;
    use crate::user::EmptyUser;

    #[test]
    fn test_goal_succeed() {
        let g = Goal::<EmptyUser>::succeed();
        assert!(g.is_succeed());
        assert!(!g.is_fail());
    }

    #[test]
    fn test_goal_fail() {
        let g = Goal::<EmptyUser>::fail();
        assert!(g.is_fail());
        assert!(!g.is_succeed());
    }

    #[derive(Debug)]
    struct TestGoal {}

    impl<E: Engine<U>, U: User> Solve<U, E> for TestGoal {
        fn solve(&self, engine: &E, _state: State<U>) -> E::Stream {
            engine.mzero()
        }
    }

    #[test]
    fn test_goal_inner() {
        let g = Goal::<EmptyUser>::new(TestGoal {});
        assert!(!g.is_succeed());
        assert!(!g.is_fail());
    }
}
