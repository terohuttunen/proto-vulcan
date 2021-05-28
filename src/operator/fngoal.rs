use crate::engine::Engine;
use crate::goal::{Goal, Solve};
use crate::operator::FnOperatorParam;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::fmt;

pub struct FnGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    f: Box<dyn Fn(&E, State<U>) -> Stream<U, E>>,
}

impl<U, E> FnGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(f: Box<dyn Fn(&E, State<U>) -> Stream<U, E>>) -> Goal<U, E> {
        Goal::new(FnGoal { f })
    }
}

impl<U, E> Solve<U, E> for FnGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, engine: &E, state: State<U>) -> Stream<U, E> {
        (*self.f)(engine, state)
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

pub fn fngoal<U, E>(param: FnOperatorParam<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    FnGoal::new(param.f)
}
