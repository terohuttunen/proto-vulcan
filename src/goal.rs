use crate::engine::Engine;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use crate::GoalCast;
use std::rc::Rc;

pub trait AnyGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
}

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"), Clone(bound = "U: User"))]
pub enum Goal<U, E>
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

impl<U, E> AnyGoal<U, E> for Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
}

impl<U, E> GoalCast<U, E, Goal<U, E>> for Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    #[inline]
    fn cast_into(self) -> Goal<U, E> {
        self.into()
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"), Clone(bound = "U: User"))]
pub enum DFSGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Succeed,
    Fail,
    Breakpoint(&'static str),
    Dynamic(Rc<dyn Solve<U, E>>),
}

impl<U, E> DFSGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn dynamic<T: Solve<U, E>>(u: T) -> DFSGoal<U, E> {
        DFSGoal::Dynamic(Rc::new(u))
    }

    pub fn succeed() -> DFSGoal<U, E> {
        DFSGoal::Succeed
    }

    pub fn fail() -> DFSGoal<U, E> {
        DFSGoal::Fail
    }

    pub fn breakpoint(id: &'static str) -> DFSGoal<U, E> {
        DFSGoal::Breakpoint(id)
    }

    pub fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        match self {
            DFSGoal::Succeed => Stream::unit(Box::new(state)),
            DFSGoal::Fail => Stream::empty(),
            DFSGoal::Breakpoint(_) => Stream::unit(Box::new(state)),
            DFSGoal::Dynamic(dynamic) => dynamic.solve(solver, state),
        }
    }

    pub fn is_succeed(&self) -> bool {
        match self {
            DFSGoal::Succeed => true,
            _ => false,
        }
    }

    pub fn is_fail(&self) -> bool {
        match self {
            DFSGoal::Fail => true,
            _ => false,
        }
    }

    pub fn is_breakpoint(&self) -> bool {
        match self {
            DFSGoal::Breakpoint(_) => true,
            _ => false,
        }
    }
}

impl<U, E> AnyGoal<U, E> for DFSGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
}

impl<U, E> Into<AdaptiveGoal<U, E>> for DFSGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn into(self) -> AdaptiveGoal<U, E> {
        match self {
            DFSGoal::Succeed => AdaptiveGoal::Succeed,
            DFSGoal::Fail => AdaptiveGoal::Fail,
            DFSGoal::Breakpoint(id) => AdaptiveGoal::Breakpoint(id),
            DFSGoal::Dynamic(dynamic) => AdaptiveGoal::Dynamic(dynamic),
        }
    }
}

impl<U, E> Into<Goal<U, E>> for DFSGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn into(self) -> Goal<U, E> {
        match self {
            DFSGoal::Succeed => Goal::Succeed,
            DFSGoal::Fail => Goal::Fail,
            DFSGoal::Breakpoint(id) => Goal::Breakpoint(id),
            DFSGoal::Dynamic(dynamic) => Goal::Dynamic(dynamic),
        }
    }
}

impl<U, E> GoalCast<U, E, AdaptiveGoal<U, E>> for DFSGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    #[inline]
    fn cast_into(self) -> AdaptiveGoal<U, E> {
        self.into()
    }
}

impl<U, E> GoalCast<U, E, Goal<U, E>> for DFSGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    #[inline]
    fn cast_into(self) -> Goal<U, E> {
        self.into()
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"), Clone(bound = "U: User"))]
pub enum AdaptiveGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Succeed,
    Fail,
    Breakpoint(&'static str),
    Dynamic(Rc<dyn Solve<U, E>>),
}

impl<U, E> AdaptiveGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn dynamic<T: Solve<U, E>>(u: T) -> AdaptiveGoal<U, E> {
        AdaptiveGoal::Dynamic(Rc::new(u))
    }

    pub fn succeed() -> AdaptiveGoal<U, E> {
        AdaptiveGoal::Succeed
    }

    pub fn fail() -> AdaptiveGoal<U, E> {
        AdaptiveGoal::Fail
    }

    pub fn breakpoint(id: &'static str) -> AdaptiveGoal<U, E> {
        AdaptiveGoal::Breakpoint(id)
    }

    pub fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        match self {
            AdaptiveGoal::Succeed => Stream::unit(Box::new(state)),
            AdaptiveGoal::Fail => Stream::empty(),
            AdaptiveGoal::Breakpoint(_) => Stream::unit(Box::new(state)),
            AdaptiveGoal::Dynamic(dynamic) => dynamic.solve(solver, state),
        }
    }

    pub fn is_succeed(&self) -> bool {
        match self {
            AdaptiveGoal::Succeed => true,
            _ => false,
        }
    }

    pub fn is_fail(&self) -> bool {
        match self {
            AdaptiveGoal::Fail => true,
            _ => false,
        }
    }

    pub fn is_breakpoint(&self) -> bool {
        match self {
            AdaptiveGoal::Breakpoint(_) => true,
            _ => false,
        }
    }
}

impl<U, E> AnyGoal<U, E> for AdaptiveGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
}

impl<U, E> Into<Goal<U, E>> for AdaptiveGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn into(self) -> Goal<U, E> {
        match self {
            AdaptiveGoal::Succeed => Goal::Succeed,
            AdaptiveGoal::Fail => Goal::Fail,
            AdaptiveGoal::Breakpoint(id) => Goal::Breakpoint(id),
            AdaptiveGoal::Dynamic(dynamic) => Goal::Dynamic(dynamic),
        }
    }
}

impl<U, E> GoalCast<U, E, Goal<U, E>> for AdaptiveGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    #[inline]
    fn cast_into(self) -> Goal<U, E> {
        self.into()
    }
}

impl<U, E> Into<AdaptiveGoal<U, E>> for Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn into(self) -> AdaptiveGoal<U, E> {
        match self {
            Goal::Succeed => AdaptiveGoal::Succeed,
            Goal::Fail => AdaptiveGoal::Fail,
            Goal::Breakpoint(id) => AdaptiveGoal::Breakpoint(id),
            Goal::Dynamic(dynamic) => AdaptiveGoal::Dynamic(dynamic),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::engine::{DefaultEngine, Engine};
    use crate::prelude::*;
    use crate::solver::Solve;
    use crate::state::State;
    use crate::stream::Stream;
    use crate::user::DefaultUser;

    #[test]
    fn test_goal_succeed() {
        let g = Goal::<DefaultUser, DefaultEngine<DefaultUser>>::succeed();
        assert!(g.is_succeed());
        assert!(!g.is_fail());
    }

    #[test]
    fn test_goal_fail() {
        let g = Goal::<DefaultUser, DefaultEngine<DefaultUser>>::fail();
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
        let g = Goal::<DefaultUser, DefaultEngine<DefaultUser>>::dynamic(TestGoal {});
        assert!(!g.is_succeed());
        assert!(!g.is_fail());
    }
}
