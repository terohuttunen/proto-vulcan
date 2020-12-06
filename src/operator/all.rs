use crate::goal::{Goal, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;
use std::rc::Rc;

#[derive(Debug)]
pub struct All<U: User> {
    goal_1: Goal<U>,
    goal_2: Goal<U>,
}

impl<U: User> All<U> {
    pub fn new(goal_1: Goal<U>, goal_2: Goal<U>) -> Goal<U> {
        Rc::new(All { goal_1, goal_2 })
    }

    pub fn from_vec(mut v: Vec<Goal<U>>) -> Goal<U> {
        let mut p = proto_vulcan!(true);
        for g in v.drain(..).rev() {
            p = All::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[Goal<U>]) -> Goal<U> {
        let mut p = proto_vulcan!(true);
        for g in goals.to_vec().drain(..).rev() {
            p = All::new(g, p);
        }
        p
    }

    pub fn from_iter<I>(iter: I) -> Goal<U>
    where
        I: Iterator<Item = Goal<U>>,
    {
        let mut p = proto_vulcan!(true);
        for g in iter {
            p = All::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a conjunction
    // of all the goals.
    pub fn from_conjunctions(conjunctions: &[&[Goal<U>]]) -> Goal<U> {
        let mut p = proto_vulcan!(true);
        for g in conjunctions.iter().map(|conj| All::from_array(*conj)).rev() {
            p = All::new(g, p);
        }
        p
    }
}

impl<U: User> Solver<U> for All<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let goal_1 = Rc::clone(&self.goal_1);
        let goal_2 = Rc::clone(&self.goal_2);
        let stream = Stream::Lazy(LazyStream::from_goal(goal_1, state));

        Stream::bind(stream, goal_2)
    }
}
