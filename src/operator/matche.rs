//! # Match-operator
//!
//! Pattern matching to the tree-terms is done with the `match`-operator, which corresponds to
//! miniKanren `matche`. Matche, matchu, and matcha are also available.
//! ```rust
//! # extern crate proto_vulcan;
//! # use proto_vulcan::prelude::*;
//! pub fn membero(x: LTerm, l: LTerm) -> Goal {
//!     proto_vulcan_closure!(match l {
//!         [head | _] => head == x,
//!         [_ | rest] => membero(x, rest),
//!     })
//! }
//! # fn main() {}
//! ```
//!

use crate::goal::{Goal, GoalCast};
use crate::operator::conde::Conde;
use crate::operator::PatternMatchOperatorParam;

pub fn matche(param: PatternMatchOperatorParam<Goal>) -> Goal {
    Conde::from_conjunctions(param.arms).cast_into()
}
