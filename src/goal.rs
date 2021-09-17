use crate::engine::Engine;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::marker::PhantomData;
use std::rc::Rc;

pub use crate::GoalCast;

pub trait AnyGoal<U, E>: std::fmt::Debug + std::clone::Clone + 'static
where
    U: User,
    E: Engine<U>,
{
    fn succeed() -> Self
    where
        Self: Sized;

    fn fail() -> Self
    where
        Self: Sized;

    fn breakpoint(id: &'static str) -> Self
    where
        Self: Sized;

    fn dynamic(u: Rc<dyn Solve<U, E>>) -> Self
    where
        Self: Sized;

    fn is_succeed(&self) -> bool;

    fn is_fail(&self) -> bool;

    fn is_breakpoint(&self) -> bool;

    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E>;
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

impl<U, E> AnyGoal<U, E> for Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn succeed() -> Goal<U, E> {
        Goal::Succeed
    }

    fn fail() -> Goal<U, E> {
        Goal::Fail
    }

    fn breakpoint(id: &'static str) -> Goal<U, E> {
        Goal::Breakpoint(id)
    }

    fn dynamic(u: Rc<dyn Solve<U, E>>) -> Goal<U, E> {
        Goal::Dynamic(u)
    }

    fn is_succeed(&self) -> bool {
        match self {
            Goal::Succeed => true,
            _ => false,
        }
    }

    fn is_fail(&self) -> bool {
        match self {
            Goal::Fail => true,
            _ => false,
        }
    }

    fn is_breakpoint(&self) -> bool {
        match self {
            Goal::Breakpoint(_) => true,
            _ => false,
        }
    }

    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        match self {
            Goal::Succeed => Stream::unit(Box::new(state)),
            Goal::Fail => Stream::empty(),
            Goal::Breakpoint(_) => Stream::unit(Box::new(state)),
            Goal::Dynamic(dynamic) => dynamic.solve(solver, state),
        }
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

impl<U, E> AnyGoal<U, E> for DFSGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn succeed() -> DFSGoal<U, E> {
        DFSGoal::Succeed
    }

    fn fail() -> DFSGoal<U, E> {
        DFSGoal::Fail
    }

    fn breakpoint(id: &'static str) -> DFSGoal<U, E> {
        DFSGoal::Breakpoint(id)
    }

    fn dynamic(u: Rc<dyn Solve<U, E>>) -> DFSGoal<U, E> {
        DFSGoal::Dynamic(u)
    }

    fn is_succeed(&self) -> bool {
        match self {
            DFSGoal::Succeed => true,
            _ => false,
        }
    }

    fn is_fail(&self) -> bool {
        match self {
            DFSGoal::Fail => true,
            _ => false,
        }
    }

    fn is_breakpoint(&self) -> bool {
        match self {
            DFSGoal::Breakpoint(_) => true,
            _ => false,
        }
    }

    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        match self {
            DFSGoal::Succeed => Stream::unit(Box::new(state)),
            DFSGoal::Fail => Stream::empty(),
            DFSGoal::Breakpoint(_) => Stream::unit(Box::new(state)),
            DFSGoal::Dynamic(dynamic) => dynamic.solve(solver, state),
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

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"), Clone(bound = "U: User"))]
pub struct InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    pub goal: G,
    _phantom: PhantomData<U>,
    _phantom2: PhantomData<E>,
}

impl<U, E, G> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    pub fn new(goal: G) -> InferredGoal<U, E, G> {
        InferredGoal {
            goal,
            _phantom: PhantomData,
            _phantom2: PhantomData,
        }
    }
}

// DFSGoal -> Goal
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

// InferredGoal<G> -> G
impl<U, E, G> GoalCast<U, E, G> for InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    #[inline]
    fn cast_into(self) -> G {
        self.goal
    }
}

// InferredGoal<G> -> InferredGoal<G>
impl<U, E, G> GoalCast<U, E, InferredGoal<U, E, G>> for InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    #[inline]
    fn cast_into(self) -> InferredGoal<U, E, G> {
        self
    }
}

// Goal -> Goal
impl<U, E> GoalCast<U, E, Goal<U, E>> for Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    #[inline]
    fn cast_into(self) -> Self {
        self
    }
}

// DFSGoal -> DFSGoal
impl<U, E> GoalCast<U, E, DFSGoal<U, E>> for DFSGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    #[inline]
    fn cast_into(self) -> Self {
        self
    }
}

#[cfg(test)]
mod test {
    use super::AnyGoal;
    use crate::engine::{DefaultEngine, Engine};
    use crate::prelude::*;
    use crate::solver::Solve;
    use crate::state::State;
    use crate::stream::Stream;
    use crate::user::DefaultUser;
    use std::rc::Rc;

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
        let g = Goal::<DefaultUser, DefaultEngine<DefaultUser>>::dynamic(Rc::new(TestGoal {}));
        assert!(!g.is_succeed());
        assert!(!g.is_fail());
    }
}
