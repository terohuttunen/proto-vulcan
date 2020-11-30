use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::state::State;
use crate::stream::Stream;
use crate::user::UserState;
use std::fmt::Debug;
use std::rc::Rc;

// operator { <body> }
pub struct OperatorParam<'a, U: UserState> {
    pub body: &'a [&'a [Rc<dyn Goal<U>>]],
}

// operator <term> {
//    <pattern0> | <pattern1> => <body0/1>,
//    <pattern2> => <body2>,
//    ...
//    _ => <body_default>,
// }
pub struct PatternMatchOperatorParam<'a, U: UserState> {
    // First goal of each arm is the match-goal
    pub arms: &'a [&'a [Rc<dyn Goal<U>>]],
}

// project |x, y, ...| { <body> }
pub struct ProjectOperatorParam<'a, U: UserState> {
    pub var_list: Vec<Rc<LTerm>>,
    pub body: &'a [&'a [Rc<dyn Goal<U>>]],
}

// fngoal [move]* |state| { <rust> }
pub struct FnOperatorParam<U: UserState> {
    pub f: Box<dyn Fn(State<U>) -> Stream<U>>,
}

// closure { <body> }
pub struct ClosureOperatorParam<U: UserState> {
    pub f: Box<dyn Fn() -> Rc<dyn Goal<U>>>,
}

// for x in coll { <body> }
pub struct ForOperatorParam<U, T>
where
    U: UserState,
    T: Debug + 'static,
    for<'b> &'b T: IntoIterator<Item = &'b Rc<LTerm>>,
{
    pub coll: T,
    // Goal generator: generates a goal for each cycle of the "loop" given element from the
    // collection.
    pub g: Box<dyn Fn(Rc<LTerm>) -> Rc<dyn Goal<U>>>,
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
