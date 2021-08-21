use crate::engine::Engine;
use crate::goal::{Goal, Solve};
use crate::operator::ClosureOperatorParam;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::fmt;

pub struct Closure<U, E>
where
    U: User,
    E: Engine<U>,
{
    f: Box<dyn Fn() -> Goal<U, E>>,
}

impl<U, E> Closure<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(param: ClosureOperatorParam<U, E>) -> Goal<U, E> {
        Goal::new(Closure { f: param.f })
    }
}

impl<U, E> Solve<U, E> for Closure<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, engine: &mut E, state: State<U, E>) -> Stream<U, E> {
        (*self.f)().solve(engine, state)
    }
}

impl<U, E> fmt::Debug for Closure<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn fmt(&self, fm: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Goals that are put into closure are typically recursive; therefore, evaluating
        // the goal here and trying to print it will end up in infinite recursion.
        write!(fm, "Closure(...)")
    }
}
