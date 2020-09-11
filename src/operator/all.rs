use crate::goal::Goal;
use crate::state::{State, UserState};
use crate::stream::{LazyStream, Stream};
use std::rc::Rc;

#[derive(Debug)]
pub struct All<U: UserState> {
    goal_1: Rc<dyn Goal<U>>,
    goal_2: Rc<dyn Goal<U>>,
}

impl<U: UserState> All<U> {
    pub fn new(goal_1: Rc<dyn Goal<U>>, goal_2: Rc<dyn Goal<U>>) -> Rc<dyn Goal<U>> {
        Rc::new(All { goal_1, goal_2 })
    }

    pub fn from_vec(mut v: Vec<Rc<dyn Goal<U>>>) -> Rc<dyn Goal<U>> {
        let mut p = proto_vulcan!(true);
        for g in v.drain(..).rev() {
            p = All::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[Rc<dyn Goal<U>>]) -> Rc<dyn Goal<U>> {
        let mut p = proto_vulcan!(true);
        for g in goals.to_vec().drain(..).rev() {
            p = All::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a conjunction
    // of all the goals.
    pub fn from_conjunctions(conjunctions: &[&[Rc<dyn Goal<U>>]]) -> Rc<dyn Goal<U>> {
        let mut p = proto_vulcan!(true);
        for g in conjunctions.iter().map(|conj| All::from_array(*conj)).rev() {
            p = All::new(g, p);
        }
        p
    }
}

impl<U: UserState> Goal<U> for All<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        let goal_1 = Rc::clone(&self.goal_1);
        let goal_2 = Rc::clone(&self.goal_2);
        let stream = Stream::Lazy(LazyStream::from_goal(goal_1, state));

        Stream::bind(stream, goal_2)
    }
}
