//! # Relations
//!
//! Proto-vulcan relations are implemented as Rust-functions that have `LTerm`-type
//! parameters, and `Goal` return value. Because proto-vulcan is parametrized by
//! generic `User`-type, functions must be made generic with respect to it if we want
//! to use anything other than the default `DefaultUser`. A simple function example that
//! implements a relation that succeeds when argument `s` is an empty list is declared as:
//! ```rust
//! extern crate proto_vulcan;
//! use proto_vulcan::prelude::*;
//!
//! pub fn emptyo<U: User, E: Engine<U>>(s: LTerm<U, E>) -> Goal<U, E> {
//!     proto_vulcan!([] == s)
//! }
//! # fn main() {}
//! ```
//! ## Recursion
//! The relation-constructor calls within `proto_vulcan!`-macro are evaluated immediately when the
//! relation-constructor containing the macro is called; relations within `proto-vulcan!` are just
//! function calls. Recursive relations must instead use `proto_vulcan_closure!`-macro, that
//! puts the function calls and necessary context into a closure that will be evaluated later.
//! ```rust
//! extern crate proto_vulcan;
//! use proto_vulcan::prelude::*;
//!
//! pub fn append<U: User, E: Engine<U>>(l: LTerm<U, E>, s: LTerm<U, E>, ls: LTerm<U, E>) -> Goal<U, E> {
//!     proto_vulcan_closure!(
//!        match [l, s, ls] {
//!            [[], x, x] => ,
//!            [[x | l1], l2, [x | l3]] => append(l1, l2, l3),
//!        }
//!     )
//! }
//!
//! # fn main() {}
//! ```
#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod always;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod append;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod cons;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod diseq;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod distinct;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod empty;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod eq;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod fail;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod first;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod member1;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod member;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod never;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod permute;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod rember;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod rest;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod succeed;

// CLP(FD)
#[cfg(feature = "clpfd")]
pub mod clpfd;

// CLP(Z)
#[cfg(feature = "clpz")]
#[doc(hidden)]
pub mod clpz;

#[cfg(feature = "core")]
#[doc(inline)]
pub use diseq::diseq;

#[cfg(feature = "core")]
#[doc(inline)]
pub use eq::eq;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use always::always;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use append::append;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use cons::cons;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use distinct::distinct;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use empty::empty;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use first::first;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use member1::member1;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use member::member;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use never::never;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use permute::permute;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use rember::rember;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use rest::rest;

#[cfg(feature = "core")]
#[doc(inline)]
pub use fail::fail;

#[cfg(feature = "core")]
#[doc(inline)]
pub use succeed::succeed;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use clpfd::diseqfd::diseqfd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use clpfd::distinctfd::distinctfd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use clpfd::infd::infd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use clpfd::infd::infdrange;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use clpfd::ltefd::ltefd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use clpfd::ltfd::ltfd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use clpfd::minusfd::minusfd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use clpfd::plusfd::plusfd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use clpfd::timesfd::timesfd;

#[cfg(feature = "clpz")]
#[doc(inline)]
pub use clpz::plusz::plusz;

#[cfg(feature = "clpz")]
#[doc(inline)]
pub use clpz::timesz::timesz;
