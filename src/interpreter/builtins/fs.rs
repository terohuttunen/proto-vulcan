//! Filesystem builtin predicates

use super::macros::{extract_single_relational, extract_two_relational, validate_args_relational};
use super::ArgumentValue;
use crate::goal::{AnyGoal, Goal, GoalCast};
use crate::lterm::{LTerm, LTermInner, LValue};
use crate::relation::fail;
use crate::solver::{Solve, Solver};
use crate::state::{Constraint, SResult, State};
use crate::stream::Stream;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Convert a Rust string to an LTerm string
fn string_to_lterm(s: String) -> LTerm {
    LTerm::from(LTermInner::Val(LValue::String(s)))
}

/// Convert a Rust integer to an LTerm number
fn int_to_lterm(n: i64) -> LTerm {
    LTerm::from(LTermInner::Val(LValue::Number(n as isize)))
}

/// Convert a boolean to an LTerm boolean
fn bool_to_lterm(b: bool) -> LTerm {
    LTerm::from(LTermInner::Val(LValue::Bool(b)))
}

/// Convert a list of strings to an LTerm list
fn string_list_to_lterm(strings: Vec<String>) -> LTerm {
    let lterms: Vec<LTerm> = strings.into_iter().map(string_to_lterm).collect();
    LTerm::from_vec(lterms)
}

/// Convert a list of bytes to an LTerm list of numbers
fn byte_list_to_lterm(bytes: Vec<u8>) -> LTerm {
    let lterms: Vec<LTerm> = bytes.into_iter().map(|b| int_to_lterm(b as i64)).collect();
    LTerm::from_vec(lterms)
}

/// For now, we'll keep builtins simple and return raw values
/// The Proto-Vulcan layer will handle FsResult wrapping
fn fs_result_ok(value: LTerm) -> LTerm {
    value
}

fn fs_result_err(message: String) -> LTerm {
    string_to_lterm(format!("ERROR: {}", message))
}

/// Extract a path string from an ArgumentValue
fn extract_path(arg: ArgumentValue) -> Result<PathBuf, Goal> {
    match arg {
        ArgumentValue::Meta(_meta) => {
            // For meta values, we'll handle differently based on RuntimeValue type
            // This is a simplified version for now
            Err(fail().cast_into())
        }
        ArgumentValue::Relational(lterm) => {
            // For relational terms, we need to walk the term and extract string
            // This is a simplified version - in practice we'd need state context
            if let LTermInner::Val(LValue::String(s)) = lterm.as_ref() {
                Ok(PathBuf::from(s.clone()))
            } else {
                Err(fail().cast_into())
            }
        }
    }
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

/// Extract a byte vector from an LTerm list in the context of a state
fn extract_bytes_from_lterm(lterm: &LTerm, state: &State) -> Option<Vec<u8>> {
    let walked = state.smap_ref().walk(lterm);
    let mut bytes = Vec::new();
    
    for item in walked.iter() {
        if let LTermInner::Val(LValue::Number(n)) = item.as_ref() {
            if *n >= 0 && *n <= 255 {
                bytes.push(*n as u8);
            } else {
                return None; // Invalid byte value
            }
        } else {
            return None; // Not a number
        }
    }
    
    Some(bytes)
}

/// Extract a string vector from an LTerm list in the context of a state
fn extract_strings_from_lterm(lterm: &LTerm, state: &State) -> Option<Vec<String>> {
    let walked = state.smap_ref().walk(lterm);
    let mut strings = Vec::new();
    
    for item in walked.iter() {
        if let LTermInner::Val(LValue::String(s)) = item.as_ref() {
            strings.push(s.clone());
        } else {
            return None; // Not a string
        }
    }
    
    Some(strings)
}

// =============================================================================
// FILE READING BUILTINS
// =============================================================================

/// Read entire file as string
pub fn read_file_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, content_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct ReadFileGoal {
        path_term: LTerm,
        content_term: LTerm,
    }

    impl Solve for ReadFileGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                match fs::read_to_string(&path_str) {
                    Ok(content) => {
                        let content_lterm = string_to_lterm(content);
                        match state.unify(&self.content_term, &content_lterm) {
                            Ok(new_state) => Stream::unit(Box::new(new_state)),
                            Err(_) => Stream::empty(),
                        }
                    }
                    Err(_) => Stream::empty(), // File read failed
                }
            } else {
                Stream::empty() // Path is not a concrete string
            }
        }
    }

    Goal::dynamic(Rc::new(ReadFileGoal {
        path_term,
        content_term,
    }))
}

/// Read file as list of bytes
pub fn read_file_bytes_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, bytes_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct ReadFileBytesGoal {
        path_term: LTerm,
        bytes_term: LTerm,
    }

    impl Solve for ReadFileBytesGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                match fs::read(&path_str) {
                    Ok(bytes) => {
                        let bytes_lterm = byte_list_to_lterm(bytes);
                        match state.unify(&self.bytes_term, &bytes_lterm) {
                            Ok(new_state) => Stream::unit(Box::new(new_state)),
                            Err(_) => Stream::empty(),
                        }
                    }
                    Err(_) => Stream::empty(),
                }
            } else {
                Stream::empty()
            }
        }
    }

    Goal::dynamic(Rc::new(ReadFileBytesGoal {
        path_term,
        bytes_term,
    }))
}

/// Read file as list of lines
pub fn read_file_lines_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, lines_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct ReadFileLinesGoal {
        path_term: LTerm,
        lines_term: LTerm,
    }

    impl Solve for ReadFileLinesGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                match fs::read_to_string(&path_str) {
                    Ok(content) => {
                        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                        let lines_lterm = string_list_to_lterm(lines);
                        match state.unify(&self.lines_term, &lines_lterm) {
                            Ok(new_state) => Stream::unit(Box::new(new_state)),
                            Err(_) => Stream::empty(),
                        }
                    }
                    Err(_) => Stream::empty(),
                }
            } else {
                Stream::empty()
            }
        }
    }

    Goal::dynamic(Rc::new(ReadFileLinesGoal {
        path_term,
        lines_term,
    }))
}

// =============================================================================
// FILE WRITING BUILTINS
// =============================================================================

/// Write string to file
pub fn write_file_builtin(args: Vec<ArgumentValue>) -> Goal {
    let lterms = match validate_args_relational(args, 3) {
        Ok(lterms) => lterms,
        Err(goal) => return goal,
    };
    
    let path_term = lterms[0].clone();
    let content_term = lterms[1].clone();
    let result_term = lterms[2].clone();

    #[derive(Debug)]
    struct WriteFileGoal {
        path_term: LTerm,
        content_term: LTerm,
        result_term: LTerm,
    }

    impl Solve for WriteFileGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            let content_walked = state.smap_ref().walk(&self.content_term);
            
            if let (Some(path_str), Some(content_str)) = (
                extract_string_from_lterm(&path_walked, &state),
                extract_string_from_lterm(&content_walked, &state)
            ) {
                let result = match fs::write(&path_str, &content_str) {
                    Ok(()) => fs_result_ok(string_to_lterm("success".to_string())),
                    Err(e) => fs_result_err(format!("Failed to write file: {}", e)),
                };
                
                match state.unify(&self.result_term, &result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                let error_result = fs_result_err("Invalid path or content".to_string());
                match state.unify(&self.result_term, &error_result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            }
        }
    }

    Goal::dynamic(Rc::new(WriteFileGoal {
        path_term,
        content_term,
        result_term,
    }))
}

/// Write bytes to file
pub fn write_file_bytes_builtin(args: Vec<ArgumentValue>) -> Goal {
    let lterms = match validate_args_relational(args, 3) {
        Ok(lterms) => lterms,
        Err(goal) => return goal,
    };
    
    let path_term = lterms[0].clone();
    let bytes_term = lterms[1].clone();
    let result_term = lterms[2].clone();

    #[derive(Debug)]
    struct WriteFileBytesGoal {
        path_term: LTerm,
        bytes_term: LTerm,
        result_term: LTerm,
    }

    impl Solve for WriteFileBytesGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            let bytes_walked = state.smap_ref().walk(&self.bytes_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                if let Some(bytes) = extract_bytes_from_lterm(&bytes_walked, &state) {
                    let result = match fs::write(&path_str, &bytes) {
                        Ok(()) => fs_result_ok(string_to_lterm("success".to_string())),
                        Err(e) => fs_result_err(format!("Failed to write file: {}", e)),
                    };
                    
                    match state.unify(&self.result_term, &result) {
                        Ok(new_state) => Stream::unit(Box::new(new_state)),
                        Err(_) => Stream::empty(),
                    }
                } else {
                    let error_result = fs_result_err("Invalid bytes format".to_string());
                    match state.unify(&self.result_term, &error_result) {
                        Ok(new_state) => Stream::unit(Box::new(new_state)),
                        Err(_) => Stream::empty(),
                    }
                }
            } else {
                let error_result = fs_result_err("Invalid path".to_string());
                match state.unify(&self.result_term, &error_result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            }
        }
    }

    Goal::dynamic(Rc::new(WriteFileBytesGoal {
        path_term,
        bytes_term,
        result_term,
    }))
}

/// Write lines to file (joins with newlines)
pub fn write_file_lines_builtin(args: Vec<ArgumentValue>) -> Goal {
    let lterms = match validate_args_relational(args, 3) {
        Ok(lterms) => lterms,
        Err(goal) => return goal,
    };
    
    let path_term = lterms[0].clone();
    let lines_term = lterms[1].clone();
    let result_term = lterms[2].clone();

    #[derive(Debug)]
    struct WriteFileLinesGoal {
        path_term: LTerm,
        lines_term: LTerm,
        result_term: LTerm,
    }

    impl Solve for WriteFileLinesGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            let lines_walked = state.smap_ref().walk(&self.lines_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                if let Some(lines) = extract_strings_from_lterm(&lines_walked, &state) {
                    // Join lines with newlines
                    let content = lines.join("\n");
                    
                    let result = match fs::write(&path_str, &content) {
                        Ok(()) => fs_result_ok(string_to_lterm("success".to_string())),
                        Err(e) => fs_result_err(format!("Failed to write file: {}", e)),
                    };
                    
                    match state.unify(&self.result_term, &result) {
                        Ok(new_state) => Stream::unit(Box::new(new_state)),
                        Err(_) => Stream::empty(),
                    }
                } else {
                    let error_result = fs_result_err("Invalid lines format".to_string());
                    match state.unify(&self.result_term, &error_result) {
                        Ok(new_state) => Stream::unit(Box::new(new_state)),
                        Err(_) => Stream::empty(),
                    }
                }
            } else {
                let error_result = fs_result_err("Invalid path".to_string());
                match state.unify(&self.result_term, &error_result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            }
        }
    }

    Goal::dynamic(Rc::new(WriteFileLinesGoal {
        path_term,
        lines_term,
        result_term,
    }))
}

/// Append string to file
pub fn append_file_builtin(args: Vec<ArgumentValue>) -> Goal {
    let lterms = match validate_args_relational(args, 3) {
        Ok(lterms) => lterms,
        Err(goal) => return goal,
    };
    
    let path_term = lterms[0].clone();
    let content_term = lterms[1].clone();
    let result_term = lterms[2].clone();

    #[derive(Debug)]
    struct AppendFileGoal {
        path_term: LTerm,
        content_term: LTerm,
        result_term: LTerm,
    }

    impl Solve for AppendFileGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            let content_walked = state.smap_ref().walk(&self.content_term);
            
            if let (Some(path_str), Some(content_str)) = (
                extract_string_from_lterm(&path_walked, &state),
                extract_string_from_lterm(&content_walked, &state)
            ) {
                // Use OpenOptions to append to file
                let result = match std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path_str)
                    .and_then(|mut file| {
                        use std::io::Write;
                        file.write_all(content_str.as_bytes())
                    })
                {
                    Ok(()) => fs_result_ok(string_to_lterm("success".to_string())),
                    Err(e) => fs_result_err(format!("Failed to append to file: {}", e)),
                };
                
                match state.unify(&self.result_term, &result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                let error_result = fs_result_err("Invalid path or content".to_string());
                match state.unify(&self.result_term, &error_result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            }
        }
    }

    Goal::dynamic(Rc::new(AppendFileGoal {
        path_term,
        content_term,
        result_term,
    }))
}

/// Copy file from source to destination
pub fn copy_file_builtin(args: Vec<ArgumentValue>) -> Goal {
    let lterms = match validate_args_relational(args, 3) {
        Ok(lterms) => lterms,
        Err(goal) => return goal,
    };
    
    let source_term = lterms[0].clone();
    let dest_term = lterms[1].clone();
    let result_term = lterms[2].clone();

    #[derive(Debug)]
    struct CopyFileGoal {
        source_term: LTerm,
        dest_term: LTerm,
        result_term: LTerm,
    }

    impl Solve for CopyFileGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let source_walked = state.smap_ref().walk(&self.source_term);
            let dest_walked = state.smap_ref().walk(&self.dest_term);
            
            if let (Some(source_str), Some(dest_str)) = (
                extract_string_from_lterm(&source_walked, &state),
                extract_string_from_lterm(&dest_walked, &state)
            ) {
                let result = match fs::copy(&source_str, &dest_str) {
                    Ok(bytes_copied) => {
                        // Return the number of bytes copied as success indicator
                        fs_result_ok(int_to_lterm(bytes_copied as i64))
                    },
                    Err(e) => fs_result_err(format!("Failed to copy file: {}", e)),
                };
                
                match state.unify(&self.result_term, &result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                let error_result = fs_result_err("Invalid source or destination path".to_string());
                match state.unify(&self.result_term, &error_result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            }
        }
    }

    Goal::dynamic(Rc::new(CopyFileGoal {
        source_term,
        dest_term,
        result_term,
    }))
}

/// Remove/delete file
pub fn remove_file_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, result_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct RemoveFileGoal {
        path_term: LTerm,
        result_term: LTerm,
    }

    impl Solve for RemoveFileGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                let result = match fs::remove_file(&path_str) {
                    Ok(()) => fs_result_ok(string_to_lterm("success".to_string())),
                    Err(e) => fs_result_err(format!("Failed to remove file: {}", e)),
                };
                
                match state.unify(&self.result_term, &result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                let error_result = fs_result_err("Invalid path".to_string());
                match state.unify(&self.result_term, &error_result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            }
        }
    }

    Goal::dynamic(Rc::new(RemoveFileGoal {
        path_term,
        result_term,
    }))
}

/// Rename/move file from old path to new path
pub fn rename_file_builtin(args: Vec<ArgumentValue>) -> Goal {
    let lterms = match validate_args_relational(args, 3) {
        Ok(lterms) => lterms,
        Err(goal) => return goal,
    };
    
    let old_path_term = lterms[0].clone();
    let new_path_term = lterms[1].clone();
    let result_term = lterms[2].clone();

    #[derive(Debug)]
    struct RenameFileGoal {
        old_path_term: LTerm,
        new_path_term: LTerm,
        result_term: LTerm,
    }

    impl Solve for RenameFileGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let old_path_walked = state.smap_ref().walk(&self.old_path_term);
            let new_path_walked = state.smap_ref().walk(&self.new_path_term);
            
            if let (Some(old_path_str), Some(new_path_str)) = (
                extract_string_from_lterm(&old_path_walked, &state),
                extract_string_from_lterm(&new_path_walked, &state)
            ) {
                let result = match fs::rename(&old_path_str, &new_path_str) {
                    Ok(()) => fs_result_ok(string_to_lterm("success".to_string())),
                    Err(e) => fs_result_err(format!("Failed to rename file: {}", e)),
                };
                
                match state.unify(&self.result_term, &result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                let error_result = fs_result_err("Invalid old or new path".to_string());
                match state.unify(&self.result_term, &error_result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            }
        }
    }

    Goal::dynamic(Rc::new(RenameFileGoal {
        old_path_term,
        new_path_term,
        result_term,
    }))
}

/// Get file size in bytes
pub fn file_size_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, size_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct FileSizeGoal {
        path_term: LTerm,
        size_term: LTerm,
    }

    impl Solve for FileSizeGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                let result = match fs::metadata(&path_str) {
                    Ok(metadata) => {
                        let size = metadata.len();
                        int_to_lterm(size as i64)
                    },
                    Err(e) => {
                        // Return error as negative number or string
                        return Stream::empty(); // File doesn't exist or can't access
                    }
                };
                
                match state.unify(&self.size_term, &result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                Stream::empty() // Path is not concrete
            }
        }
    }

    Goal::dynamic(Rc::new(FileSizeGoal {
        path_term,
        size_term,
    }))
}

/// List directory contents as list of entry names
pub fn read_dir_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, entries_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct ReadDirGoal {
        path_term: LTerm,
        entries_term: LTerm,
    }

    impl Solve for ReadDirGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                match fs::read_dir(&path_str) {
                    Ok(entries) => {
                        let mut entry_names = Vec::new();
                        for entry in entries {
                            if let Ok(entry) = entry {
                                if let Some(name) = entry.file_name().to_str() {
                                    entry_names.push(name.to_string());
                                }
                            }
                        }
                        let entries_lterm = string_list_to_lterm(entry_names);
                        match state.unify(&self.entries_term, &entries_lterm) {
                            Ok(new_state) => Stream::unit(Box::new(new_state)),
                            Err(_) => Stream::empty(),
                        }
                    },
                    Err(_) => Stream::empty(), // Directory doesn't exist or can't read
                }
            } else {
                Stream::empty() // Path is not concrete
            }
        }
    }

    Goal::dynamic(Rc::new(ReadDirGoal {
        path_term,
        entries_term,
    }))
}

/// Create directory
pub fn create_dir_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, result_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct CreateDirGoal {
        path_term: LTerm,
        result_term: LTerm,
    }

    impl Solve for CreateDirGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                let result = match fs::create_dir(&path_str) {
                    Ok(()) => fs_result_ok(string_to_lterm("success".to_string())),
                    Err(e) => fs_result_err(format!("Failed to create directory: {}", e)),
                };
                
                match state.unify(&self.result_term, &result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                let error_result = fs_result_err("Invalid path".to_string());
                match state.unify(&self.result_term, &error_result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            }
        }
    }

    Goal::dynamic(Rc::new(CreateDirGoal {
        path_term,
        result_term,
    }))
}

/// Create directory and all parent directories
pub fn create_dir_all_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, result_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct CreateDirAllGoal {
        path_term: LTerm,
        result_term: LTerm,
    }

    impl Solve for CreateDirAllGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                let result = match fs::create_dir_all(&path_str) {
                    Ok(()) => fs_result_ok(string_to_lterm("success".to_string())),
                    Err(e) => fs_result_err(format!("Failed to create directory: {}", e)),
                };
                
                match state.unify(&self.result_term, &result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                let error_result = fs_result_err("Invalid path".to_string());
                match state.unify(&self.result_term, &error_result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            }
        }
    }

    Goal::dynamic(Rc::new(CreateDirAllGoal {
        path_term,
        result_term,
    }))
}

/// Remove empty directory
pub fn remove_dir_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, result_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct RemoveDirGoal {
        path_term: LTerm,
        result_term: LTerm,
    }

    impl Solve for RemoveDirGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                let result = match fs::remove_dir(&path_str) {
                    Ok(()) => fs_result_ok(string_to_lterm("success".to_string())),
                    Err(e) => fs_result_err(format!("Failed to remove directory: {}", e)),
                };
                
                match state.unify(&self.result_term, &result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                let error_result = fs_result_err("Invalid path".to_string());
                match state.unify(&self.result_term, &error_result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            }
        }
    }

    Goal::dynamic(Rc::new(RemoveDirGoal {
        path_term,
        result_term,
    }))
}

/// Remove directory and all contents
pub fn remove_dir_all_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, result_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct RemoveDirAllGoal {
        path_term: LTerm,
        result_term: LTerm,
    }

    impl Solve for RemoveDirAllGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                let result = match fs::remove_dir_all(&path_str) {
                    Ok(()) => fs_result_ok(string_to_lterm("success".to_string())),
                    Err(e) => fs_result_err(format!("Failed to remove directory tree: {}", e)),
                };
                
                match state.unify(&self.result_term, &result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                let error_result = fs_result_err("Invalid path".to_string());
                match state.unify(&self.result_term, &error_result) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            }
        }
    }

    Goal::dynamic(Rc::new(RemoveDirAllGoal {
        path_term,
        result_term,
    }))
}

/// Canonicalize path (resolve . and .. components, follow symlinks)
pub fn canonicalize_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, canonical_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct CanonicalizeGoal {
        path_term: LTerm,
        canonical_term: LTerm,
    }

    impl Solve for CanonicalizeGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                match fs::canonicalize(&path_str) {
                    Ok(canonical_path) => {
                        let canonical_str = canonical_path.to_string_lossy().to_string();
                        let canonical_lterm = string_to_lterm(canonical_str);
                        match state.unify(&self.canonical_term, &canonical_lterm) {
                            Ok(new_state) => Stream::unit(Box::new(new_state)),
                            Err(_) => Stream::empty(),
                        }
                    },
                    Err(_) => Stream::empty(), // Path doesn't exist or can't canonicalize
                }
            } else {
                Stream::empty() // Path is not concrete
            }
        }
    }

    Goal::dynamic(Rc::new(CanonicalizeGoal {
        path_term,
        canonical_term,
    }))
}

/// Make path relative to a base directory
pub fn relative_path_builtin(args: Vec<ArgumentValue>) -> Goal {
    let lterms = match validate_args_relational(args, 3) {
        Ok(lterms) => lterms,
        Err(goal) => return goal,
    };
    
    let path_term = lterms[0].clone();
    let base_term = lterms[1].clone();
    let relative_term = lterms[2].clone();

    #[derive(Debug)]
    struct RelativePathGoal {
        path_term: LTerm,
        base_term: LTerm,
        relative_term: LTerm,
    }

    impl Solve for RelativePathGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            let base_walked = state.smap_ref().walk(&self.base_term);
            
            if let (Some(path_str), Some(base_str)) = (
                extract_string_from_lterm(&path_walked, &state),
                extract_string_from_lterm(&base_walked, &state)
            ) {
                let path = std::path::Path::new(&path_str);
                let base = std::path::Path::new(&base_str);
                
                match path.strip_prefix(base) {
                    Ok(relative_path) => {
                        let relative_str = relative_path.to_string_lossy().to_string();
                        let relative_lterm = string_to_lterm(relative_str);
                        match state.unify(&self.relative_term, &relative_lterm) {
                            Ok(new_state) => Stream::unit(Box::new(new_state)),
                            Err(_) => Stream::empty(),
                        }
                    },
                    Err(_) => Stream::empty(), // Path is not under base
                }
            } else {
                Stream::empty()
            }
        }
    }

    Goal::dynamic(Rc::new(RelativePathGoal {
        path_term,
        base_term,
        relative_term,
    }))
}

/// Normalize path separators for current platform
pub fn normalize_path_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, normalized_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct NormalizePathGoal {
        path_term: LTerm,
        normalized_term: LTerm,
    }

    impl Solve for NormalizePathGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                // Simple normalization: convert to PathBuf and back to string
                let path = std::path::PathBuf::from(path_str);
                let normalized_str = path.to_string_lossy().to_string();
                let normalized_lterm = string_to_lterm(normalized_str);
                
                match state.unify(&self.normalized_term, &normalized_lterm) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                Stream::empty() // Path is not concrete
            }
        }
    }

    Goal::dynamic(Rc::new(NormalizePathGoal {
        path_term,
        normalized_term,
    }))
}

// =============================================================================
// FILE STATUS BUILTINS
// =============================================================================

/// Check if file exists
pub fn file_exists_builtin(args: Vec<ArgumentValue>) -> Goal {
    let path_term = match extract_single_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct FileExistsGoal {
        path_term: LTerm,
    }

    impl Solve for FileExistsGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                if Path::new(&path_str).exists() {
                    Stream::unit(Box::new(state)) // File exists, succeed
                } else {
                    Stream::empty() // File doesn't exist, fail
                }
            } else {
                Stream::empty() // Path is not concrete
            }
        }
    }

    Goal::dynamic(Rc::new(FileExistsGoal { path_term }))
}

/// Check if path is a file
pub fn is_file_builtin(args: Vec<ArgumentValue>) -> Goal {
    let path_term = match extract_single_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct IsFileGoal {
        path_term: LTerm,
    }

    impl Solve for IsFileGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                if Path::new(&path_str).is_file() {
                    Stream::unit(Box::new(state))
                } else {
                    Stream::empty()
                }
            } else {
                Stream::empty()
            }
        }
    }

    Goal::dynamic(Rc::new(IsFileGoal { path_term }))
}

/// Check if path is a directory
pub fn is_dir_builtin(args: Vec<ArgumentValue>) -> Goal {
    let path_term = match extract_single_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct IsDirGoal {
        path_term: LTerm,
    }

    impl Solve for IsDirGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                if Path::new(&path_str).is_dir() {
                    Stream::unit(Box::new(state))
                } else {
                    Stream::empty()
                }
            } else {
                Stream::empty()
            }
        }
    }

    Goal::dynamic(Rc::new(IsDirGoal { path_term }))
}

// =============================================================================
// PATH MANIPULATION BUILTINS
// =============================================================================

/// Constraint-based join path components that supports bidirectional semantics
pub fn join_path_builtin(args: Vec<ArgumentValue>) -> Goal {
    let lterms = match validate_args_relational(args, 3) {
        Ok(lterms) => lterms,
        Err(goal) => return goal,
    };
    
    let base_term = lterms[0].clone();
    let component_term = lterms[1].clone();
    let result_term = lterms[2].clone();

    let constraint = Rc::new(JoinPathConstraint {
        base_term,
        component_term,
        result_term,
    });

    // Create goal that immediately adds the constraint
    #[derive(Debug)]
    struct JoinPathGoal {
        constraint: Rc<JoinPathConstraint>,
    }

    impl Solve for JoinPathGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::dynamic(Rc::new(JoinPathGoal { constraint }))
}

/// Constraint that maintains the relationship: join_path(Base, Component, Result)
#[derive(Debug)]
struct JoinPathConstraint {
    base_term: LTerm,
    component_term: LTerm,
    result_term: LTerm,
}

impl Constraint for JoinPathConstraint {
    fn run(self: Rc<Self>, mut state: State) -> SResult {
        let base_walked = state.smap_ref().walk(&self.base_term);
        let component_walked = state.smap_ref().walk(&self.component_term);
        let result_walked = state.smap_ref().walk(&self.result_term);

        // Extract concrete strings where available
        let base_str = extract_string_from_lterm(&base_walked, &state);
        let component_str = extract_string_from_lterm(&component_walked, &state);
        let result_str = extract_string_from_lterm(&result_walked, &state);

        match (base_str, component_str, result_str) {
            // All three grounded: verify the relationship
            (Some(base), Some(component), Some(result)) => {
                let expected_result = Path::new(&base).join(&component);
                if expected_result.to_string_lossy() == result {
                    Ok(state)
                } else {
                    Err(()) // Path join mismatch
                }
            }
            
            // Base and component grounded, result ungrounded: compute result
            (Some(base), Some(component), None) => {
                let result_path = Path::new(&base).join(&component);
                let result_lterm = string_to_lterm(result_path.to_string_lossy().to_string());
                let result_walked_clone = result_walked.clone();
                state.smap_to_mut().extend(result_walked_clone, result_lterm);
                state.run_constraints()
            }
            
            // Result and component grounded, base ungrounded: try to derive base
            (None, Some(component), Some(result)) => {
                let result_path = Path::new(&result);
                // Try to remove the component suffix to get the base
                if let Some(base_path) = try_remove_suffix(&result_path, &component) {
                    let base_lterm = string_to_lterm(base_path.to_string_lossy().to_string());
                    let base_walked_clone = base_walked.clone();
                    state.smap_to_mut().extend(base_walked_clone, base_lterm);
                    state.run_constraints()
                } else {
                    Err(()) // Cannot derive valid base path
                }
            }
            
            // Result and base grounded, component ungrounded: try to derive component
            (Some(base), None, Some(result)) => {
                let base_path = Path::new(&base);
                let result_path = Path::new(&result);
                
                // Try to get the relative path from base to result
                if let Ok(component_path) = result_path.strip_prefix(base_path) {
                    let component_lterm = string_to_lterm(component_path.to_string_lossy().to_string());
                    let component_walked_clone = component_walked.clone();
                    state.smap_to_mut().extend(component_walked_clone, component_lterm);
                    state.run_constraints()
                } else {
                    Err(()) // Result is not within base path
                }
            }
            
            // Less than two grounded: defer constraint until more information available
            _ => Ok(state.with_constraint(self)),
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.base_term.clone(), self.component_term.clone(), self.result_term.clone()]
    }
}

impl std::fmt::Display for JoinPathConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "join_path({}, {}, {})", self.base_term, self.component_term, self.result_term)
    }
}

/// Helper function to try removing a suffix component from a path to derive the base
fn try_remove_suffix(full_path: &Path, suffix: &str) -> Option<PathBuf> {
    let full_str = full_path.to_string_lossy();
    let suffix_path = Path::new(suffix);
    
    // Try to find where the suffix starts in the full path
    if full_str.ends_with(suffix) {
        let base_len = full_str.len() - suffix.len();
        if base_len > 0 {
            let base_str = &full_str[..base_len];
            // Remove trailing path separator if present
            let base_str = base_str.trim_end_matches('/').trim_end_matches('\\');
            if !base_str.is_empty() {
                return Some(PathBuf::from(base_str));
            }
        }
    }
    None
}

/// Get parent directory
pub fn parent_path_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, parent_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct ParentPathGoal {
        path_term: LTerm,
        parent_term: LTerm,
    }

    impl Solve for ParentPathGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let constraint = ParentPathConstraint::new(self.path_term.clone(), self.parent_term.clone());
            match constraint.run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::dynamic(Rc::new(ParentPathGoal {
        path_term,
        parent_term,
    }))
}

/// Get filename from path
pub fn file_name_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, name_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct FileNameGoal {
        path_term: LTerm,
        name_term: LTerm,
    }

    impl Solve for FileNameGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let constraint = FileNameConstraint::new(self.path_term.clone(), self.name_term.clone());
            match constraint.run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::dynamic(Rc::new(FileNameGoal {
        path_term,
        name_term,
    }))
}

/// Get file stem (filename without extension)
pub fn file_stem_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, stem_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct FileStemGoal {
        path_term: LTerm,
        stem_term: LTerm,
    }

    impl Solve for FileStemGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                if let Some(stem) = Path::new(&path_str).file_stem() {
                    let stem_str = stem.to_string_lossy().to_string();
                    let stem_lterm = string_to_lterm(stem_str);
                    
                    match state.unify(&self.stem_term, &stem_lterm) {
                        Ok(new_state) => Stream::unit(Box::new(new_state)),
                        Err(_) => Stream::empty(),
                    }
                } else {
                    Stream::empty() // No file stem
                }
            } else {
                Stream::empty()
            }
        }
    }

    Goal::dynamic(Rc::new(FileStemGoal {
        path_term,
        stem_term,
    }))
}

/// Get file extension
pub fn extension_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, ext_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct ExtensionGoal {
        path_term: LTerm,
        ext_term: LTerm,
    }

    impl Solve for ExtensionGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                let extension = Path::new(&path_str).extension()
                    .map(|ext| ext.to_string_lossy().to_string())
                    .unwrap_or_default(); // Empty string if no extension
                    
                let ext_lterm = string_to_lterm(extension);
                
                match state.unify(&self.ext_term, &ext_lterm) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                Stream::empty()
            }
        }
    }

    Goal::dynamic(Rc::new(ExtensionGoal {
        path_term,
        ext_term,
    }))
}

/// Split path into components
pub fn path_components_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (path_term, components_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct PathComponentsGoal {
        path_term: LTerm,
        components_term: LTerm,
    }

    impl Solve for PathComponentsGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                let path = Path::new(&path_str);
                let components: Vec<String> = path.components()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .collect();
                
                let components_lterm = string_list_to_lterm(components);
                
                match state.unify(&self.components_term, &components_lterm) {
                    Ok(new_state) => Stream::unit(Box::new(new_state)),
                    Err(_) => Stream::empty(),
                }
            } else {
                Stream::empty()
            }
        }
    }

    Goal::dynamic(Rc::new(PathComponentsGoal {
        path_term,
        components_term,
    }))
}

/// Check if path is absolute
pub fn is_absolute_path_builtin(args: Vec<ArgumentValue>) -> Goal {
    let path_term = match extract_single_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct IsAbsolutePathGoal {
        path_term: LTerm,
    }

    impl Solve for IsAbsolutePathGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let path_walked = state.smap_ref().walk(&self.path_term);
            
            if let Some(path_str) = extract_string_from_lterm(&path_walked, &state) {
                if Path::new(&path_str).is_absolute() {
                    Stream::unit(Box::new(state))
                } else {
                    Stream::empty()
                }
            } else {
                Stream::empty()
            }
        }
    }

    Goal::dynamic(Rc::new(IsAbsolutePathGoal { path_term }))
}

/// Concatenate a list of strings
pub fn string_concat_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (strings_term, result_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct StringConcatGoal {
        strings_term: LTerm,
        result_term: LTerm,
    }

    impl Solve for StringConcatGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            let strings_walked = state.smap_ref().walk(&self.strings_term);
            
            let mut result_string = String::new();
            for item in strings_walked.iter() {
                if let Some(s) = extract_string_from_lterm(&item, &state) {
                    result_string.push_str(&s);
                } else {
                    return Stream::empty(); // Non-string item in list
                }
            }
            
            let result_lterm = string_to_lterm(result_string);
            
            match state.unify(&self.result_term, &result_lterm) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::dynamic(Rc::new(StringConcatGoal {
        strings_term,
        result_term,
    }))
}

// =============================================================================
// PATH OPERATION CONSTRAINTS
// =============================================================================

#[derive(Debug)]
pub struct ParentPathConstraint {
    path_term: LTerm,
    parent_term: LTerm,
}

impl ParentPathConstraint {
    pub fn new(path_term: LTerm, parent_term: LTerm) -> Rc<dyn Constraint> {
        Rc::new(ParentPathConstraint { path_term, parent_term })
    }
}

impl Constraint for ParentPathConstraint {
    fn run(self: Rc<Self>, mut state: State) -> SResult {
        let path_walked = state.smap_ref().walk(&self.path_term);
        let parent_walked = state.smap_ref().walk(&self.parent_term);

        match (
            extract_string_from_lterm(&path_walked, &state),
            extract_string_from_lterm(&parent_walked, &state),
        ) {
            // Both grounded: verify parent relationship
            (Some(path_str), Some(parent_str)) => {
                let path = Path::new(&path_str);
                let expected_parent = if let Some(parent) = path.parent() {
                    parent.to_string_lossy().to_string()
                } else if path_str == "/" {
                    "/".to_string()
                } else {
                    return Err(()) // No parent available
                };

                if expected_parent == parent_str {
                    Ok(state)
                } else {
                    Err(()) // Parent mismatch
                }
            }
            
            // Path grounded, parent ungrounded: derive parent
            (Some(path_str), None) => {
                let path = Path::new(&path_str);
                let parent_str = if let Some(parent) = path.parent() {
                    parent.to_string_lossy().to_string()
                } else if path_str == "/" {
                    "/".to_string()
                } else {
                    return Err(()) // No parent available
                };

                let parent_lterm = string_to_lterm(parent_str);
                let parent_walked_clone = parent_walked.clone();
                state.smap_to_mut().extend(parent_walked_clone, parent_lterm);
                state.run_constraints()
            }
            
            // Parent grounded, path ungrounded: this is complex, defer for now
            (None, Some(_parent_str)) => {
                // Deriving path from parent is not generally solvable since many paths can have same parent
                // Defer the constraint until more information is available
                Ok(state.with_constraint(self))
            }
            
            // Both ungrounded: defer until one becomes grounded
            (None, None) => {
                Ok(state.with_constraint(self))
            }
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.path_term.clone(), self.parent_term.clone()]
    }
}

impl std::fmt::Display for ParentPathConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "parent_path({}, {})", self.path_term, self.parent_term)
    }
}

#[derive(Debug)]
pub struct FileNameConstraint {
    path_term: LTerm,
    name_term: LTerm,
}

impl FileNameConstraint {
    pub fn new(path_term: LTerm, name_term: LTerm) -> Rc<dyn Constraint> {
        Rc::new(FileNameConstraint { path_term, name_term })
    }
}

impl Constraint for FileNameConstraint {
    fn run(self: Rc<Self>, mut state: State) -> SResult {
        let path_walked = state.smap_ref().walk(&self.path_term);
        let name_walked = state.smap_ref().walk(&self.name_term);

        match (
            extract_string_from_lterm(&path_walked, &state),
            extract_string_from_lterm(&name_walked, &state),
        ) {
            // Both grounded: verify filename relationship
            (Some(path_str), Some(name_str)) => {
                let path = Path::new(&path_str);
                if let Some(filename) = path.file_name() {
                    let expected_name = filename.to_string_lossy().to_string();
                    if expected_name == name_str {
                        Ok(state)
                    } else {
                        Err(()) // Filename mismatch
                    }
                } else {
                    Err(()) // Path has no filename
                }
            }
            
            // Path grounded, name ungrounded: derive filename
            (Some(path_str), None) => {
                let path = Path::new(&path_str);
                if let Some(filename) = path.file_name() {
                    let name_str = filename.to_string_lossy().to_string();
                    let name_lterm = string_to_lterm(name_str);
                    let name_walked_clone = name_walked.clone();
                    state.smap_to_mut().extend(name_walked_clone, name_lterm);
                    state.run_constraints()
                } else {
                    Err(()) // Path has no filename
                }
            }
            
            // Name grounded, path ungrounded: this is complex, defer for now
            (None, Some(_name_str)) => {
                // Deriving full path from just filename is not generally solvable
                // Defer the constraint until more information is available
                Ok(state.with_constraint(self))
            }
            
            // Both ungrounded: defer until one becomes grounded
            (None, None) => {
                Ok(state.with_constraint(self))
            }
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.path_term.clone(), self.name_term.clone()]
    }
}

impl std::fmt::Display for FileNameConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "file_name({}, {})", self.path_term, self.name_term)
    }
}

