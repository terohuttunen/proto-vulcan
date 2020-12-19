//! A miniKanren-family relational logic programming language embedded in Rust.
//!
//! In addition to core miniKanren language, proto-vulcan currently supports disequality
//! constraints CLP(Tree), finite-domain constraints CLP(FD), as well as pattern-matching
//! (`match`) operator. The user can write additional relations and operators
//! as Rust functions, and even customize logic terms.
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
//! Pattern-matching operators have an additional tree-term parameter and a map from
//! patterns to goals. Each arm of the operator has a pattern or a set of patterns; if any of
//! the patterns of an arm unify with the tree-term, then the operator may produce solutions
//! from the goal of that branch if it succeeds as well.
//! ```text
//! <operator> <tree-term> {
//!     <pattern0> | <pattern1> => <goal1>,
//!     <pattern2> => <goal2>,
//!     ...
//! }
//! ```
//!
//! Relations are functions that can have any number of tree-term parameters,
//! for example: `foo(<tree-term1>, <tree-term2>)`. The tree-terms can be:
//! * Literal constants: `1234`, `"foo"`, `'c'`, `true`
//! * Variables: `a`, `b`, `c`, `d`
//! * Any-variables: `_`
//! * Proper lists: `[1, 2, 3, 'c']`
//! * Improper lists: `[first, second | rest]`
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
//! ## Match-operator
//! Pattern matching to the tree-terms is done with the `match`-operator, which corresponds to
//! miniKanren `matche`. Matche, matchu, and matcha are also available.
//! ```rust
//! # extern crate proto_vulcan;
//! # use proto_vulcan::*;
//! pub fn membero<U: User>(x: LTerm, l: LTerm) -> Goal<U> {
//!     proto_vulcan_closure!(match l {
//!         [head | _] => head == x,
//!         [_ | rest] => membero(x, rest),
//!     })
//! }
//! # fn main() {}
//! ```
//!
//! ## For-operator
//! For the same effect as miniKanren's `everyg`, proto-vulcan uses the `for`-operator, which
//! ensures that a goal `g` succeeds for all `x` in collection `coll`. The collection must be such
//! that it implements `IntoIterator<Item = &LTerm>`.
//! ```ignore
//! for x in &coll {
//!     g
//! }
//!
//! ```
//!
//! ## Embedding in Rust
//! To embed proto-vulcan in Rust, four macros are used: `proto_vulcan!`, `proto_vulcan_closure!`,
//! `proto_vulcan_query!`, and `lterm!`.
//!
//! * `proto_vulcan!(<goal>)` declares a Proto-vulcan goal, and returns a Rust
//! variable of type `Goal`.
//! * `proto_vulcan_closure!(<goal>)` declares a Proto-vulcan goal, and returns a Rust
//! variable of type `Goal`. The goal expression is evaluated lazily when the goal
//! is evaluated. The closure takes ownership of all variables referenced within the closure.
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
//! Proto-vulcan relations are implemented as Rust-functions that have `LTerm`-type
//! parameters, and `Goal` return value. Because proto-vulcan is parametrized by
//! generic `User`-type, functions must be made generic with respect to it if we want
//! to use anything other than the default `EmptyUser`. A simple function example that
//! implements a relation that succeeds when argument `s` is an empty list is declared as:
//! ```rust
//! extern crate proto_vulcan;
//! use proto_vulcan::*;
//!
//! pub fn emptyo<U: User>(s: LTerm) -> Goal<U> {
//!     proto_vulcan!([] == s)
//! }
//! # fn main() {}
//! ```
//! # Declaring operators
//! The signature of operators is different from relations. Operators have different kinds of
//! parameters, of which only `OperatorParam` and `PatternMatchOperatorParam` are of interest
//! to user; the parser generates these parameter types for regular operators and pattern-match
//! operators, respectively.
//! ```rust
//! # extern crate proto_vulcan;
//! # use proto_vulcan::*;
//! pub struct OperatorParam<'a, U: User> {
//!     pub body: &'a [&'a [Goal<U>]],
//! }
//!
//! // operator <term> {
//! //    <pattern0> | <pattern1> => <body0/1>,
//! //    <pattern2> => <body2>,
//! //    ...
//! //    _ => <body_default>,
//! // }
//! pub struct PatternMatchOperatorParam<'a, U: User> {
//!     // First goal of each arm is the match-goal
//!     pub arms: &'a [&'a [Goal<U>]],
//! }
//! ```
//! Even though the structs are identical, the first goal on each arm of
//! `PatternMatchOperatorParam` is the pattern and the match-term equality.
//!
//! For example `onceo` can be implemented as:
//! ```rust
//! extern crate proto_vulcan;
//! use proto_vulcan::*;
//! use proto_vulcan::operator::condu;
//! use proto_vulcan::operator::OperatorParam;
//!
//! pub fn onceo<U: User>(param: OperatorParam<U>) -> Goal<U> {
//!    let g = crate::operator::all::All::from_conjunctions(param.body);
//!    proto_vulcan!(condu { g })
//! }
//! # fn main() {}
//! ```
//!
//!
//! # Recursion
//! The relation-constructor calls within `proto_vulcan!`-macro are evaluated immediately when the
//! relation-constructor containing the macro is called; relations within `proto-vulcan!` are just
//! function calls. Recursive relations must instead use `proto_vulcan_closure!`-macro, that
//! puts the function calls and necessary context into a closure that will be evaluated later.
//! ```rust
//! extern crate proto_vulcan;
//! use proto_vulcan::*;
//!
//! pub fn appendo(l: LTerm, s: LTerm, ls: LTerm) -> Goal {
//!     proto_vulcan_closure!(
//!        match [l, s, ls] {
//!            [[], x, x] => ,
//!            [[x | l1], l2, [x | l3]] => appendo(l1, l2, l3),
//!        }
//!     )
//! }
//!
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
//! Proto-vulcan assumes that all arguments to relations are of type `LTerm`. If the argument
//! is not of type `LTerm`, then it must be quoted with `#`-prefix, in order to be passed
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
//! fn example() -> Goal {
//!     proto_vulcan!(
//!         fngoal |state| {
//!             // There could be more Rust here modifying the `state`
//!             proto_vulcan!(true).solve(state)
//!         }
//!     )
//! }
//! # fn main() {}
//! ```
//! See more complex example in `reification.rs` of Proto-vulcan itself.
//!
//! # User extensions
//! By defining a struct that implements the `Clone`- `Debug`- and `User`-traits, the search
//! `State`-monad can be extended with any kind of information that gets cloned along with the
//! search when it forks, and discarded when branches fail. This can be used to add additional
//! clone-on-write constraint-stores, for example. The user-defined state can be accessed wherever
//! `State` is available, such as in in `fngoal |state| { }`-functions and in constraints.
//!
//! The `User`-trait provides optional hooks that the user can implement. What hooks there
//! should be is still largely TBD.
//!
//! Another way of extending Proto-vulcan is `LTerm`s that implement `UserUnify`-trait. User
//! defined state is not available in user defined unification, as `LTerm` is not parametrized
//! by the user state type.
//!
//! [`miniKanren`]: http://minikanren.org

#[macro_use]
extern crate proto_vulcan_macros;

pub use proto_vulcan_macros::{lterm, proto_vulcan, proto_vulcan_closure};

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

pub use goal::{Goal, Solve};
pub use lterm::LTerm;
pub use lvalue::LValue;
pub use state::Constraint;
pub use user::{User, UserUnify};

// conde is the only non-built-in operator exported by default.
pub use crate::operator::conde::conde;
