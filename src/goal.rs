use crate::state::State;
use crate::stream::Stream;
use crate::user::{EmptyUser, User};
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Goal<U = EmptyUser>
where
    U: User,
{
    Succeed,
    Fail,
    Inner(Rc<dyn Solve<U>>),
}

impl<U> Goal<U>
where
    U: User,
{
    pub fn new<T: Solve<U> + 'static>(u: T) -> Goal<U> {
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
pub trait Solve<U = EmptyUser>: fmt::Debug
where
    U: User,
{
    /// Generate a stream of solutions to the goal by applying it to some initial state.
    fn solve(&self, state: State<U>) -> Stream<U>;
}

#[cfg(test)]
mod test {
    use super::Solve;
    use crate::state::State;
    use crate::stream::Stream;
    use crate::user::EmptyUser;
    use crate::*;

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

    impl<U: User> Solve<U> for TestGoal {
        fn solve(&self, _state: State<U>) -> Stream<U> {
            Stream::empty()
        }
    }

    #[test]
    fn test_goal_inner() {
        let g = Goal::<EmptyUser>::new(TestGoal {});
        assert!(!g.is_succeed());
        assert!(!g.is_fail());
    }
}
