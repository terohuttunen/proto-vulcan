use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::operator::all::All;
use crate::operator::ForOperatorParam;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::fmt::Debug;

pub struct Everyg<U, T>
where
    U: User,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U>>,
{
    coll: T,
    g: Box<dyn Fn(LTerm<U>) -> Goal<U>>,
}

impl<U, T> Debug for Everyg<U, T>
where
    U: User,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Everyg()")
    }
}

impl<U, T> Everyg<U, T>
where
    U: User,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U>>,
{
    fn new(coll: T, g: Box<dyn Fn(LTerm<U>) -> Goal<U>>) -> Goal<U> {
        Goal::new(Everyg { coll, g })
    }
}

impl<U, T> Solve<U> for Everyg<U, T>
where
    U: User,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U>>,
{
    fn solve(&self, state: State<U>) -> Stream<U> {
        let term_iter = IntoIterator::into_iter(&self.coll);
        let goal_iter = term_iter.map(|term| (*self.g)(term.clone()));
        All::from_iter(goal_iter).solve(state)
    }
}

pub fn everyg<U, T>(param: ForOperatorParam<U, T>) -> Goal<U>
where
    U: User,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U>>,
{
    Everyg::new(param.coll, param.g)
}
