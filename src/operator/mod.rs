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

#[doc(hidden)]
pub mod anyo;
#[doc(hidden)]
pub mod closure;
//#[doc(hidden)]
//pub mod conda;
//#[doc(hidden)]
pub mod conde;
//#[doc(hidden)]
//pub mod condu;
#[doc(hidden)]
pub mod conj;
#[doc(hidden)]
pub mod disj;
#[doc(hidden)]
pub mod everyg;
#[doc(hidden)]
pub mod fngoal;
#[doc(hidden)]
pub mod fresh;
//#[doc(hidden)]
//pub mod matcha;
#[doc(hidden)]
pub mod matche;
//#[doc(hidden)]
//pub mod matchu;
//#[doc(hidden)]
//pub mod onceo;
#[doc(hidden)]
pub mod project;

#[doc(inline)]
pub use anyo::anyo;

//#[doc(inline)]
//pub use conda::conda;

#[doc(inline)]
pub use conde::conde;

#[doc(inline)]
pub use conde::cond;

//#[doc(inline)]
//pub use condu::condu;

//#[doc(inline)]
//pub use onceo::onceo;

#[doc(inline)]
pub use matche::matche;

//#[doc(inline)]
//pub use matchu::matchu;

//#[doc(inline)]
//pub use matcha::matcha;

#[doc(inline)]
pub use everyg::everyg;
