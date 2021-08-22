use crate::engine::Engine;
use crate::goal::Goal;
use crate::operator::all::All;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct Any<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub goal_1: Goal<U, E>,
    pub goal_2: Goal<U, E>,
}

impl<U, E> Any<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(goal_1: Goal<U, E>, goal_2: Goal<U, E>) -> Goal<U, E> {
        Goal::Disj(Rc::new(Any { goal_1, goal_2 }))
    }

    pub fn new_raw(goal_1: Goal<U, E>, goal_2: Goal<U, E>) -> Any<U, E> {
        Any { goal_1, goal_2 }
    }

    pub fn from_vec(mut v: Vec<Goal<U, E>>) -> Goal<U, E> {
        let mut p = proto_vulcan!(false);
        for g in v.drain(..).rev() {
            p = Any::new(g, p);
        }
        p
    }

    pub fn from_array(goals: &[Goal<U, E>]) -> Goal<U, E> {
        let mut p = proto_vulcan!(false);
        for g in goals.to_vec().drain(..).rev() {
            p = Any::new(g, p);
        }
        p
    }

    // The parameter is a list of conjunctions, and the resulting goal is a disjunction
    // of conjunctions.
    pub fn from_conjunctions(conjunctions: &[&[Goal<U, E>]]) -> Goal<U, E> {
        let mut p = proto_vulcan!(false);
        for g in conjunctions.iter().map(|conj| All::from_array(*conj)).rev() {
            p = Any::new(g, p);
        }
        p
    }
}

impl<U, E> Solve<U, E> for Any<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        let stream = self.goal_1.solve(solver, state.clone());
        let lazy = LazyStream::pause(Box::new(state), self.goal_2.clone());
        Stream::mplus(stream, lazy)
    }
}
