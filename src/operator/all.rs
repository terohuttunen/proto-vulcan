use crate::engine::Engine;
use crate::goal::{Goal, Solve};
use crate::state::State;
use crate::user::User;

#[derive(Debug)]
pub struct All<U, E>
where
    U: User,
    E: Engine<U>,
{
    goal_1: Goal<U, E>,
    goal_2: Goal<U, E>,
}

impl<U, E> All<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(goal_1: Goal<U, E>, goal_2: Goal<U, E>) -> Goal<U, E> {
        Goal::new(All { goal_1, goal_2 })
    }

    pub fn from_vec(mut v: Vec<Goal<U, E>>) -> Goal<U, E> {
        let mut p = proto_vulcan!(true);
        for g in v.drain(..).rev() {
            p = All::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[Goal<U, E>]) -> Goal<U, E> {
        let mut p = proto_vulcan!(true);
        for g in goals.to_vec().drain(..).rev() {
            p = All::new(g, p);
        }
        p
    }

    pub fn from_iter<I>(iter: I) -> Goal<U, E>
    where
        I: Iterator<Item = Goal<U, E>>,
    {
        let mut p = proto_vulcan!(true);
        for g in iter {
            p = All::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a conjunction
    // of all the goals.
    pub fn from_conjunctions(conjunctions: &[&[Goal<U, E>]]) -> Goal<U, E> {
        let mut p = proto_vulcan!(true);
        for g in conjunctions.iter().map(|conj| All::from_array(*conj)).rev() {
            p = All::new(g, p);
        }
        p
    }
}

impl<U, E> Solve<U, E> for All<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, engine: &E, state: State<U>) -> E::Stream {
        let stream = engine.inc(engine.lazy(self.goal_1.clone(), state));
        engine.mbind(stream, self.goal_2.clone())
    }
}
