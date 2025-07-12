//! This module contains the logic for discovering and running Proto-Vulcan tests.

use super::assertions::{assert_eq, assert_neq};
use super::environment::Environment;
use super::parser::{
    ast::{Item, Program, RelationDefinition, SearchStrategy},
    parse_str,
};
use super::query::QueryResult;
use super::{Interpreter, InterpreterError};
use crate::engine::{DefaultEngine, Engine};
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::{DefaultUser, User};
use colored::*;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use walkdir::WalkDir;

type DefaultInterpreter = Interpreter<DefaultUser, DefaultEngine<DefaultUser>>;

/// A single discovered test case.
#[derive(Debug, Clone)]
pub struct TestItem {
    /// The path to the file containing the test.
    pub file_path: PathBuf,
    /// The name of the test relation.
    pub test_name: String,
    /// Whether the test is expected to produce no results.
    pub should_fail: bool,
}

/// The result of a single test execution.
#[derive(Debug, Clone, PartialEq)]
pub enum TestResult {
    Pass,
    Fail,
    Error(String),
}

/// The main struct for finding and running tests.
pub struct TestRunner {
    /// A list of all discovered tests.
    pub discovered_tests: Vec<TestItem>,
}

impl TestRunner {
    /// Creates a new `TestRunner` and discovers all tests.
    pub fn new(root_dir: &Path) -> Result<Self, InterpreterError> {
        let discovered_tests = Self::discover(root_dir)?;
        Ok(Self { discovered_tests })
    }

    /// The main entry point for the test runner. Returns `true` if all tests pass.
    pub fn run(&self) -> bool {
        println!("\nRunning {} tests", self.discovered_tests.len());
        let mut passed = 0;
        let mut failed = 0;

        for item in &self.discovered_tests {
            let test_name = format!("test {} ... ", item.test_name);
            print!("{}", test_name);

            let result = self.execute_test(item);
            match result {
                TestResult::Pass => {
                    println!("{}", "ok".green());
                    passed += 1;
                }
                TestResult::Fail => {
                    println!("{}", "FAILED".red());
                    failed += 1;
                }
                TestResult::Error(e) => {
                    println!("{}", "ERROR".red());
                    println!("  Error: {}", e);
                    failed += 1; // Errors are considered failures
                }
            }
        }

        println!(
            "\nTest result: {}. {} passed; {} failed.",
            if failed == 0 {
                "ok".green()
            } else {
                "FAILED".red()
            },
            passed,
            failed,
        );

        failed == 0
    }

    fn execute_test(&self, item: &TestItem) -> TestResult {
        let mut interpreter = DefaultInterpreter::with_stdlib();

        // Manually register assertion relations
        self.register_assertion_builtins(&mut interpreter.environment.borrow_mut());

        let file_contents = match fs::read_to_string(&item.file_path) {
            Ok(c) => c,
            Err(e) => return TestResult::Error(e.to_string()),
        };

        let program = match parse_str(&file_contents) {
            Ok(p) => p,
            Err(e) => return TestResult::Error(format!("Parse error: {}", e)),
        };

        if let Err(e) = interpreter.load_program(program) {
            return TestResult::Error(format!("Load error: {}", e));
        }

        let query_string = format!("{}()", item.test_name);
        match interpreter.query(&query_string) {
            Ok(results) => {
                let successful_run = !results.is_empty();
                // A test passes if its success state matches what's expected.
                // (Succeeds and shouldn't fail) OR (Fails and should fail)
                if successful_run != item.should_fail {
                    TestResult::Pass
                } else {
                    TestResult::Fail
                }
            }
            Err(e) => TestResult::Error(format!("Runtime error: {}", e)),
        }
    }

    fn register_assertion_builtins<U, E>(&self, env: &mut Environment<U, E>)
    where
        U: User,
        E: Engine<U>,
    {
        let assert_eq_rel = Rc::new(move |args: Vec<LTerm<U, E>>| -> Goal<U, E> {
            assert_eq(args[0].clone(), args[1].clone())
        });
        env.add_native_relation("assert_eq".to_string(), assert_eq_rel, 2);

        let assert_neq_rel = Rc::new(move |args: Vec<LTerm<U, E>>| -> Goal<U, E> {
            assert_neq(args[0].clone(), args[1].clone())
        });
        env.add_native_relation("assert_neq".to_string(), assert_neq_rel, 2);
    }

    /// Discovers all tests within the given directory.
    fn discover(root_dir: &Path) -> Result<Vec<TestItem>, InterpreterError> {
        let mut tests = Vec::new();
        let pv_files = WalkDir::new(root_dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "pv"));

        for entry in pv_files {
            let path = entry.path();
            let content =
                fs::read_to_string(path).map_err(|e| InterpreterError::IoError(e.to_string()))?;

            // We parse the file but don't care about the result, only that it doesn't error.
            // A full semantic check isn't performed here, only parsing.
            let program = match parse_str(&content) {
                Ok(prog) => prog,
                Err(e) => {
                    // We print a warning but don't stop the test discovery.
                    // A broken file shouldn't prevent other tests from running.
                    println!("Warning: Could not parse file {}: {}", path.display(), e);
                    continue;
                }
            };

            for item in &program.items {
                if let Item::Relation(rel_def) = item {
                    if let Some(test_attr) = rel_def.attributes.iter().find(|a| a.name == "test") {
                        let should_fail = test_attr.args.iter().any(|arg| arg == "should_fail");
                        tests.push(TestItem {
                            file_path: path.to_path_buf(),
                            test_name: rel_def.name.clone(),
                            should_fail,
                        });
                    }
                }
            }
        }

        Ok(tests)
    }
}
