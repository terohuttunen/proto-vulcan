//! # Embeds Rust into Proto-vulcan
//!
//! Sometimes it is useful to write goals in Rust and embed them in Proto-vulcan with the built-in
//! operator `fngoal |state| { <rust-code> }`, where `state` is the current value of the
//! `State`-monad. The function must return a `Stream<U, E>`, that can be obtained by applying the
//! returned goal to the input state. For example, a goal that always succeeds, can be written as:
//! ```rust
//! # extern crate proto_vulcan;
//! # use proto_vulcan::prelude::*;
//! fn example<U: User, E: Engine<U>>() -> Goal<U, E> {
//!     proto_vulcan!(
//!         fngoal |engine, state| {
//!             // There could be more Rust here modifying the `state`
//!             let g: Goal<U, E> = proto_vulcan!(true);
//!             g.solve(engine, state)
//!         }
//!     )
//! }
//! # fn main() {}
//! ```
//! See more complex example in `reification.rs` of Proto-vulcan itself.
//!
use crate::engine::Engine;
use crate::goal::{AnyGoal, InferredGoal};
use crate::operator::FnOperatorParam;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::fmt;
use std::rc::Rc;

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
        InferredGoal::new(G::dynamic(Rc::new(FnGoal { f })))
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
