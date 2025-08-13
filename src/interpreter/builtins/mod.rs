//! Unified builtin predicate system for Proto-Vulcan interpreter
//!
//! This module provides a centralized system for registering and managing
//! builtin predicates that can be called from .pv programs.

use super::environment::Environment;
use super::runtime::context::ArgumentValue;
use crate::goal::Goal;
use std::collections::HashMap;
use std::rc::Rc;

mod core;
mod testing;
mod macros;
mod fs;
mod env;
mod string;

/// Specification for a builtin predicate
pub struct BuiltinSpec {
    pub name: &'static str,
    pub arity: usize,
    pub func: fn(Vec<ArgumentValue>) -> Goal,
}

/// Get all builtin predicate specifications
fn get_builtin_specs() -> Vec<BuiltinSpec> {
    vec![
        // Core language builtins
        BuiltinSpec { name: "__builtin_length", arity: 2, func: core::length_builtin },
        
        // Testing builtins
        BuiltinSpec { name: "assert_eq", arity: 2, func: testing::assert_eq_builtin },
        BuiltinSpec { name: "assert_neq", arity: 2, func: testing::assert_neq_builtin },
        BuiltinSpec { name: "assert_bound", arity: 1, func: testing::assert_bound_builtin },
        BuiltinSpec { name: "assert_unbound", arity: 1, func: testing::assert_unbound_builtin },
        BuiltinSpec { name: "assert_domain_size", arity: 2, func: testing::assert_domain_size_builtin },
        
        // Filesystem builtins
        BuiltinSpec { name: "__builtin_read_file", arity: 2, func: fs::read_file_builtin },
        BuiltinSpec { name: "__builtin_read_file_bytes", arity: 2, func: fs::read_file_bytes_builtin },
        BuiltinSpec { name: "__builtin_read_file_lines", arity: 2, func: fs::read_file_lines_builtin },
        BuiltinSpec { name: "__builtin_write_file", arity: 3, func: fs::write_file_builtin },
        BuiltinSpec { name: "__builtin_write_file_bytes", arity: 3, func: fs::write_file_bytes_builtin },
        BuiltinSpec { name: "__builtin_write_file_lines", arity: 3, func: fs::write_file_lines_builtin },
        BuiltinSpec { name: "__builtin_append_file", arity: 3, func: fs::append_file_builtin },
        BuiltinSpec { name: "__builtin_copy_file", arity: 3, func: fs::copy_file_builtin },
        BuiltinSpec { name: "__builtin_remove_file", arity: 2, func: fs::remove_file_builtin },
        BuiltinSpec { name: "__builtin_rename_file", arity: 3, func: fs::rename_file_builtin },
        BuiltinSpec { name: "__builtin_file_size", arity: 2, func: fs::file_size_builtin },
        BuiltinSpec { name: "__builtin_read_dir", arity: 2, func: fs::read_dir_builtin },
        BuiltinSpec { name: "__builtin_create_dir", arity: 2, func: fs::create_dir_builtin },
        BuiltinSpec { name: "__builtin_create_dir_all", arity: 2, func: fs::create_dir_all_builtin },
        BuiltinSpec { name: "__builtin_remove_dir", arity: 2, func: fs::remove_dir_builtin },
        BuiltinSpec { name: "__builtin_remove_dir_all", arity: 2, func: fs::remove_dir_all_builtin },
        BuiltinSpec { name: "__builtin_canonicalize", arity: 2, func: fs::canonicalize_builtin },
        BuiltinSpec { name: "__builtin_relative_path", arity: 3, func: fs::relative_path_builtin },
        BuiltinSpec { name: "__builtin_normalize_path", arity: 2, func: fs::normalize_path_builtin },
        BuiltinSpec { name: "__builtin_file_exists", arity: 1, func: fs::file_exists_builtin },
        BuiltinSpec { name: "__builtin_is_file", arity: 1, func: fs::is_file_builtin },
        BuiltinSpec { name: "__builtin_is_dir", arity: 1, func: fs::is_dir_builtin },
        BuiltinSpec { name: "__builtin_join_path", arity: 3, func: fs::join_path_builtin },
        BuiltinSpec { name: "__builtin_parent_path", arity: 2, func: fs::parent_path_builtin },
        BuiltinSpec { name: "__builtin_file_name", arity: 2, func: fs::file_name_builtin },
        BuiltinSpec { name: "__builtin_file_stem", arity: 2, func: fs::file_stem_builtin },
        BuiltinSpec { name: "__builtin_extension", arity: 2, func: fs::extension_builtin },
        BuiltinSpec { name: "__builtin_path_components", arity: 2, func: fs::path_components_builtin },
        BuiltinSpec { name: "__builtin_is_absolute_path", arity: 1, func: fs::is_absolute_path_builtin },
        BuiltinSpec { name: "__builtin_string_concat", arity: 2, func: fs::string_concat_builtin },
        
        // Environment builtins
        BuiltinSpec { name: "__builtin_env_var", arity: 2, func: env::env_var_builtin },
        BuiltinSpec { name: "__builtin_current_dir", arity: 1, func: env::current_dir_builtin },
        BuiltinSpec { name: "__builtin_exe_dir", arity: 1, func: env::exe_dir_builtin },
        BuiltinSpec { name: "__builtin_temp_dir", arity: 1, func: env::temp_dir_builtin },
        BuiltinSpec { name: "__builtin_expand_path", arity: 2, func: env::expand_path_builtin },
        BuiltinSpec { name: "__builtin_which", arity: 2, func: env::which_builtin },
        BuiltinSpec { name: "__builtin_env_vars", arity: 1, func: env::env_vars_builtin },
        BuiltinSpec { name: "__builtin_argv", arity: 1, func: env::argv_builtin },
        
        // String manipulation builtins
        BuiltinSpec { name: "__builtin_string_to_chars", arity: 2, func: string::string_to_chars_builtin },
        BuiltinSpec { name: "__builtin_chars_to_string", arity: 2, func: string::chars_to_string_builtin },
        BuiltinSpec { name: "__builtin_char_to_digit", arity: 2, func: string::char_to_digit_builtin },
        BuiltinSpec { name: "__builtin_digit_to_char", arity: 2, func: string::digit_to_char_builtin },
        BuiltinSpec { name: "__builtin_string_to_int", arity: 2, func: string::string_to_int_builtin },
        BuiltinSpec { name: "__builtin_int_to_string", arity: 2, func: string::int_to_string_builtin },
        BuiltinSpec { name: "__builtin_string_length", arity: 2, func: string::string_length_builtin },
        BuiltinSpec { name: "__builtin_is_digit", arity: 1, func: string::is_digit_builtin },
        BuiltinSpec { name: "__builtin_is_alpha", arity: 1, func: string::is_alpha_builtin },
        BuiltinSpec { name: "__builtin_is_alnum", arity: 1, func: string::is_alnum_builtin },
        BuiltinSpec { name: "__builtin_string_split", arity: 3, func: string::string_split_builtin },
        BuiltinSpec { name: "__builtin_string_split_lines", arity: 2, func: string::string_split_lines_builtin },
        BuiltinSpec { name: "__builtin_string_join", arity: 3, func: string::string_join_builtin },
    ]
}

/// Create a registry of builtin predicate names and their arities for validation
pub fn get_builtin_registry() -> std::collections::HashMap<String, usize> {
    let mut registry = std::collections::HashMap::new();
    
    for spec in get_builtin_specs() {
        registry.insert(spec.name.to_string(), spec.arity);
    }
    
    registry
}

/// Check if a builtin predicate exists in the environment and has the correct arity
pub fn validate_builtin(env: &Environment, name: &str, arity: usize) -> Result<(), String> {
    // For now, just check if the predicate exists in the environment
    match env.lookup(name) {
        Some(_entry) => {
            // TODO: Add proper arity checking when RuntimeValue supports it
            Ok(())
        }
        None => Err(format!("Unknown builtin predicate '{}'", name)),
    }
}

/// Register all builtin predicates with the environment
pub fn register_builtins(env: &mut Environment) {
    let specs = get_builtin_specs();
    
    for spec in specs {
        env.add_builtin_relation(spec.name.to_string(), Rc::new(spec.func), spec.arity);
    }
}