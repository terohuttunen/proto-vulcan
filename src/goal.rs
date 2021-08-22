use crate::engine::{DefaultEngine, Engine};
use crate::operator::all::All;
use crate::operator::any::Any;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::{DefaultUser, User};
use std::fmt;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub enum Goal<U = DefaultUser, E = DefaultEngine<U>>
where
    U: User,
    E: Engine<U>,
{
    Succeed,
    Fail,
    Disj(Rc<Any<U, E>>),
    Conj(Rc<All<U, E>>),
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

    pub fn new_disj(goal_1: Goal<U, E>, goal_2: Goal<U, E>) -> Goal<U, E> {
        Goal::Disj(Rc::new(Any::new_raw(goal_1, goal_2)))
    }

    pub fn new_conj(goal_1: Goal<U, E>, goal_2: Goal<U, E>) -> Goal<U, E> {
        Goal::Conj(Rc::new(All::new_raw(goal_1, goal_2)))
    }

    pub fn succeed() -> Goal<U, E> {
        Goal::Succeed
    }

    pub fn fail() -> Goal<U, E> {
        Goal::Fail
    }

    pub fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        match self {
            Goal::Succeed => Stream::unit(Box::new(state)),
            Goal::Fail => Stream::empty(),
            Goal::Disj(g) => g.solve(solver, state),
            Goal::Conj(g) => g.solve(solver, state),
            Goal::Inner(inner) => inner.solve(solver, state),
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
            Goal::Disj(g) => Goal::Disj(Rc::clone(g)),
            Goal::Conj(g) => Goal::Conj(Rc::clone(g)),
            Goal::Inner(inner) => Goal::Inner(Rc::clone(inner)),
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
        let g = Goal::<DefaultUser>::new(TestGoal {});
        assert!(!g.is_succeed());
        assert!(!g.is_fail());
    }
}
