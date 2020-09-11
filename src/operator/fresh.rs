use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::state::{State, UserState};
use crate::stream::{LazyStream, Stream};
use std::rc::Rc;

#[derive(Debug)]
pub struct Fresh<U: UserState> {
    variables: Vec<Rc<LTerm>>,
    body: Rc<dyn Goal<U>>,
}

impl<U: UserState> Fresh<U> {
    pub fn new(variables: Vec<Rc<LTerm>>, body: Rc<dyn Goal<U>>) -> Rc<dyn Goal<U>> {
        Rc::new(Fresh { variables, body }) as Rc<dyn Goal<U>>
    }
}

impl<U: UserState> Goal<U> for Fresh<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        let goal = Rc::clone(&self.body);
        Stream::Lazy(LazyStream::from_goal(goal, state))
    }
}
