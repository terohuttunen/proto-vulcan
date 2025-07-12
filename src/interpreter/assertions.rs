//! This module provides built-in assertion relations for the test framework.

use crate::engine::Engine;
use crate::goal::{Goal, GoalCast};
use crate::lterm::LTerm;
use crate::relation::{diseq, eq};
use crate::user::User;

/// A built-in relation that succeeds if two terms can be unified.
/// This is the primary assertion for checking equality in tests.
pub fn assert_eq<U, E>(term1: LTerm<U, E>, term2: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    eq(term1, term2).cast_into()
}

/// A built-in relation that succeeds if two terms can NOT be unified.
/// This is the primary assertion for checking inequality in tests.
pub fn assert_neq<U, E>(term1: LTerm<U, E>, term2: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    diseq(term1, term2).cast_into()
}
