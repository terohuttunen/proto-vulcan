//! Helper macros and utilities for builtin predicate implementation

use super::ArgumentValue;
use crate::goal::{Goal, GoalCast};
use crate::lterm::LTerm;
use crate::relation::fail;

/// Helper function to validate arity and extract relational arguments
pub fn validate_args_relational(args: Vec<ArgumentValue>, expected_arity: usize) -> Result<Vec<LTerm>, Goal> {
    if args.len() != expected_arity {
        return Err(fail().cast_into());
    }
    
    let mut lterms = Vec::new();
    for arg in args {
        match arg {
            ArgumentValue::Relational(lterm) => lterms.push(lterm),
            ArgumentValue::Meta(_) => {
                // For now, meta arguments in basic builtins are not supported
                return Err(fail().cast_into());
            }
        }
    }
    
    Ok(lterms)
}

/// Helper function to extract a single relational argument
pub fn extract_single_relational(args: Vec<ArgumentValue>) -> Result<LTerm, Goal> {
    let lterms = validate_args_relational(args, 1)?;
    Ok(lterms[0].clone())
}

/// Helper function to extract two relational arguments
pub fn extract_two_relational(args: Vec<ArgumentValue>) -> Result<(LTerm, LTerm), Goal> {
    let lterms = validate_args_relational(args, 2)?;
    Ok((lterms[0].clone(), lterms[1].clone()))
}