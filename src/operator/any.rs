use crate::goal::{Goal, Solver};
use crate::operator::all::All;
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;

#[derive(Debug)]
pub struct Any<U: User> {
    goal_1: Goal<U>,
    goal_2: Goal<U>,
}

impl<U: User> Any<U> {
    pub fn new(goal_1: Goal<U>, goal_2: Goal<U>) -> Goal<U> {
        Goal::new(Any { goal_1, goal_2 })
    }

    pub fn from_vec(mut v: Vec<Goal<U>>) -> Goal<U> {
        let mut p = proto_vulcan!(false);
        for g in v.drain(..).rev() {
            p = Any::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[Goal<U>]) -> Goal<U> {
        let mut p = proto_vulcan!(false);
        for g in goals.to_vec().drain(..).rev() {
            p = Any::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a disjunction
    // of conjunctions.
    pub fn from_conjunctions(conjunctions: &[&[Goal<U>]]) -> Goal<U> {
        let mut p = proto_vulcan!(false);
        for g in conjunctions.iter().map(|conj| All::from_array(*conj)).rev() {
            p = Any::new(g, p);
        }
        p
    }
}

impl<U: User> Solver<U> for Any<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let s1 = self.goal_1.solve(state.clone());
        let s2 = self.goal_2.solve(state);
        if s2.is_empty() {
            s1
        } else {
            Stream::mplus(s1, LazyStream::from_stream(s2))
        }
    }
}
