//! Environment builtin predicates

use super::macros::{extract_single_relational, extract_two_relational};
use super::ArgumentValue;
use crate::goal::{AnyGoal, Goal, GoalCast};
use crate::lterm::{LTerm, LTermInner, LValue};
use crate::relation::fail;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Convert a Rust string to an LTerm string
fn string_to_lterm(s: String) -> LTerm {
    LTerm::from(LTermInner::Val(LValue::String(s)))
}

/// Convert a list of (key, value) pairs to an LTerm list of tuples
fn env_vars_to_lterm(vars: Vec<(String, String)>) -> LTerm {
    let lterms: Vec<LTerm> = vars
        .into_iter()
        .map(|(key, value)| {
            // Create tuple as a 2-element list [key, value]
            LTerm::from_vec(vec![string_to_lterm(key), string_to_lterm(value)])
        })
        .collect();
    LTerm::from_vec(lterms)
}

/// Extract a string from an LTerm in the context of a state
fn extract_string_from_lterm(lterm: &LTerm, state: &State) -> Option<String> {
    let walked = state.smap_ref().walk(lterm);
    if let LTermInner::Val(LValue::String(s)) = walked.as_ref() {
        Some(s.clone())
    } else {
        None
    }
}

// =============================================================================
// ENVIRONMENT VARIABLE BUILTINS
// =============================================================================

/// Get environment variable value
pub fn env_var_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (name_term, value_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct EnvVarGoal {
        name_term: LTerm,
        value_term: LTerm,
    }

    impl Solve for EnvVarGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let name_walked = state.smap_ref().walk(&self.name_term);
            
            if let Some(name_str) = extract_string_from_lterm(&name_walked, &state) {
                match env::var(&name_str) {
                    Ok(value) => {
                        let value_lterm = string_to_lterm(value);
                        match state.unify(&self.value_term, &value_lterm) {
                            Ok(new_state) => Stream::unit(Box::new(new_state)),
                            Err(_) => Stream::empty(),
                        }
                    }
                    Err(_) => Stream::empty(), // Environment variable not found
                }
            } else {
                Stream::empty() // Name is not a concrete string
            }
        }
    }

    Goal::dynamic(Rc::new(EnvVarGoal {
        name_term,
        value_term,
    }))
}

// =============================================================================
// SYSTEM DIRECTORY BUILTINS
// =============================================================================

/// Get current working directory
pub fn current_dir_builtin(args: Vec<ArgumentValue>) -> Goal {
    let dir_term = match extract_single_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct CurrentDirGoal {
        dir_term: LTerm,
    }

    impl Solve for CurrentDirGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match env::current_dir() {
                Ok(path) => {
                    let path_str = path.to_string_lossy().to_string();
                    let path_lterm = string_to_lterm(path_str);
                    
                    match state.unify(&self.dir_term, &path_lterm) {
                        Ok(new_state) => Stream::unit(Box::new(new_state)),
                        Err(_) => Stream::empty(),
                    }
                }
                Err(_) => Stream::empty(), // Failed to get current directory
            }
        }
    }

    Goal::dynamic(Rc::new(CurrentDirGoal { dir_term }))
}

/// Get executable directory
pub fn exe_dir_builtin(args: Vec<ArgumentValue>) -> Goal {
    let dir_term = match extract_single_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct ExeDirGoal {
        dir_term: LTerm,
    }

    impl Solve for ExeDirGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match env::current_exe() {
                Ok(exe_path) => {
                    if let Some(parent) = exe_path.parent() {
                        let dir_str = parent.to_string_lossy().to_string();
                        let dir_lterm = string_to_lterm(dir_str);
                        
                        match state.unify(&self.dir_term, &dir_lterm) {
                            Ok(new_state) => Stream::unit(Box::new(new_state)),
                            Err(_) => Stream::empty(),
                        }
                    } else {
                        Stream::empty() // No parent directory
                    }
                }
                Err(_) => Stream::empty(), // Failed to get executable path
            }
        }
    }

    Goal::dynamic(Rc::new(ExeDirGoal { dir_term }))
}

/// Get temporary directory
pub fn temp_dir_builtin(args: Vec<ArgumentValue>) -> Goal {
    let dir_term = match extract_single_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct TempDirGoal {
        dir_term: LTerm,
    }

    impl Solve for TempDirGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let temp_path = env::temp_dir();
            let temp_str = temp_path.to_string_lossy().to_string();
            let temp_lterm = string_to_lterm(temp_str);
            
            match state.unify(&self.dir_term, &temp_lterm) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::dynamic(Rc::new(TempDirGoal { dir_term }))
}

// =============================================================================
// PATH OPERATIONS WITH ENVIRONMENT
// =============================================================================

/// Expand environment variables in path
pub fn expand_path_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, expanded_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct ExpandPathGoal {
        path_term: LTerm,
        expanded_term: LTerm,
    }

    impl Solve for ExpandPathGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                let expanded_str = expand_env_vars(&path_str);
                let expanded_lterm = string_to_lterm(expanded_str);
                
                match state.unify(&self.expanded_term, &expanded_lterm) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                Stream::empty()
            }
        }
    }

    Goal::dynamic(Rc::new(ExpandPathGoal {
        path_term,
        expanded_term,
    }))
}

/// Find executable in PATH
pub fn which_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (program_term, path_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct WhichGoal {
        program_term: LTerm,
        path_term: LTerm,
    }

    impl Solve for WhichGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let program_walked = state.smap_ref().walk(&self.program_term);
            
            if let Some(program_str) = extract_string_from_lterm(&program_walked, &state) {
                if let Some(executable_path) = find_executable(&program_str) {
                    let path_str = executable_path.to_string_lossy().to_string();
                    let path_lterm = string_to_lterm(path_str);
                    
                    match state.unify(&self.path_term, &path_lterm) {
                        Ok(new_state) => Stream::unit(Box::new(new_state)),
                        Err(_) => Stream::empty(),
                    }
                } else {
                    Stream::empty() // Program not found in PATH
                }
            } else {
                Stream::empty()
            }
        }
    }

    Goal::dynamic(Rc::new(WhichGoal {
        program_term,
        path_term,
    }))
}

// =============================================================================
// ENVIRONMENT INSPECTION BUILTINS
// =============================================================================

/// List all environment variables
pub fn env_vars_builtin(args: Vec<ArgumentValue>) -> Goal {
    let vars_term = match extract_single_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct EnvVarsGoal {
        vars_term: LTerm,
    }

    impl Solve for EnvVarsGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let vars: Vec<(String, String)> = env::vars().collect();
            let vars_lterm = env_vars_to_lterm(vars);
            
            match state.unify(&self.vars_term, &vars_lterm) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::dynamic(Rc::new(EnvVarsGoal { vars_term }))
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Expand environment variables in a string (simple $VAR and ${VAR} syntax)
fn expand_env_vars(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '$' {
            if let Some(&'{') = chars.peek() {
                // Handle ${VAR} syntax
                chars.next(); // consume '{'
                let mut var_name = String::new();
                let mut found_closing_brace = false;
                
                while let Some(ch) = chars.next() {
                    if ch == '}' {
                        found_closing_brace = true;
                        break;
                    }
                    var_name.push(ch);
                }
                
                if found_closing_brace {
                    match env::var(&var_name) {
                        Ok(value) => result.push_str(&value),
                        Err(_) => {
                            // Variable not found, keep original syntax
                            result.push('$');
                            result.push('{');
                            result.push_str(&var_name);
                            result.push('}');
                        }
                    }
                } else {
                    // Malformed ${VAR syntax, keep as is
                    result.push('$');
                    result.push('{');
                    result.push_str(&var_name);
                }
            } else {
                // Handle $VAR syntax
                let mut var_name = String::new();
                
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        var_name.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                
                if !var_name.is_empty() {
                    match env::var(&var_name) {
                        Ok(value) => result.push_str(&value),
                        Err(_) => {
                            // Variable not found, keep original syntax
                            result.push('$');
                            result.push_str(&var_name);
                        }
                    }
                } else {
                    // Just a lone $
                    result.push('$');
                }
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// Find executable in PATH
fn find_executable(program: &str) -> Option<PathBuf> {
    if let Ok(path_var) = env::var("PATH") {
        let path_separator = if cfg!(windows) { ';' } else { ':' };
        let executable_extension = if cfg!(windows) { ".exe" } else { "" };
        
        for dir in path_var.split(path_separator) {
            let mut candidate = PathBuf::from(dir);
            candidate.push(program);
            
            // Try without extension first
            if candidate.is_file() {
                return Some(candidate);
            }
            
            // Try with extension on Windows
            if !executable_extension.is_empty() {
                candidate.set_extension(executable_extension.trim_start_matches('.'));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}