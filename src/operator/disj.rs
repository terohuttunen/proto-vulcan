use crate::engine::Engine;
use crate::goal::{AnyGoal, DFSGoal, Goal};
use crate::operator::conj::{Conj, DFSConj};
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct Disj<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub goal_1: Goal<U, E>,
    pub goal_2: Goal<U, E>,
}

impl<U, E> Disj<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(goal_1: Goal<U, E>, goal_2: Goal<U, E>) -> Goal<U, E> {
        Goal::Dynamic(Rc::new(Disj { goal_1, goal_2 }))
    }

    pub fn new_raw(goal_1: Goal<U, E>, goal_2: Goal<U, E>) -> Disj<U, E> {
        Disj { goal_1, goal_2 }
    }

    pub fn from_vec(mut v: Vec<Goal<U, E>>) -> Goal<U, E> {
        let mut p = Goal::fail();
        for g in v.drain(..).rev() {
            p = Disj::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[Goal<U, E>]) -> Goal<U, E> {
        let mut p = Goal::fail();
        for g in goals.to_vec().drain(..).rev() {
            p = Disj::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a disjunction
    // of conjunctions.
    pub fn from_conjunctions(conjunctions: &[&[Goal<U, E>]]) -> Goal<U, E> {
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

impl<U, E> Solve<U, E> for Disj<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        Stream::lazy_mplus(
            LazyStream::pause(Box::new(state.clone()), self.goal_1.clone()),
            LazyStream::pause(Box::new(state), self.goal_2.clone()),
        )
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct DFSDisj<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub goal_1: DFSGoal<U, E>,
    pub goal_2: DFSGoal<U, E>,
}

impl<U, E> DFSDisj<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(goal_1: DFSGoal<U, E>, goal_2: DFSGoal<U, E>) -> DFSGoal<U, E> {
        DFSGoal::Dynamic(Rc::new(DFSDisj { goal_1, goal_2 }))
    }

    pub fn new_raw(goal_1: DFSGoal<U, E>, goal_2: DFSGoal<U, E>) -> DFSDisj<U, E> {
        DFSDisj { goal_1, goal_2 }
    }

    pub fn from_vec(mut v: Vec<DFSGoal<U, E>>) -> DFSGoal<U, E> {
        let mut p = DFSGoal::fail();
        for g in v.drain(..).rev() {
            p = DFSDisj::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[DFSGoal<U, E>]) -> DFSGoal<U, E> {
        let mut p = DFSGoal::fail();
        for g in goals.to_vec().drain(..).rev() {
            p = DFSDisj::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a disjunction
    // of conjunctions.
    pub fn from_conjunctions(conjunctions: &[&[DFSGoal<U, E>]]) -> DFSGoal<U, E> {
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

impl<U, E> Solve<U, E> for DFSDisj<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        DFSGoal::mplus(state, self.goal_1.clone(), self.goal_2.clone())
    }
}
