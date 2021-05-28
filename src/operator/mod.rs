use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::fmt::Debug;

// operator { <body> }
pub struct OperatorParam<'a, U: User, E: Engine<U>> {
    pub body: &'a [&'a [Goal<U, E>]],
}

// operator <term> {
//    <pattern0> | <pattern1> => <body0/1>,
//    <pattern2> => <body2>,
//    ...
//    _ => <body_default>,
// }
pub struct PatternMatchOperatorParam<'a, U: User, E: Engine<U>> {
    // First goal of each arm is the match-goal
    pub arms: &'a [&'a [Goal<U, E>]],
}

// project |x, y, ...| { <body> }
pub struct ProjectOperatorParam<'a, U: User, E: Engine<U>> {
    pub var_list: Vec<LTerm<U>>,
    pub body: &'a [&'a [Goal<U, E>]],
}

// fngoal [move]* |engine, state| { <rust> }
pub struct FnOperatorParam<U: User, E: Engine<U>> {
    pub f: Box<dyn Fn(&E, State<U>) -> Stream<U, E>>,
}

// closure { <body> }
pub struct ClosureOperatorParam<U: User, E: Engine<U>> {
    pub f: Box<dyn Fn() -> Goal<U, E>>,
}

// for x in coll { <body> }
pub struct ForOperatorParam<T, U, E>
where
    E: Engine<U>,
    U: User,
    T: Debug + 'static,
    for<'b> &'b T: IntoIterator<Item = &'b LTerm<U>>,
{
    pub coll: T,
    // Goal generator: generates a goal for each cycle of the "loop" given element from the
    // collection.
    pub g: Box<dyn Fn(LTerm<U>) -> Goal<U, E>>,
}

#[doc(hidden)]
pub mod all;
#[doc(hidden)]
pub mod any;
#[doc(hidden)]
pub mod anyo;
#[doc(hidden)]
pub mod closure;
#[doc(hidden)]
pub mod conda;
#[doc(hidden)]
pub mod conde;
#[doc(hidden)]
pub mod condu;
#[doc(hidden)]
pub mod everyg;
#[doc(hidden)]
pub mod fngoal;
#[doc(hidden)]
pub mod fresh;
#[doc(hidden)]
pub mod matcha;
#[doc(hidden)]
pub mod matche;
#[doc(hidden)]
pub mod matchu;
#[doc(hidden)]
pub mod onceo;
#[doc(hidden)]
pub mod project;

#[doc(inline)]
pub use anyo::anyo;

#[doc(inline)]
pub use conda::conda;

#[doc(inline)]
pub use conde::conde;

#[doc(inline)]
pub use condu::condu;

#[doc(inline)]
pub use onceo::onceo;

#[doc(inline)]
pub use matche::matche;

#[doc(inline)]
pub use matchu::matchu;

#[doc(inline)]
pub use matcha::matcha;

#[doc(inline)]
pub use everyg::everyg;
