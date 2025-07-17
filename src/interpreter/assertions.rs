//! This module provides built-in assertion relations for the test framework.

use crate::engine::Engine;
use crate::goal::{AnyGoal, Goal, GoalCast};
use crate::lterm::LTerm;
use crate::relation::{diseq, eq, fail, succeed};
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use derivative::Derivative;
use std::rc::Rc;

/// A built-in relation that succeeds if two terms can be unified.
/// This is the primary assertion for checking equality in tests.
pub fn assert_eq<U, E>(term1: LTerm<U, E>, term2: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    if std::env::var("PROTO_VULCAN_DEBUG_TESTS").is_ok() {
        println!("EVALUATING assert_eq({:?}, {:?})", term1, term2);
    }
    eq(term1, term2).cast_into()
}

/// A built-in relation that succeeds if two terms can NOT be unified.
/// This is the primary assertion for checking inequality in tests.
pub fn assert_neq<U, E>(term1: LTerm<U, E>, term2: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    if std::env::var("PROTO_VULCAN_DEBUG_TESTS").is_ok() {
        println!("EVALUATING assert_neq({:?}, {:?})", term1, term2);
    }
    diseq(term1, term2).cast_into()
}

/// A struct that implements the logic for checking if a variable is bound
#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
struct AssertBoundGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    term: LTerm<U, E>,
}

impl<U, E> Solve<U, E> for AssertBoundGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        let debug_enabled = std::env::var("PROTO_VULCAN_DEBUG_TESTS").is_ok();

        if debug_enabled {
            println!("EVALUATING assert_bound({:?})", self.term);
        }

        if self.term.is_var() {
            // Check if the variable is bound by walking the substitution map
            let walked = state.smap_ref().walk(&self.term);

            // Also check if variable has a domain in the domain store (constraint domain binding)
            let has_domain = state.dstore_ref().contains_key(&walked);

            if debug_enabled {
                println!("   - Variable: {:?}", self.term);
                println!("   - After walk: {:?}", walked);
                println!("   - Pointer equal: {}", LTerm::ptr_eq(walked, &self.term));
                println!("   - Has domain in dstore: {}", has_domain);
            }

            // A variable is bound if:
            // 1. Walk returns a non-variable (bound in smap), OR
            // 2. It has a domain in the domain store (constrained by finite domain)
            let is_bound = !walked.is_var() || has_domain;

            if is_bound {
                if debug_enabled {
                    if !walked.is_var() {
                        println!("   PASS: Variable is bound to {:?}", walked);
                    } else {
                        println!("   PASS: Variable is constrained by finite domain");
                    }
                }
                Stream::unit(Box::new(state))
            } else {
                if debug_enabled {
                    println!("   FAIL: Variable is unbound");
                }
                Stream::empty()
            }
        } else {
            // Non-variables are always bound, so this should succeed
            if debug_enabled {
                println!("   PASS: Non-variable is always bound");
            }
            Stream::unit(Box::new(state))
        }
    }
}

/// A built-in relation that succeeds if a variable is bound (has a value).
/// This is useful for testing variable states in constraint domains.
pub fn assert_bound<U, E>(term: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    if term.is_var() {
        Goal::dynamic(Rc::new(AssertBoundGoal { term }))
    } else {
        // Non-variables are always considered "bound" (ground terms)
        succeed().cast_into()
    }
}

/// A struct that implements the logic for checking if a variable is unbound
#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
struct AssertUnboundGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    term: LTerm<U, E>,
}

impl<U, E> Solve<U, E> for AssertUnboundGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        let debug_enabled = std::env::var("PROTO_VULCAN_DEBUG_TESTS").is_ok();

        if debug_enabled {
            println!("EVALUATING assert_unbound({:?})", self.term);
        }

        if self.term.is_var() {
            // Check if the variable is unbound by walking the substitution map
            let walked = state.smap_ref().walk(&self.term);

            // Also check if variable has a domain in the domain store (constraint domain binding)
            let has_domain = state.dstore_ref().contains_key(&walked);

            if debug_enabled {
                println!("   - Variable: {:?}", self.term);
                println!("   - After walk: {:?}", walked);
                println!("   - Pointer equal: {}", LTerm::ptr_eq(walked, &self.term));
                println!("   - Has domain in dstore: {}", has_domain);
            }

            // A variable is unbound if:
            // 1. Walk returns a variable (not bound in smap), AND
            // 2. It has no domain in the domain store (not constrained by finite domain)
            let is_unbound = walked.is_var() && !has_domain;

            if is_unbound {
                if debug_enabled {
                    println!("   PASS: Variable is unbound");
                }
                Stream::unit(Box::new(state))
            } else {
                if debug_enabled {
                    if !walked.is_var() {
                        println!("   FAIL: Variable is bound to {:?}", walked);
                    } else {
                        println!("   FAIL: Variable is constrained by finite domain");
                    }
                }
                Stream::empty()
            }
        } else {
            // Non-variables are always bound, so this should fail
            if debug_enabled {
                println!("   FAIL: Non-variable is always bound");
            }
            Stream::empty()
        }
    }
}

/// A built-in relation that succeeds if a variable is unbound (no value).
/// This is the complement to assert_bound.
pub fn assert_unbound<U, E>(term: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    if term.is_var() {
        Goal::dynamic(Rc::new(AssertUnboundGoal { term }))
    } else {
        // Non-variables are always bound, so this should fail
        fail().cast_into()
    }
}

/// A struct that implements the logic for checking domain size
#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
struct AssertDomainSizeGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    term: LTerm<U, E>,
    expected_size: usize,
}

impl<U, E> Solve<U, E> for AssertDomainSizeGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        let debug_enabled = std::env::var("PROTO_VULCAN_DEBUG_TESTS").is_ok();

        if debug_enabled {
            println!(
                "EVALUATING assert_domain_size({:?}, {})",
                self.term, self.expected_size
            );
        }

        if self.term.is_var() {
            let walked = state.smap_ref().walk(&self.term);

            if debug_enabled {
                println!("   - Variable: {:?}", self.term);
                println!("   - After walk: {:?}", walked);
                println!("   - Is variable after walk: {}", walked.is_var());
            }

            if walked.is_var() {
                // Check if the variable has a domain in the domain store
                if let Some(domain) = state.dstore_ref().get(&walked) {
                    let actual_size = match domain.as_ref() {
                        crate::state::FiniteDomain::Interval(range) => {
                            (range.end() - range.start()).saturating_add(1) as usize
                        }
                        crate::state::FiniteDomain::Sparse(vec) => vec.len(),
                    };

                    if debug_enabled {
                        println!("   - Domain found with size: {}", actual_size);
                        println!("   - Expected size: {}", self.expected_size);
                    }

                    if actual_size == self.expected_size {
                        if debug_enabled {
                            println!("   PASS: Domain size matches expected");
                        }
                        Stream::unit(Box::new(state))
                    } else {
                        if debug_enabled {
                            println!(
                                "   FAIL: Domain size {} does not match expected {}",
                                actual_size, self.expected_size
                            );
                        }
                        Stream::empty()
                    }
                } else {
                    if debug_enabled {
                        println!("   FAIL: No domain found for variable");
                    }
                    Stream::empty()
                }
            } else {
                // Variable is bound to a concrete value
                if debug_enabled {
                    println!("   - Variable is bound to concrete value: {:?}", walked);
                }

                if self.expected_size == 1 {
                    if debug_enabled {
                        println!("   PASS: Concrete value has domain size 1");
                    }
                    Stream::unit(Box::new(state))
                } else {
                    if debug_enabled {
                        println!(
                            "   FAIL: Concrete value has domain size 1, expected {}",
                            self.expected_size
                        );
                    }
                    Stream::empty()
                }
            }
        } else {
            // Non-variables have domain size 1 (they are concrete values)
            if debug_enabled {
                println!("   - Non-variable term: {:?}", self.term);
            }

            if self.expected_size == 1 {
                if debug_enabled {
                    println!("   PASS: Non-variable has domain size 1");
                }
                Stream::unit(Box::new(state))
            } else {
                if debug_enabled {
                    println!(
                        "   FAIL: Non-variable has domain size 1, expected {}",
                        self.expected_size
                    );
                }
                Stream::empty()
            }
        }
    }
}

/// A built-in relation that succeeds if a variable's constraint domain has the expected size.
/// This is useful for testing constraint domain behavior.
pub fn assert_domain_size<U, E>(term: LTerm<U, E>, expected_size: usize) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Goal::dynamic(Rc::new(AssertDomainSizeGoal {
        term,
        expected_size,
    }))
}
