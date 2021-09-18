//! # Match-operator
//!
//! Pattern matching to the tree-terms is done with the `match`-operator, which corresponds to
//! miniKanren `matche`. Matche, matchu, and matcha are also available.
//! ```rust
//! # extern crate proto_vulcan;
//! # use proto_vulcan::prelude::*;
//! pub fn membero<U: User, E: Engine<U>>(x: LTerm<U, E>, l: LTerm<U, E>) -> Goal<U, E> {
//!     proto_vulcan_closure!(match l {
//!         [head | _] => head == x,
//!         [_ | rest] => membero(x, rest),
//!     })
//! }
//! # fn main() {}
//! ```
//!

use crate::engine::Engine;
use crate::goal::{Goal, GoalCast};
use crate::operator::conde::Conde;
use crate::operator::PatternMatchOperatorParam;
use crate::user::User;

pub fn matche<U, E>(param: PatternMatchOperatorParam<U, E, Goal<U, E>>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Conde::from_conjunctions(param.arms).cast_into()
}
