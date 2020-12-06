use crate::goal::{Goal, Solver};
use crate::lterm::LTerm;
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;
use std::rc::Rc;

#[derive(Debug)]
pub struct Fresh<U: User> {
    variables: Vec<LTerm>,
    body: Goal<U>,
}

impl<U: User> Fresh<U> {
    pub fn new(variables: Vec<LTerm>, body: Goal<U>) -> Goal<U> {
        Rc::new(Fresh { variables, body }) as Goal<U>
    }
}

impl<U: User> Solver<U> for Fresh<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let goal = Rc::clone(&self.body);
        Stream::Lazy(LazyStream::from_goal(goal, state))
    }
}
