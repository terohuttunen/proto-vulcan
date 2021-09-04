use crate::engine::Engine;
use crate::goal::{AdaptiveGoal, Goal};
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct AdaptiveConj<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub goal_1: AdaptiveGoal<U, E>,
    pub goal_2: AdaptiveGoal<U, E>,
}

impl<U, E> AdaptiveConj<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(goal_1: AdaptiveGoal<U, E>, goal_2: AdaptiveGoal<U, E>) -> AdaptiveGoal<U, E> {
        if goal_1.is_succeed() && goal_2.is_succeed() {
            return AdaptiveGoal::Succeed;
        }
        if goal_1.is_fail() || goal_2.is_fail() {
            return AdaptiveGoal::Fail;
        }

        AdaptiveGoal::Dynamic(Rc::new(AdaptiveConj { goal_1, goal_2 }))
    }

    pub fn new_raw(goal_1: AdaptiveGoal<U, E>, goal_2: AdaptiveGoal<U, E>) -> AdaptiveConj<U, E> {
        AdaptiveConj { goal_1, goal_2 }
    }

    pub fn from_vec(mut v: Vec<AdaptiveGoal<U, E>>) -> AdaptiveGoal<U, E> {
        let mut p = AdaptiveGoal::Succeed;
        for g in v.drain(..).rev() {
            p = AdaptiveConj::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[AdaptiveGoal<U, E>]) -> AdaptiveGoal<U, E> {
        let mut p = AdaptiveGoal::Succeed;
        for g in goals.to_vec().drain(..).rev() {
            p = AdaptiveConj::new(g, p);
        }
        p
    }

    pub fn from_iter<I>(iter: I) -> AdaptiveGoal<U, E>
    where
        I: Iterator<Item = AdaptiveGoal<U, E>>,
    {
        let mut p = AdaptiveGoal::Succeed;
        for g in iter {
            p = AdaptiveConj::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a conjunction
    // of all the goals.
    pub fn from_conjunctions(conjunctions: &[&[AdaptiveGoal<U, E>]]) -> AdaptiveGoal<U, E> {
        let mut p = AdaptiveGoal::Succeed;
        for g in conjunctions
            .iter()
            .map(|conj| AdaptiveConj::from_array(*conj))
            .rev()
        {
            p = AdaptiveConj::new(g, p);
        }
        p
    }
}

impl<U, E> Solve<U, E> for AdaptiveConj<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        Stream::lazy_bind(
            LazyStream::pause(Box::new(state), self.goal_1.clone().into()),
            self.goal_2.clone().into(),
        )
    }
}
