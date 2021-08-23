use crate::engine::{DefaultEngine, Engine};
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::{DefaultUser, User};
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"), Clone(bound = "U: User"))]
pub enum Goal<U = DefaultUser, E = DefaultEngine<U>>
where
    U: User,
    E: Engine<U>,
{
    Succeed,
    Fail,
    Breakpoint(&'static str),
    Dynamic(Rc<dyn Solve<U, E>>),
}

impl<U, E> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn dynamic<T: Solve<U, E>>(u: T) -> Goal<U, E> {
        Goal::Dynamic(Rc::new(u))
    }

    pub fn succeed() -> Goal<U, E> {
        Goal::Succeed
    }

    pub fn fail() -> Goal<U, E> {
        Goal::Fail
    }

    pub fn breakpoint(id: &'static str) -> Goal<U, E> {
        Goal::Breakpoint(id)
    }

    pub fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        match self {
            Goal::Succeed => Stream::unit(Box::new(state)),
            Goal::Fail => Stream::empty(),
            Goal::Breakpoint(_) => Stream::unit(Box::new(state)),
            Goal::Dynamic(dynamic) => dynamic.solve(solver, state),
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

    pub fn is_breakpoint(&self) -> bool {
        match self {
            Goal::Breakpoint(_) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::engine::Engine;
    use crate::prelude::*;
    use crate::solver::Solve;
    use crate::state::State;
    use crate::stream::Stream;
    use crate::user::DefaultUser;

    #[test]
    fn test_goal_succeed() {
        let g = Goal::<DefaultUser>::succeed();
        assert!(g.is_succeed());
        assert!(!g.is_fail());
    }

    #[test]
    fn test_goal_fail() {
        let g = Goal::<DefaultUser>::fail();
        assert!(g.is_fail());
        assert!(!g.is_succeed());
    }

    #[derive(Debug)]
    struct TestGoal {}

    impl<E: Engine<U>, U: User> Solve<U, E> for TestGoal {
        fn solve(&self, _engine: &Solver<U, E>, _state: State<U, E>) -> Stream<U, E> {
            Stream::empty()
        }
    }

    #[test]
    fn test_goal_inner() {
        let g = Goal::<DefaultUser>::dynamic(TestGoal {});
        assert!(!g.is_succeed());
        assert!(!g.is_fail());
    }
}
