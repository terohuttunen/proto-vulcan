//! # Operators
//!
//! The signature of operators is different from relations. Operators have different kinds of
//! parameters, of which only `OperatorParam` and `PatternMatchOperatorParam` are of interest
//! to user; the parser generates these parameter types for regular operators and pattern-match
//! operators, respectively.
//! ```rust
//! # extern crate proto_vulcan;
//! # use proto_vulcan::prelude::*;
//! # use proto_vulcan::goal::AnyGoal;
//! pub struct OperatorParam<'a, G: AnyGoal> {
//!     pub body: &'a [&'a [G]],
//! }
//!
//! // operator <term> {
//! //    <pattern0> | <pattern1> => <body0/1>,
//! //    <pattern2> => <body2>,
//! //    ...
//! //    _ => <body_default>,
//! // }
//! pub struct PatternMatchOperatorParam<'a, G: AnyGoal> {
//!     // First goal of each arm is the match-goal
//!     pub arms: &'a [&'a [G]],
//! }
//! ```
//! Even though the structs are identical, the first goal on each arm of
//! `PatternMatchOperatorParam` is the pattern and the match-term equality.
//!
//! For example `onceo` can be implemented as:
//! ```rust
//! extern crate proto_vulcan;
//! use proto_vulcan::prelude::*;
//! use proto_vulcan::operator::condu;
//! use proto_vulcan::operator::OperatorParam;
//!
//! pub fn onceo(param: OperatorParam<Goal>) -> Goal {
//!    let g = proto_vulcan::operator::conj::Conj::from_conjunctions(param.body);
//!    proto_vulcan!(condu { g })
//! }
//! # fn main() {}
//! ```
//!

use crate::goal::AnyGoal;
use crate::lterm::LTerm;
use crate::solver::Solver;
use crate::state::State;
use crate::stream::Stream;
use std::fmt::Debug;

// operator { <body> }
pub struct OperatorParam<'a, G: AnyGoal>
{
    pub body: &'a [&'a [G]],
}

impl<'a, G: AnyGoal> OperatorParam<'a, G>
{
    #[inline]
    pub fn new(body: &'a [&'a [G]]) -> OperatorParam<'a, G> {
        OperatorParam {
            body,
        }
    }
}

// operator <term> {
//    <pattern0> | <pattern1> => <body0/1>,
//    <pattern2> => <body2>,
//    ...
//    _ => <body_default>,
// }
pub struct PatternMatchOperatorParam<'a, G: AnyGoal>
{
    // First goal of each arm is the match-goal
    pub arms: &'a [&'a [G]],
}

impl<'a, G: AnyGoal> PatternMatchOperatorParam<'a, G>
{
    #[inline]
    pub fn new(arms: &'a [&'a [G]]) -> PatternMatchOperatorParam<'a, G> {
        PatternMatchOperatorParam {
            arms,
        }
    }
}

// fngoal [move]* |engine, state| { <rust> }
pub struct FnOperatorParam
{
    pub f: Box<dyn Fn(&Solver, State) -> Stream>,
}

// closure { <body> }
pub struct ClosureOperatorParam<G: AnyGoal>
{
    pub f: Box<dyn Fn() -> G>,
}

impl<G: AnyGoal> ClosureOperatorParam<G>
{
    #[inline]
    pub fn new(f: Box<dyn Fn() -> G>) -> ClosureOperatorParam<G> {
        ClosureOperatorParam {
            f,
        }
    }
}

// for x in coll { <body> }
pub struct ForOperatorParam<T, G>
where
    G: AnyGoal,
    T: Debug + 'static,
    for<'b> &'b T: IntoIterator<Item = &'b LTerm>,
{
    pub coll: T,
    // Goal generator: generates a goal for each cycle of the "loop" given element from the
    // collection.
    pub g: Box<dyn Fn(LTerm) -> G>,
}

impl<T, G> ForOperatorParam<T, G>
where
    G: AnyGoal,
    T: Debug + 'static,
    for<'b> &'b T: IntoIterator<Item = &'b LTerm>,
{
    #[inline]
    pub fn new(coll: T, g: Box<dyn Fn(LTerm) -> G>) -> ForOperatorParam<T, G> {
        ForOperatorParam { coll, g }
    }
}

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod anyo;
#[cfg(feature = "core")]
#[doc(hidden)]
pub mod closure;
#[doc(hidden)]
pub mod conda;
#[cfg(feature = "core")]
#[doc(hidden)]
pub mod conde;
#[doc(hidden)]
pub mod condu;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod conj;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod disj;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod everyg;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod fngoal;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod dfs;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod fresh;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod matcha;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod matche;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod matchu;

#[cfg(any(feature = "extras", feature = "clpfd"))]
#[doc(hidden)]
pub mod onceo;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod project;

#[cfg(feature = "core")]
#[doc(inline)]
pub use dfs::dfs;

#[cfg(feature = "core")]
#[doc(inline)]
pub use anyo::anyo;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use conda::conda;

#[cfg(feature = "core")]
#[doc(inline)]
pub use conde::conde;

#[cfg(feature = "core")]
#[doc(inline)]
pub use conde::cond;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use condu::condu;

#[cfg(any(feature = "extras", feature = "clpfd"))]
#[doc(inline)]
pub use onceo::onceo;

#[cfg(feature = "core")]
#[doc(inline)]
pub use matche::matche;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use matchu::matchu;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use matcha::matcha;

#[cfg(feature = "core")]
#[doc(inline)]
pub use everyg::everyg;
