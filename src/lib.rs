#![doc = include_str!("../README.md")]
extern crate self as proto_vulcan;

#[macro_use]
extern crate proto_vulcan_macros;

pub use proto_vulcan_macros::{
    compound, lterm, proto_vulcan, proto_vulcan_closure, proto_vulcan_query,
};

#[macro_use]
extern crate derivative;

pub mod compound;
use compound::CompoundObject;

#[cfg(feature = "debugger")]
pub mod debugger;
pub mod engine;
pub mod goal;
pub mod lresult;
pub mod lterm;
pub mod lvalue;
pub mod operator;
pub mod query;
pub mod relation;
pub mod solver;
pub mod state;
pub mod stream;
pub mod user;

use engine::Engine;
use std::borrow::Borrow;
use user::User;

pub trait Upcast<U, E, SuperType>
where
    U: User,
    E: Engine<U>,
    Self: CompoundObject<U, E> + Clone,
    SuperType: CompoundObject<U, E>,
{
    fn to_super<K: Borrow<Self>>(k: &K) -> SuperType;

    fn into_super(self) -> SuperType;
}

pub trait Downcast<U, E>
where
    U: User,
    E: Engine<U>,
    Self: CompoundObject<U, E>,
{
    type SubType: CompoundObject<U, E>;

    fn into_sub(self) -> Self::SubType;
}

pub trait GoalCast<U, E, SuperGoal>
where
    U: User,
    E: Engine<U>,
{
    fn cast_into(self) -> SuperGoal;
}

pub mod prelude {

    pub use proto_vulcan_macros::{
        compound, lterm, proto_vulcan, proto_vulcan_closure, proto_vulcan_query,
    };

    pub use crate::compound::CompoundTerm;
    pub use crate::engine::{DefaultEngine, Engine};
    pub use crate::goal::{AnyGoal, Goal};
    pub use crate::lterm::LTerm;
    pub use crate::lvalue::LValue;
    pub use crate::solver::{Solve, Solver};
    pub use crate::state::Constraint;
    pub use crate::user::{DefaultUser, User};

    // conde is the only non-built-in operator exported by default.
    pub use crate::operator::conde::conde;
}
