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
use crate::engine::Engine;
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::operator::conj::InferredConj;
use crate::operator::ForOperatorParam;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::fmt::Debug;
use std::rc::Rc;

pub struct Everyg<T, U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U, E>>,
{
    coll: T,
    g: Box<dyn Fn(LTerm<U, E>) -> G>,
}

impl<T, U, E, G> Debug for Everyg<T, U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U, E>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Everyg()")
    }
}

impl<T, U, E, G> Everyg<T, U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U, E>>,
{
    fn new(coll: T, g: Box<dyn Fn(LTerm<U, E>) -> G>) -> InferredGoal<U, E, G> {
        InferredGoal::new(G::dynamic(Rc::new(Everyg { coll, g })))
    }
}

impl<T, U, E, G> Solve<U, E> for Everyg<T, U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U, E>>,
{
    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        let term_iter = IntoIterator::into_iter(&self.coll);
        let goal_iter = term_iter.map(|term| (*self.g)(term.clone()));
        InferredConj::from_iter(goal_iter).goal.solve(solver, state)
    }
}

pub fn everyg<T, U, E, G>(param: ForOperatorParam<T, U, E, G>) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a LTerm<U, E>>,
{
    Everyg::new(param.coll, param.g)
}
