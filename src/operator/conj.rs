use crate::engine::Engine;
use crate::goal::{AnyGoal, DFSGoal, Goal, InferredGoal};
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;
use crate::GoalCast;
use std::any::Any;
use std::marker::PhantomData;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct Conj<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub goal_1: Goal<U, E>,
    pub goal_2: Goal<U, E>,
}

impl<U, E> Conj<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(goal_1: Goal<U, E>, goal_2: Goal<U, E>) -> Goal<U, E> {
        if goal_1.is_succeed() && goal_2.is_succeed() {
            return Goal::succeed();
        }
        if goal_1.is_fail() || goal_2.is_fail() {
            return Goal::fail();
        }

        Goal::dynamic(Conj { goal_1, goal_2 })
    }

    pub fn new_raw(goal_1: Goal<U, E>, goal_2: Goal<U, E>) -> Conj<U, E> {
        Conj { goal_1, goal_2 }
    }

    pub fn from_vec(mut v: Vec<Goal<U, E>>) -> Goal<U, E> {
        let mut p = Goal::succeed();
        for g in v.drain(..).rev() {
            p = Conj::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[Goal<U, E>]) -> Goal<U, E> {
        let mut p = Goal::succeed();
        for g in goals.to_vec().drain(..).rev() {
            p = Conj::new(g, p);
        }
        p
    }

    pub fn from_iter<I>(iter: I) -> Goal<U, E>
    where
        I: Iterator<Item = Goal<U, E>>,
    {
        let mut p = Goal::succeed();
        for g in iter {
            p = Conj::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a conjunction
    // of all the goals.
    pub fn from_conjunctions(conjunctions: &[&[Goal<U, E>]]) -> Goal<U, E> {
        let mut p = Goal::succeed();
        for g in conjunctions
            .iter()
            .map(|conj| Conj::from_array(*conj))
            .rev()
        {
            p = Conj::new(g, p);
        }
        p
    }
}

impl<U, E> Solve<U, E> for Conj<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        Stream::lazy_bind(
            LazyStream::pause(Box::new(state), self.goal_1.clone()),
            self.goal_2.clone(),
        )
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct DFSConj<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub goal_1: DFSGoal<U, E>,
    pub goal_2: DFSGoal<U, E>,
}

impl<U, E> DFSConj<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(goal_1: DFSGoal<U, E>, goal_2: DFSGoal<U, E>) -> DFSGoal<U, E> {
        if goal_1.is_succeed() && goal_2.is_succeed() {
            return DFSGoal::succeed();
        }
        if goal_1.is_fail() || goal_2.is_fail() {
            return DFSGoal::fail();
        }

        DFSGoal::dynamic(DFSConj { goal_1, goal_2 })
    }

    pub fn new_raw(goal_1: DFSGoal<U, E>, goal_2: DFSGoal<U, E>) -> DFSConj<U, E> {
        DFSConj { goal_1, goal_2 }
    }

    pub fn from_vec(mut v: Vec<DFSGoal<U, E>>) -> DFSGoal<U, E> {
        let mut p = DFSGoal::succeed();
        for g in v.drain(..).rev() {
            p = DFSConj::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[DFSGoal<U, E>]) -> DFSGoal<U, E> {
        let mut p = DFSGoal::succeed();
        for g in goals.to_vec().drain(..).rev() {
            p = DFSConj::new(g, p);
        }
        p
    }

    pub fn from_iter<I>(iter: I) -> DFSGoal<U, E>
    where
        I: Iterator<Item = DFSGoal<U, E>>,
    {
        let mut p = DFSGoal::succeed();
        for g in iter {
            p = DFSConj::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a conjunction
    // of all the goals.
    pub fn from_conjunctions(conjunctions: &[&[DFSGoal<U, E>]]) -> DFSGoal<U, E> {
        let mut p = DFSGoal::succeed();
        for g in conjunctions
            .iter()
            .map(|conj| DFSConj::from_array(*conj))
            .rev()
        {
            p = DFSConj::new(g, p);
        }
        p
    }
}

impl<U, E> Solve<U, E> for DFSConj<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        Stream::lazy_bind_dfs(
            LazyStream::pause_dfs(Box::new(state), self.goal_1.clone()),
            self.goal_2.clone(),
        )
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct InferredConj<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    pub goal_1: G,
    pub goal_2: G,
    _phantom: PhantomData<U>,
    _phantom2: PhantomData<E>,
}

impl<U, E, G> InferredConj<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E> + 'static,
{
    pub fn new(goal_1: G, goal_2: G) -> InferredGoal<U, E, G> {
        if goal_1.is_succeed() && goal_2.is_succeed() {
            return InferredGoal::succeed();
        }
        if goal_1.is_fail() || goal_2.is_fail() {
            return InferredGoal::fail();
        }

        InferredGoal::dynamic(InferredConj {
            goal_1,
            goal_2,
            _phantom: PhantomData,
            _phantom2: PhantomData,
        })
    }

    pub fn new_raw(goal_1: G, goal_2: G) -> InferredConj<U, E, G> {
        InferredConj {
            goal_1,
            goal_2,
            _phantom: PhantomData,
            _phantom2: PhantomData,
        }
    }

    pub fn from_vec(mut v: Vec<G>) -> InferredGoal<U, E, G> {
        let mut p = G::succeed();
        for g in v.drain(..).rev() {
            p = InferredConj::new(g, p).cast_into();
        }
        InferredGoal::new(p)
    }

    pub fn from_array(goals: &[G]) -> InferredGoal<U, E, G> {
        let mut p = G::succeed();
        for g in goals.to_vec().drain(..).rev() {
            p = InferredConj::new(g, p).cast_into();
        }
        InferredGoal::new(p)
    }

    pub fn from_iter<I>(iter: I) -> InferredGoal<U, E, G>
    where
        I: Iterator<Item = G>,
    {
        let mut p = G::succeed();
        for g in iter {
            p = InferredConj::new(g, p).cast_into();
        }
        InferredGoal::new(p)
    }

    // The parameter is a list of conjunctions, and the resulting goal is a conjunction
    // of all the goals.
    pub fn from_conjunctions(conjunctions: &[&[G]]) -> InferredGoal<U, E, G> {
        let mut p = G::succeed();
        for g in conjunctions
            .iter()
            .map(|conj| InferredConj::from_array(*conj).cast_into())
            .rev()
        {
            p = InferredConj::new(g, p).cast_into();
        }
        InferredGoal::new(p)
    }

    pub fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<U, E, G> Solve<U, E> for InferredConj<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E> + 'static,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        if let Some(bfs) = self
            .as_any()
            .downcast_ref::<InferredConj<U, E, Goal<U, E>>>()
        {
            Stream::lazy_bind(
                LazyStream::pause(Box::new(state), bfs.goal_1.clone().cast_into()),
                bfs.goal_2.clone().cast_into(),
            )
        } else if let Some(dfs) = self
            .as_any()
            .downcast_ref::<InferredConj<U, E, DFSGoal<U, E>>>()
        {
            Stream::lazy_bind_dfs(
                LazyStream::pause_dfs(Box::new(state), dfs.goal_1.clone().cast_into()),
                dfs.goal_2.clone().cast_into(),
            )
        } else {
            unreachable!()
        }
    }
}
