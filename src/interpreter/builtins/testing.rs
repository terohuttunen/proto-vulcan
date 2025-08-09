//! Testing assertion builtin predicates

use super::macros::{extract_single_relational, extract_two_relational, validate_args_relational};
use super::ArgumentValue;
use crate::goal::{Goal, GoalCast};
use crate::interpreter::assertions::{assert_eq, assert_neq, assert_bound, assert_unbound, assert_domain_size};
use crate::relation::fail;

/// assert_eq builtin - succeeds if two terms can be unified
pub fn assert_eq_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (term1, term2) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };
    assert_eq(term1, term2)
}

/// assert_neq builtin - succeeds if two terms cannot be unified
pub fn assert_neq_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (term1, term2) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };
    assert_neq(term1, term2)
}

/// assert_bound builtin - succeeds if a variable is bound
pub fn assert_bound_builtin(args: Vec<ArgumentValue>) -> Goal {
    let term = match extract_single_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };
    assert_bound(term)
}

/// assert_unbound builtin - succeeds if a variable is unbound
pub fn assert_unbound_builtin(args: Vec<ArgumentValue>) -> Goal {
    let term = match extract_single_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };
    assert_unbound(term)
}

/// assert_domain_size builtin - succeeds if a variable's domain has expected size
pub fn assert_domain_size_builtin(args: Vec<ArgumentValue>) -> Goal {
    let lterms = match validate_args_relational(args, 2) {
        Ok(result) => result,
        Err(goal) => return goal,
    };
    
    let term = lterms[0].clone();
    let size_term = lterms[1].clone();
    
    // Extract the expected size from the second argument
    if let Some(size_val) = size_term.get_number() {
        if size_val >= 0 {
            let size = size_val as usize;
            assert_domain_size(term, size)
        } else {
            fail().cast_into()
        }
    } else {
        fail().cast_into()
    }
}