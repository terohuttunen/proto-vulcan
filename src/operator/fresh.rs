use crate::engine::Engine;
use crate::goal::{AnyGoal, GoalCast, InferredGoal};
use crate::lterm::LTerm;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct Fresh<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    variables: Vec<LTerm<U, E>>,
    body: InferredGoal<U, E, G>,
}

impl<U, E, G> Fresh<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    pub fn new(variables: Vec<LTerm<U, E>>, body: InferredGoal<U, E, G>) -> InferredGoal<U, E, G> {
        InferredGoal::dynamic(Fresh { variables, body })
    }
}

impl<U, E, G> Solve<U, E> for Fresh<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        let goal = self.body.clone();
        G::pause(state, goal.cast_into())
    }
}
