use crate::engine::Engine;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
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

    fn dynamic<T: Solve<U, E>>(u: T) -> Self
    where
        Self: Sized;

    fn is_succeed(&self) -> bool;

    fn is_fail(&self) -> bool;

    fn is_breakpoint(&self) -> bool;

    fn bind(state: State<U, E>, goal_1: Self, goal_2: Self) -> Stream<U, E>;

    fn mplus(state: State<U, E>, goal_1: Self, goal_2: Self) -> Stream<U, E>;

    fn pause(state: State<U, E>, goal: Self) -> Stream<U, E>;

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

    fn dynamic<T: Solve<U, E>>(u: T) -> Goal<U, E> {
        Goal::Dynamic(Rc::new(u))
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

    fn bind(state: State<U, E>, goal_1: Goal<U, E>, goal_2: Goal<U, E>) -> Stream<U, E> {
        Stream::lazy_bind(LazyStream::pause(Box::new(state), goal_1), goal_2)
    }

    fn mplus(state: State<U, E>, goal_1: Goal<U, E>, goal_2: Goal<U, E>) -> Stream<U, E> {
        Stream::lazy_mplus(
            LazyStream::pause(Box::new(state.clone()), goal_1),
            LazyStream::pause(Box::new(state), goal_2),
        )
    }

    fn pause(state: State<U, E>, goal: Goal<U, E>) -> Stream<U, E> {
        Stream::pause(Box::new(state), goal)
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

    fn dynamic<T: Solve<U, E>>(u: T) -> DFSGoal<U, E> {
        DFSGoal::Dynamic(Rc::new(u))
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

    fn bind(state: State<U, E>, goal_1: DFSGoal<U, E>, goal_2: DFSGoal<U, E>) -> Stream<U, E> {
        Stream::lazy_bind_dfs(LazyStream::pause_dfs(Box::new(state), goal_1), goal_2)
    }

    fn mplus(state: State<U, E>, goal_1: DFSGoal<U, E>, goal_2: DFSGoal<U, E>) -> Stream<U, E> {
        Stream::lazy_mplus_dfs(
            LazyStream::pause_dfs(Box::new(state.clone()), goal_1),
            LazyStream::pause_dfs(Box::new(state), goal_2),
        )
    }

    fn pause(state: State<U, E>, goal: DFSGoal<U, E>) -> Stream<U, E> {
        Stream::pause_dfs(Box::new(state), goal)
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
    goal: G,
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

impl<U, E, G> AnyGoal<U, E> for InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    fn succeed() -> InferredGoal<U, E, G> {
        InferredGoal::new(G::succeed())
    }

    fn fail() -> InferredGoal<U, E, G> {
        InferredGoal::new(G::fail())
    }

    fn breakpoint(id: &'static str) -> InferredGoal<U, E, G> {
        InferredGoal::new(G::breakpoint(id))
    }

    fn dynamic<T: Solve<U, E>>(u: T) -> InferredGoal<U, E, G> {
        InferredGoal::new(G::dynamic(u))
    }

    fn is_succeed(&self) -> bool {
        self.goal.is_succeed()
    }

    fn is_fail(&self) -> bool {
        self.goal.is_fail()
    }

    fn is_breakpoint(&self) -> bool {
        self.goal.is_breakpoint()
    }

    fn bind(
        state: State<U, E>,
        goal_1: InferredGoal<U, E, G>,
        goal_2: InferredGoal<U, E, G>,
    ) -> Stream<U, E> {
        G::bind(state, goal_1.cast_into(), goal_2.cast_into())
    }

    fn mplus(
        state: State<U, E>,
        goal_1: InferredGoal<U, E, G>,
        goal_2: InferredGoal<U, E, G>,
    ) -> Stream<U, E> {
        G::mplus(state, goal_1.cast_into(), goal_2.cast_into())
    }

    fn pause(state: State<U, E>, goal: InferredGoal<U, E, G>) -> Stream<U, E> {
        G::pause(state, goal.cast_into())
    }

    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        self.goal.solve(solver, state)
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

// Goal -> Goal
impl<U, E> GoalCast<U, E, Goal<U, E>> for Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
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
    fn cast_into(self) -> Self {
        self
    }
}

// InferredGoal -> InferredGoal
/*
impl<U, E> GoalCast<U, E, InferredGoal<U, E, Goal<U, E>>> for InferredGoal<U, E, Goal<U, E>>
where
    U: User,
    E: Engine<U>,
{
    fn cast_into(self) -> Self {
        self
    }
}

impl<U, E> GoalCast<U, E, InferredGoal<U, E, DFSGoal<U, E>>> for InferredGoal<U, E, DFSGoal<U, E>>
where
    U: User,
    E: Engine<U>,
{
    fn cast_into(self) -> Self {
        self
    }
}
*/
/*
impl<U, E, G> GoalCast<U, E, InferredGoal<U, E, G>> for InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    fn cast_into(self) -> Self {
        self
    }
}
*/

#[cfg(test)]
mod test {
    use super::AnyGoal;
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
