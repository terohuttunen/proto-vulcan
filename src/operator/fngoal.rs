//! # Embeds Rust into Proto-vulcan
//!
//! Sometimes it is useful to write goals in Rust and embed them in Proto-vulcan with the built-in
//! operator `fngoal |state| { <rust-code> }`, where `state` is the current value of the
//! `State`-monad. The function must return a `Stream<U, E>`, that can be obtained by applying the
//! returned goal to the input state. For example, a goal that always succeeds, can be written as:
//! ```rust
//! # extern crate proto_vulcan;
//! # use proto_vulcan::prelude::*;
//! fn example() -> Goal {
//!     proto_vulcan!(
//!         fngoal |engine, state| {
//!             // There could be more Rust here modifying the `state`
//!             let g: Goal = proto_vulcan!(true);
//!             g.solve(engine, state)
//!         }
//!     )
//! }
//! # fn main() {}
//! ```
//! See more complex example in `reification.rs` of Proto-vulcan itself.
//!
use crate::goal::{AnyGoal, InferredGoal};
use crate::operator::FnOperatorParam;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use std::fmt;
use std::rc::Rc;

pub struct FnGoal {
    f: Box<dyn Fn(&Solver, State) -> Stream>,
}

impl FnGoal {
    pub fn new<G: AnyGoal>(f: Box<dyn Fn(&Solver, State) -> Stream>) -> InferredGoal<G> {
        InferredGoal::new(G::dynamic(Rc::new(FnGoal { f })))
    }
}

impl Solve for FnGoal {
    fn solve(&self, solver: &Solver, state: State) -> Stream {
        (*self.f)(solver, state)
    }
}

impl fmt::Debug for FnGoal {
    fn fmt(&self, fm: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fm, "FnGoal()")
    }
}

pub fn fngoal<G>(param: FnOperatorParam) -> InferredGoal<G>
where
    G: AnyGoal,
{
    FnGoal::new(param.f)
}
