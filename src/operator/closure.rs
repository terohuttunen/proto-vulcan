use crate::goal::{Goal, Solver};
use crate::operator::ClosureOperatorParam;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::fmt;

pub struct Closure<U: User> {
    f: Box<dyn Fn() -> Goal<U>>,
}

impl<U: User> Closure<U> {
    pub fn new(param: ClosureOperatorParam<U>) -> Goal<U> {
        Goal::new(Closure { f: param.f })
    }
}

impl<U: User> Solver<U> for Closure<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        (*self.f)().solve(state)
    }
}

impl<U: User> fmt::Debug for Closure<U> {
    fn fmt(&self, fm: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Goals that are put into closure are typically recursive; therefore, evaluating
        // the goal here and trying to print it will end up in infinite recursion.
        write!(fm, "Closure(...)")
    }
}
