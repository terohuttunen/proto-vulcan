use crate::goal::{AnyGoal, InferredGoal};
use crate::operator::ClosureOperatorParam;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use std::fmt;
use std::rc::Rc;

pub struct Closure<G: AnyGoal> {
    f: Box<dyn Fn() -> G>,
}

impl<G: AnyGoal + 'static> Closure<G> {
    pub fn new(param: ClosureOperatorParam<G>) -> InferredGoal<G> {
        InferredGoal::new(G::dynamic(Rc::new(Closure { f: param.f })))
    }
}

impl<G: AnyGoal + 'static> Solve for Closure<G> {
    fn solve(&self, solver: &Solver, state: State) -> Stream {
        (*self.f)().solve(solver, state)
    }
}

impl<G: AnyGoal> fmt::Debug for Closure<G> {
    fn fmt(&self, fm: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Goals that are put into closure are typically recursive; therefore, evaluating
        // the goal here and trying to print it will end up in infinite recursion.
        write!(fm, "Closure(...)")
    }
}
