//! Unified builtin predicate system for Proto-Vulcan interpreter
//!
//! This module provides a centralized system for registering and managing
//! builtin predicates that can be called from .pv programs.

use super::environment::Environment;
use super::runtime::context::ArgumentValue;
use crate::goal::Goal;
use std::rc::Rc;

mod core;
mod testing;
mod macros;

/// Specification for a builtin predicate
pub struct BuiltinSpec {
    pub name: &'static str,
    pub arity: usize,
    pub func: fn(Vec<ArgumentValue>) -> Goal,
}

/// Register all builtin predicates with the environment
pub fn register_builtins(env: &mut Environment) {
    let specs = [
        // Core language builtins
        BuiltinSpec { name: "__builtin_length", arity: 2, func: core::length_builtin },
        
        // Testing builtins
        BuiltinSpec { name: "assert_eq", arity: 2, func: testing::assert_eq_builtin },
        BuiltinSpec { name: "assert_neq", arity: 2, func: testing::assert_neq_builtin },
        BuiltinSpec { name: "assert_bound", arity: 1, func: testing::assert_bound_builtin },
        BuiltinSpec { name: "assert_unbound", arity: 1, func: testing::assert_unbound_builtin },
        BuiltinSpec { name: "assert_domain_size", arity: 2, func: testing::assert_domain_size_builtin },
    ];
    
    for spec in specs {
        env.add_builtin_relation(spec.name.to_string(), Rc::new(spec.func), spec.arity);
    }
}