use crate::engine::Engine;
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::operator::all::All;
use crate::operator::ForOperatorParam;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::fmt::Debug;

pub struct Everyg<T, U, E>
where
    U: User,
    E: Engine<U>,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U, E>>,
{
    coll: T,
    g: Box<dyn Fn(LTerm<U, E>) -> Goal<U, E>>,
}

impl<T, U, E> Debug for Everyg<T, U, E>
where
    U: User,
    E: Engine<U>,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U, E>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Everyg()")
    }
}

impl<T, U, E> Everyg<T, U, E>
where
    U: User,
    E: Engine<U>,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U, E>>,
{
    fn new(coll: T, g: Box<dyn Fn(LTerm<U, E>) -> Goal<U, E>>) -> Goal<U, E> {
        Goal::new(Everyg { coll, g })
    }
}

impl<T, U, E> Solve<U, E> for Everyg<T, U, E>
where
    U: User,
    E: Engine<U>,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U, E>>,
{
    fn solve(&self, engine: &mut E, state: State<U, E>) -> Stream<U, E> {
        let term_iter = IntoIterator::into_iter(&self.coll);
        let goal_iter = term_iter.map(|term| (*self.g)(term.clone()));
        All::from_iter(goal_iter).solve(engine, state)
    }
}

pub fn everyg<T, U, E>(param: ForOperatorParam<T, U, E>) -> Goal<U, E>
where
    E: Engine<U>,
    U: User,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U, E>>,
{
    Everyg::new(param.coll, param.g)
}
