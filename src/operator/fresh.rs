use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct Fresh<E: Engine<U>, U: User> {
    variables: Vec<LTerm<U, E>>,
    body: Goal<U, E>,
}

impl<E: Engine<U>, U: User> Fresh<E, U> {
    pub fn new(variables: Vec<LTerm<U, E>>, body: Goal<U, E>) -> Goal<U, E> {
        Goal::dynamic(Fresh { variables, body }) as Goal<U, E>
    }
}

impl<E: Engine<U>, U: User> Solve<U, E> for Fresh<E, U> {
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        let goal = self.body.clone();
        Stream::pause(Box::new(state), goal)
    }
}
