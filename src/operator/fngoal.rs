use crate::engine::Engine;
use crate::goal::{AnyGoal, InferredGoal};
use crate::operator::FnOperatorParam;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::fmt;

pub struct FnGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    f: Box<dyn Fn(&Solver<U, E>, State<U, E>) -> Stream<U, E>>,
}

impl<U, E> FnGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new<G: AnyGoal<U, E>>(
        f: Box<dyn Fn(&Solver<U, E>, State<U, E>) -> Stream<U, E>>,
    ) -> InferredGoal<U, E, G> {
        InferredGoal::dynamic(FnGoal { f })
    }
}

impl<U, E> Solve<U, E> for FnGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        (*self.f)(solver, state)
    }
}

impl<U, E> fmt::Debug for FnGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn fmt(&self, fm: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fm, "FnGoal()")
    }
}

pub fn fngoal<U, E, G>(param: FnOperatorParam<U, E>) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    FnGoal::new(param.f)
}
