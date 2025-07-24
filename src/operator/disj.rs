use crate::goal::{AnyGoal, DFSGoal, Goal};
use crate::operator::conj::{Conj, DFSConj};
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Disj {
    pub goal_1: Goal,
    pub goal_2: Goal,
}

impl Disj {
    pub fn new(goal_1: Goal, goal_2: Goal) -> Goal {
        Goal::Dynamic(Rc::new(Disj { goal_1, goal_2 }))
    }

    pub fn new_raw(goal_1: Goal, goal_2: Goal) -> Disj {
        Disj { goal_1, goal_2 }
    }

    pub fn from_vec(mut v: Vec<Goal>) -> Goal {
        let mut p = Goal::fail();
        for g in v.drain(..).rev() {
            p = Disj::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[Goal]) -> Goal {
        let mut p = Goal::fail();
        for g in goals.to_vec().drain(..).rev() {
            p = Disj::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a disjunction
    // of conjunctions.
    pub fn from_conjunctions(conjunctions: &[&[Goal]]) -> Goal {
        let mut p = Goal::fail();
        for g in conjunctions
            .iter()
            .map(|conj| Conj::from_array(*conj))
            .rev()
        {
            p = Disj::new(g, p);
        }
        p
    }
}

impl Solve for Disj {
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        Stream::lazy_mplus(
            LazyStream::pause(Box::new(state.clone()), self.goal_1.clone()),
            LazyStream::pause(Box::new(state), self.goal_2.clone()),
        )
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DFSDisj {
    pub goal_1: DFSGoal,
    pub goal_2: DFSGoal,
}

impl DFSDisj {
    pub fn new(goal_1: DFSGoal, goal_2: DFSGoal) -> DFSGoal {
        DFSGoal::Dynamic(Rc::new(DFSDisj { goal_1, goal_2 }))
    }

    pub fn new_raw(goal_1: DFSGoal, goal_2: DFSGoal) -> DFSDisj {
        DFSDisj { goal_1, goal_2 }
    }

    pub fn from_vec(mut v: Vec<DFSGoal>) -> DFSGoal {
        let mut p = DFSGoal::fail();
        for g in v.drain(..).rev() {
            p = DFSDisj::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[DFSGoal]) -> DFSGoal {
        let mut p = DFSGoal::fail();
        for g in goals.to_vec().drain(..).rev() {
            p = DFSDisj::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a disjunction
    // of conjunctions.
    pub fn from_conjunctions(conjunctions: &[&[DFSGoal]]) -> DFSGoal {
        let mut p = DFSGoal::fail();
        for g in conjunctions
            .iter()
            .map(|conj| DFSConj::from_array(*conj))
            .rev()
        {
            p = DFSDisj::new(g, p);
        }
        p
    }
}

impl Solve for DFSDisj {
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        Stream::lazy_mplus_dfs(
            LazyStream::pause_dfs(Box::new(state.clone()), self.goal_1.clone()),
            LazyStream::pause_dfs(Box::new(state), self.goal_2.clone()),
        )
    }
}
