use crate::engine::Engine;
use crate::goal::Goal;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;
use std::rc::Rc;

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
            return Goal::Succeed;
        }
        if goal_1.is_fail() || goal_2.is_fail() {
            return Goal::Fail;
        }

        Goal::Dynamic(Rc::new(Conj { goal_1, goal_2 }))
    }

    pub fn new_raw(goal_1: Goal<U, E>, goal_2: Goal<U, E>) -> Conj<U, E> {
        Conj { goal_1, goal_2 }
    }

    pub fn from_vec(mut v: Vec<Goal<U, E>>) -> Goal<U, E> {
        let mut p = proto_vulcan!(true);
        for g in v.drain(..).rev() {
            p = Conj::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[Goal<U, E>]) -> Goal<U, E> {
        let mut p = proto_vulcan!(true);
        for g in goals.to_vec().drain(..).rev() {
            p = Conj::new(g, p);
        }
        p
    }

    pub fn from_iter<I>(iter: I) -> Goal<U, E>
    where
        I: Iterator<Item = Goal<U, E>>,
    {
        let mut p = proto_vulcan!(true);
        for g in iter {
            p = Conj::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a conjunction
    // of all the goals.
    pub fn from_conjunctions(conjunctions: &[&[Goal<U, E>]]) -> Goal<U, E> {
        let mut p = proto_vulcan!(true);
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
