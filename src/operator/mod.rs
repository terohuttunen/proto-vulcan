use crate::engine::Engine;
use crate::goal::AnyGoal;
use crate::lterm::LTerm;
use crate::solver::Solver;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::fmt::Debug;
use std::marker::PhantomData;

// operator { <body> }
pub struct OperatorParam<'a, U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    pub body: &'a [&'a [G]],
    _phantom: PhantomData<U>,
    _phantom2: PhantomData<E>,
}

impl<'a, U, E, G> OperatorParam<'a, U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    #[inline]
    pub fn new(body: &'a [&'a [G]]) -> OperatorParam<'a, U, E, G> {
        OperatorParam {
            body,
            _phantom: PhantomData,
            _phantom2: PhantomData,
        }
    }
}

// operator <term> {
//    <pattern0> | <pattern1> => <body0/1>,
//    <pattern2> => <body2>,
//    ...
//    _ => <body_default>,
// }
pub struct PatternMatchOperatorParam<'a, U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    // First goal of each arm is the match-goal
    pub arms: &'a [&'a [G]],
    _phantom: PhantomData<U>,
    _phantom2: PhantomData<E>,
}

impl<'a, U, E, G> PatternMatchOperatorParam<'a, U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    #[inline]
    pub fn new(arms: &'a [&'a [G]]) -> PatternMatchOperatorParam<'a, U, E, G> {
        PatternMatchOperatorParam {
            arms,
            _phantom: PhantomData,
            _phantom2: PhantomData,
        }
    }
}

// fngoal [move]* |engine, state| { <rust> }
pub struct FnOperatorParam<U: User, E: Engine<U>>
where
    U: User,
    E: Engine<U>,
{
    pub f: Box<dyn Fn(&Solver<U, E>, State<U, E>) -> Stream<U, E>>,
}

// closure { <body> }
pub struct ClosureOperatorParam<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    pub f: Box<dyn Fn() -> G>,
    _phantom: PhantomData<U>,
    _phantom2: PhantomData<E>,
}

impl<U, E, G> ClosureOperatorParam<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    #[inline]
    pub fn new(f: Box<dyn Fn() -> G>) -> ClosureOperatorParam<U, E, G> {
        ClosureOperatorParam {
            f,
            _phantom: PhantomData,
            _phantom2: PhantomData,
        }
    }
}

// for x in coll { <body> }
pub struct ForOperatorParam<T, U, E, G>
where
    E: Engine<U>,
    U: User,
    G: AnyGoal<U, E>,
    T: Debug + 'static,
    for<'b> &'b T: IntoIterator<Item = &'b LTerm<U, E>>,
{
    pub coll: T,
    // Goal generator: generates a goal for each cycle of the "loop" given element from the
    // collection.
    pub g: Box<dyn Fn(LTerm<U, E>) -> G>,
}

impl<T, U, E, G> ForOperatorParam<T, U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
    T: Debug + 'static,
    for<'b> &'b T: IntoIterator<Item = &'b LTerm<U, E>>,
{
    #[inline]
    pub fn new(coll: T, g: Box<dyn Fn(LTerm<U, E>) -> G>) -> ForOperatorParam<T, U, E, G> {
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

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod project;

#[cfg(feature = "core")]
#[doc(inline)]
pub use anyo::anyo;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use conda::conda;

#[cfg(feature = "core")]
#[doc(inline)]
pub use conde::conde;

#[cfg(feature = "extras")]
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
