use crate::goal::{AnyGoal, DFSGoal, Goal, InferredGoal};
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::GoalCast;
use std::any::Any;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Conj {
    pub goal_1: Goal,
    pub goal_2: Goal,
}

impl Conj {
    pub fn new(goal_1: Goal, goal_2: Goal) -> Goal {
        if goal_1.is_succeed() && goal_2.is_succeed() {
            return Goal::succeed();
        }
        if goal_1.is_fail() || goal_2.is_fail() {
            return Goal::fail();
        }

        Goal::dynamic(Rc::new(Conj { goal_1, goal_2 }))
    }

    pub fn new_raw(goal_1: Goal, goal_2: Goal) -> Conj {
        Conj { goal_1, goal_2 }
    }

    pub fn from_vec(mut v: Vec<Goal>) -> Goal {
        let mut p = Goal::succeed();
        for g in v.drain(..).rev() {
            p = Conj::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[Goal]) -> Goal {
        let mut p = Goal::succeed();
        for g in goals.to_vec().drain(..).rev() {
            p = Conj::new(g, p);
        }
        p
    }

    pub fn from_iter<I>(iter: I) -> Goal
    where
        I: Iterator<Item = Goal>,
    {
        let mut p = Goal::succeed();
        for g in iter {
            p = Conj::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a conjunction
    // of all the goals.
    pub fn from_conjunctions(conjunctions: &[&[Goal]]) -> Goal {
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

impl Solve for Conj {
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        Stream::lazy_bind(
            LazyStream::pause(Box::new(state), self.goal_1.clone()),
            self.goal_2.clone(),
        )
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DFSConj {
    pub goal_1: DFSGoal,
    pub goal_2: DFSGoal,
}

impl DFSConj {
    pub fn new(goal_1: DFSGoal, goal_2: DFSGoal) -> DFSGoal {
        if goal_1.is_succeed() && goal_2.is_succeed() {
            return DFSGoal::succeed();
        }
        if goal_1.is_fail() || goal_2.is_fail() {
            return DFSGoal::fail();
        }

        DFSGoal::dynamic(Rc::new(DFSConj { goal_1, goal_2 }))
    }

    pub fn new_raw(goal_1: DFSGoal, goal_2: DFSGoal) -> DFSConj {
        DFSConj { goal_1, goal_2 }
    }

    pub fn from_vec(mut v: Vec<DFSGoal>) -> DFSGoal {
        let mut p = DFSGoal::succeed();
        for g in v.drain(..).rev() {
            p = DFSConj::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[DFSGoal]) -> DFSGoal {
        let mut p = DFSGoal::succeed();
        for g in goals.to_vec().drain(..).rev() {
            p = DFSConj::new(g, p);
        }
        p
    }

    pub fn from_iter<I>(iter: I) -> DFSGoal
    where
        I: Iterator<Item = DFSGoal>,
    {
        let mut p = DFSGoal::succeed();
        for g in iter {
            p = DFSConj::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a conjunction
    // of all the goals.
    pub fn from_conjunctions(conjunctions: &[&[DFSGoal]]) -> DFSGoal {
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

impl Solve for DFSConj {
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        Stream::lazy_bind_dfs(
            LazyStream::pause_dfs(Box::new(state), self.goal_1.clone()),
            self.goal_2.clone(),
        )
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct InferredConj<G: AnyGoal> {
    pub goal_1: G,
    pub goal_2: G,
}

impl<G: AnyGoal> InferredConj<G> {
    pub fn new(goal_1: G, goal_2: G) -> InferredGoal<G> {
        if goal_1.is_succeed() && goal_2.is_succeed() {
            return InferredGoal::new(G::succeed());
        }
        if goal_1.is_fail() || goal_2.is_fail() {
            return InferredGoal::new(G::fail());
        }

        InferredGoal::new(G::dynamic(Rc::new(InferredConj { goal_1, goal_2 })))
    }

    pub fn new_raw(goal_1: G, goal_2: G) -> InferredConj<G> {
        InferredConj { goal_1, goal_2 }
    }

    pub fn from_vec(mut v: Vec<G>) -> InferredGoal<G> {
        let mut p = G::succeed();
        for g in v.drain(..).rev() {
            p = InferredConj::new(g, p).cast_into();
        }
        InferredGoal::new(p)
    }

    pub fn from_array(goals: &[G]) -> InferredGoal<G> {
        let mut p = G::succeed();
        for g in goals.to_vec().drain(..).rev() {
            p = InferredConj::new(g, p).cast_into();
        }
        InferredGoal::new(p)
    }

    pub fn from_iter<I>(iter: I) -> InferredGoal<G>
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
    pub fn from_conjunctions(conjunctions: &[&[G]]) -> InferredGoal<G> {
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

impl<G: AnyGoal> Solve for InferredConj<G> {
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        if let Some(bfs) = self.as_any().downcast_ref::<InferredConj<Goal>>() {
            Stream::lazy_bind(
                LazyStream::pause(Box::new(state), bfs.goal_1.clone().cast_into()),
                bfs.goal_2.clone().cast_into(),
            )
        } else if let Some(dfs) = self.as_any().downcast_ref::<InferredConj<DFSGoal>>() {
            Stream::lazy_bind_dfs(
                LazyStream::pause_dfs(Box::new(state), dfs.goal_1.clone().cast_into()),
                dfs.goal_2.clone().cast_into(),
            )
        } else {
            unreachable!()
        }
    }
}
