//! # For-operator
//!
//! For the same effect as miniKanren's `everyg`, proto-vulcan uses the `for`-operator, which
//! ensures that a goal `g` succeeds for all `x` in collection `coll`. The collection must be such
//! that it implements `IntoIterator<Item = &LTerm>`.
//! ```ignore
//! for x in &coll {
//!     g
//! }
//!
//! ```
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::operator::conj::InferredConj;
use crate::operator::ForOperatorParam;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use std::fmt::Debug;
use std::rc::Rc;

pub struct Everyg<T, G>
where
    G: AnyGoal,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm>,
{
    coll: T,
    g: Box<dyn Fn(LTerm) -> G>,
}

impl<T, G> Debug for Everyg<T, G>
where
    G: AnyGoal,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Everyg()")
    }
}

impl<T, G> Everyg<T, G>
where
    G: AnyGoal,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm>,
{
    fn new(coll: T, g: Box<dyn Fn(LTerm) -> G>) -> InferredGoal<G> {
        InferredGoal::new(G::dynamic(Rc::new(Everyg { coll, g })))
    }
}

impl<T, G> Solve for Everyg<T, G>
where
    G: AnyGoal,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm>,
{
    fn solve(&self, solver: &Solver, state: State) -> Stream {
        let term_iter = IntoIterator::into_iter(&self.coll);
        let goal_iter = term_iter.map(|term| (*self.g)(term.clone()));
        InferredConj::from_iter(goal_iter).goal.solve(solver, state)
    }
}

pub fn everyg<T, G>(param: ForOperatorParam<T, G>) -> InferredGoal<G>
where
    G: AnyGoal,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm>,
{
    Everyg::new(param.coll, param.g)
}
