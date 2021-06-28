use crate::engine::Engine;
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;

#[derive(Debug)]
pub struct Fresh<E: Engine<U>, U: User> {
    variables: Vec<LTerm<U, E>>,
    body: Goal<U, E>,
}

impl<E: Engine<U>, U: User> Fresh<E, U> {
    pub fn new(variables: Vec<LTerm<U, E>>, body: Goal<U, E>) -> Goal<U, E> {
        Goal::new(Fresh { variables, body }) as Goal<U, E>
    }
}

impl<E: Engine<U>, U: User> Solve<U, E> for Fresh<E, U> {
    fn solve(&self, _engine: &E, state: State<U, E>) -> Stream<U, E> {
        let goal = self.body.clone();
        Stream::pause(Box::new(state), goal)
    }
}
