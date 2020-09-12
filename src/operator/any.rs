use crate::goal::Goal;
use crate::operator::all::All;
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::UserState;
use std::rc::Rc;

#[derive(Debug)]
pub struct Any<U: UserState> {
    goal_1: Rc<dyn Goal<U>>,
    goal_2: Rc<dyn Goal<U>>,
}

impl<U: UserState> Any<U> {
    pub fn new(goal_1: Rc<dyn Goal<U>>, goal_2: Rc<dyn Goal<U>>) -> Rc<dyn Goal<U>> {
        Rc::new(Any { goal_1, goal_2 })
    }

    pub fn from_vec(mut v: Vec<Rc<dyn Goal<U>>>) -> Rc<dyn Goal<U>> {
        let mut p = proto_vulcan!(false);
        for g in v.drain(..).rev() {
            p = Any::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[Rc<dyn Goal<U>>]) -> Rc<dyn Goal<U>> {
        let mut p = proto_vulcan!(false);
        for g in goals.to_vec().drain(..).rev() {
            p = Any::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a disjunction
    // of conjunctions.
    pub fn from_conjunctions(conjunctions: &[&[Rc<dyn Goal<U>>]]) -> Rc<dyn Goal<U>> {
        let mut p = proto_vulcan!(false);
        for g in conjunctions.iter().map(|conj| All::from_array(*conj)).rev() {
            p = Any::new(g, p);
        }
        p
    }
}

impl<U: UserState> Goal<U> for Any<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        let s1 = self.goal_1.apply(state.clone());
        let s2 = self.goal_2.apply(state);
        if s2.is_empty() {
            s1
        } else {
            Stream::mplus(s1, LazyStream::from_stream(s2))
        }
    }
}
