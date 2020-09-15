//! A miniKanren-family relational logic programming language embedded in Rust.
//!
//! In addition to core miniKanren language, proto-vulcan currently provides support for
//! disequality constraints CLP(Tree), as well as finite-domain constraints CLP(FD). It also
//! provides an interface for user-defined extensions.
//!
//! The language is embedded into Rust with macros which parse the language syntax and convert it
//! into Rust.
//!
//! # Syntax overview
//! Proto-vulcan has two kinds of goal-constructors: operators and relations. Operators operate on
//! goals, and relations relate logic terms.
//!
//! Operators are written
//! with curly-bracket block with list of goals.
//! ```text
//! <operator> {
//!     <goal1>,
//!     <goal2>,
//!     ...
//! }
//! ```
//! It is up to the operator what to do with the goals; some operators compute disjunction,
//! and some conjunction.
//!
//! Relations are functions that can have any number of tree-term parameters,
//! for example: `foo(<tree-term1>, <tree-term2>)`. The tree-terms can be:
//! * Literal constants: `1234`, `"foo"`, `'c'`, `true`
//! * Variables: `a`, `b`, `c`, `d`
//! * Any-variables: `_`
//! * Proper lists: `[1, 2, 3, 'c']`
//! * Improper lists: `(a, b)`
//! * Trees: `[[1, 2], "foo", false]`
//!
//! Core miniKanren language (fresh, conde, ==) is mapped into two Proto-vulcan operators and one
//! relation.
//!
//! ## Fresh
//! Fresh logic variables are declared with `||`-operator with a familiar Rust-style syntax as:
//! ```text
//! |a, b, c| {
//!     <goal1>,
//!     <goal2>,
//!     ...
//! }
//! ```
//! where `a`, `b` and `c` are new logic variables that are visible in the block-scope. The
//! fresh-operator computes conjunction of the goals in the block body. A plain conjunction would
//! therefore be `|| {<goal1>, <goal2>, ...}`. There is also an alternative syntax for conjunctions
//! where no new logic variables are presented: `[<goal1>, <goal2>, ...]`.
//!
//! ## Conde
//! Interleaving disjunction is declared with `conde`-operator as:
//! ```text
//! conde {
//!     <goal1>,
//!     <goal2>,
//!     ...
//! }
//! ```
//!
//! ## ==
//! Equality is declared with `eq(u, v)`-relation, which has a built-in alternative syntax `u == v`.
//! ```text
//! |u, v| {
//!     u == v,
//! }
//! ```
//!
//! ## Embedding in Rust
//! To embed proto-vulcan in Rust, three macros are used: `proto_vulcan!`, `proto_vulcan_query!`,
//! and `lterm!`.
//!
//! * `proto_vulcan!(<goal>)` declares a Proto-vulcan goal, and returns a Rust
//! variable of type `Rc<dyn Goal>`.
//! * `proto_vulcan_query!(|a, b, c| { <goal> })` defines a Proto-vulcan query with query-variables
//! `a`, `b` and `c`. The returned value is a `Query`-struct, that when `run`, produces an
//! iterator that can be used to iterate over valid solutions to the logic program. The iterator
//! returns a struct with fields named after the query variables.
//! * `lterm!(<tree-term>)` declares a logic tree-term in Rust code, which can be passed to
//!    proto-vulcan program within proto_vulcan! or proto_vulcan_query!, or compared with results.
//!
//! # Example
//! A simple example of a Proto-vulcan program that declares a disjunction of query variable `q`
//! being equal to `1`, `2` or `3`.
//! ```rust
//! extern crate proto_vulcan;
//! use proto_vulcan::*;
//!
//! fn main() {
//!     let query = proto_vulcan_query!(|q| {
//!         conde {
//!             q == 1,
//!             q == 2,
//!             q == 3,
//!         }
//!     });
//!
//!     for result in query.run() {
//!         println!("q = {}", result.q);
//!     }
//! }
//! ```
//! The example program produces three solutions:
//! ```text
//! q = 1
//! q = 2
//! q = 3
//! ```
//!
//! # Declaring relations
//! Proto-vulcan relations are implemented as Rust-functions that have `&Rc<LTerm>`-type
//! parameters, and `Rc<dyn Goal>` return value. Because proto-vulcan is parametrized by
//! generic `UserState`-type, functions must be made generic with respect to it if we want
//! to use anything other than the default `EmptyUserState`. A simple function example that
//! implements a relation that succeeds when argument `s` is an empty list is declared as:
//! ```rust
//! extern crate proto_vulcan;
//! use proto_vulcan::*;
//! use std::rc::Rc;
//!
//! pub fn emptyo<U: UserState>(s: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
//!     proto_vulcan!([] == s)
//! }
//! # fn main() {}
//! ```
//! # Declaring operators
//! The signature of operators is different from relations. Operators have only one parameter
//! which is an array of arrays of goals. For example `onceo` can be implemented as:
//! ```rust
//! extern crate proto_vulcan;
//! use proto_vulcan::*;
//! use proto_vulcan::operator::condu;
//! use std::rc::Rc;
//!
//! pub fn onceo<U: UserState>(goals: &[&[Rc<dyn Goal<U>>]]) -> Rc<dyn Goal<U>> {
//!     let g = proto_vulcan::operator::all::All::from_conjunctions(goals);
//!     proto_vulcan!(condu { g })
//! }
//! # fn main() {}
//! ```
//!
//!
//! # Recursion
//! Recursive goal constructors must enclose at least the recursive call into built-in
//! `closure { <recursive-call> }` operator. Any arguments given to the function must be cloned
//! so that the closure can take the ownership. In this example we also call the
//! `emptyo(l)`-relation declared previously.
//!
//! ```rust
//! extern crate proto_vulcan;
//! use proto_vulcan::*;
//! use proto_vulcan::relation::emptyo;
//! use proto_vulcan::relation::conso;
//! use std::rc::Rc;
//!
//! pub fn appendo<U: UserState>(l: &Rc<LTerm>, s: &Rc<LTerm>, ls: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
//!     let s = Rc::clone(s);
//!     proto_vulcan!(
//!         conde {
//!             [s == ls, emptyo(l)],
//!             |a, d, res| {
//!                 conso(a, d, l),
//!                 conso(a, res, ls),
//!                 closure {
//!                     appendo(d, s, res)
//!                 }
//!             }
//!         }
//!     )
//! }
//! # fn main() {}
//! ```
//! # Constraint Logic Programming
//! ## CLP(Tree)
//! Proto-vulcan implements disequality constraint for tree-terms with built-in syntax: `x != y`.
//!
//! # Example
//! ```rust
//! extern crate proto_vulcan;
//! use proto_vulcan::*;
//! fn main() {
//!     let query = proto_vulcan_query!(|x, y| {
//!         [x, 1] != [2, y],
//!     });
//!
//!     for result in query.run() {
//!         println!("{}", result);
//!     }
//! }
//! ```
//! Because the variables are not fully constrained, they can be anything except specific values,
//! and the output of the example is:
//! ```text
//! x: _.3  where  { _.3 != 2 }
//! y: _.4  where  { _.4 != 1 }
//! ```
//!
//! ## CLP(FD)
//! Proto-vulcan implements finite-domain constraints. For disequality, a `diseqfd(x, y)`-relation
//! must be used instead of `x != y`. Other supported CLP(FD) constraints are: `distinctfd`, `ltefd`
//! `ltfd`, `plusfd`, `minusfd` and `timesfd`. Domains are assigned to variables with `infd` or
//! `infdrange`. See `n-queens`-example for code using finite-domain constraints.
//!
//! # Projection of variables
//! For projecting variables there is a built-in operator `project |x, y, z| { <body> }`, where
//! variables already declared earlier, can be projected within the operator body as specified
//! by the projection list `|x, y, z|`.
//!
//! # Quoting
//! Proto-vulcan assumes that all arguments to relations are of type `&Rc<LTerm>`. If the argument
//! is not of type `&Rc<LTerm>`, then it must be quoted with `#`-prefix, in order to be passed
//! to the relation as-is. For examples of usage, see the `n-queens` example in repository.
//!
//! # Embedding Rust into Proto-vulcan
//! Sometimes it is useful to write goals in Rust and embed them in Proto-vulcan with the built-in
//! operator `fngoal |state| { <rust-code> }`, where `state` is the current value of the
//! `State`-monad. The function must return a `Stream<U>`, that can be obtained by applying the
//! returned goal to the input state. For example, a goal that always succeeds, can be written as:
//! ```rust
//! # extern crate proto_vulcan;
//! # use proto_vulcan::*;
//! # use std::rc::Rc;
//! fn example() -> Rc<dyn Goal> {
//!     proto_vulcan!(
//!         fngoal |state| {
//!             // There could be more Rust here modifying the `state`
//!             proto_vulcan!(true).apply(state)
//!         }
//!     )
//! }
//! # fn main() {}
//! ```
//! See more complex example in `reification.rs` of Proto-vulcan itself.
//!
//! # User extensions
//! By defining a struct that implements the `Clone`- `Debug`- and `UserState`-traits, the search
//! `State`-monad can be extended with any kind of information that gets cloned along with the
//! search when it forks, and discarded when branches fail. This can be used to add additional
//! clone-on-write constraint-stores, for example. The user-defined state can be accessed wherever
//! `State` is available, such as in in `fngoal |state| { }`-functions and in constraints.
//!
//! The `UserState`-trait provides optional hooks that the user can implement. What hooks there
//! should be is still largely TBD.
//!
//! Another way of extending Proto-vulcan is `LTerm`s that implement `UserUnify`-trait. User
//! defined state is not available in user defined unification, as `LTerm` is not parametrized
//! by the user state type.
//!
//! [`miniKanren`]: http://minikanren.org

#![feature(move_ref_pattern)]

#[macro_use]
extern crate proto_vulcan_macros;

pub use proto_vulcan_macros::proto_vulcan;

#[macro_use]
extern crate derivative;

#[macro_use]
pub mod lterm;
pub mod goal;
pub mod stream;

pub mod lvalue;
pub mod operator;
pub mod relation;

pub mod state;

pub mod lresult;

pub mod user;

#[macro_use]
pub mod query;

pub use goal::Goal;
pub use lterm::LTerm;
pub use lvalue::LValue;
pub use state::Constraint;
pub use user::{UserState, UserUnify};

// conde is the only non-built-in operator exported by default.
pub use crate::operator::conde::conde;
