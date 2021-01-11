use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;

#[derive(Debug)]
pub struct Fresh<U: User> {
    variables: Vec<LTerm<U>>,
    body: Goal<U>,
}

impl<U: User> Fresh<U> {
    pub fn new(variables: Vec<LTerm<U>>, body: Goal<U>) -> Goal<U> {
        Goal::new(Fresh { variables, body }) as Goal<U>
    }
}

impl<U: User> Solve<U> for Fresh<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let goal = self.body.clone();
        Stream::Lazy(LazyStream::from_goal(goal, state))
    }
}
