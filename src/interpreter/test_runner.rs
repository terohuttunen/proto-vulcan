//! This module contains the logic for discovering and running Proto-Vulcan tests.
//!
//! # Test Attributes: Architectural Distinction
//!
//! Proto-Vulcan supports two different test patterns for handling failure scenarios,
//! each designed for different types of execution outcomes:
//!
//! ## `@test(should_fail)` - Runtime Exceptions and Errors
//!
//! Use this attribute when you expect the **goal execution to throw an actual exception**.
//! This is appropriate for:
//! - Assertion failures: `assert_eq(1, 2)`
//! - Undefined relations: `undefined_relation(x)`
//! - Type errors or malformed queries
//! - Parse errors and syntax issues
//! - Non-exhaustive pattern matches
//!
//! Example:
//! ```prolog
//! @test(should_fail)
//! rel test_assertion_failure() {
//!     assert_eq(1, 2)  // Throws assertion exception
//! }
//! ```
//!
//! ## `@test(expected = [])` - Successful Execution with No Solutions
//!
//! Use this attribute when you expect the **query to execute successfully but find no valid solutions**.
//! This is appropriate for:
//! - Constraint domain failures (impossible constraint satisfaction)
//! - List operations with no matches (e.g., `member(x, [])`, `member(4, [1,2,3])`)
//! - Logical operations that fail (e.g., `distinct([1,1])`)
//! - Over-constrained systems in CLPFD/CLPZ domains
//! - Any scenario where the solver successfully determines no solutions exist
//!
//! Example:
//! ```prolog
//! @test(expected = [])
//! rel test_impossible_constraints(result) {
//!     |x, y, z| {
//!         constraint(domain="clpfd") {
//!             [x, y, z] in 1..2,     // Domain has only 2 values
//!             alldiff [x, y, z]      // But need 3 distinct values
//!         },
//!         result == [x, y, z]
//!     }
//! }
//! ```
//!
//! ## The Key Architectural Difference
//!
//! - **`should_fail`**: Query execution throws an exception (`interpreter.query()` returns `Err`)
//! - **`expected = []`**: Query execution succeeds but finds no valid solutions (`interpreter.query()` returns `Ok(empty_vector)`)
//!
//! ## Important Note on Constraint Domains
//!
//! Constraint domain solvers (CLPFD, CLPZ) do NOT throw exceptions when constraints cannot be satisfied.
//! Instead, they return empty result sets. Therefore, impossible constraint scenarios should use
//! `@test(expected = [])`, not `@test(should_fail)`.

use super::assertions::{assert_eq, assert_neq};
use super::environment::Environment;
use super::parser::{
    ast::{self, Item},
    parse_str,
};
use super::{Interpreter, InterpreterError};
use crate::engine::{DefaultEngine, Engine};
use crate::goal::Goal;
use crate::lterm::{LTerm, LTermInner, LValue};
use crate::user::{DefaultUser, User};
use colored::*;
use regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;
use walkdir::WalkDir;

type DefaultInterpreter = Interpreter<DefaultUser, DefaultEngine<DefaultUser>>;

/// A single discovered test case.
#[derive(Debug, Clone)]
pub struct TestItem {
    /// The path to the file containing the test.
    pub file_path: PathBuf,
    /// The name of the test relation.
    pub test_name: String,
    /// Whether the test is expected to fail during goal execution (throw an exception).
    ///
    /// This is distinct from tests that execute successfully but return no results.
    /// See module documentation for the architectural distinction between
    /// `@test(should_fail)` and `@test(expected = [])`.
    pub should_fail: bool,
    /// An expected list of results for a query-based test.
    ///
    /// When `Some([])`, the test expects successful execution with no solutions found.
    /// When `None`, the test uses simple pass/fail logic based on `should_fail`.
    pub expected: Option<ast::Term>,
    /// The variable to query in a query-based test.
    pub query_variable: Option<String>,
}

/// The result of a single test execution.
#[derive(Debug, Clone, PartialEq)]
pub enum TestResult {
    Pass,
    Fail,
    Error(String),
}

/// Options for filtering and running tests.
#[derive(Debug, Clone)]
pub struct TestRunOptions {
    /// Filter tests by name pattern (supports basic glob patterns like * and ?)
    pub filter: Option<String>,
    /// Show only failed tests in output
    pub show_failures_only: bool,
    /// Show timing information for each test
    pub show_timing: bool,
    /// Run tests in parallel (if supported in the future)
    pub parallel: bool,
}

impl Default for TestRunOptions {
    fn default() -> Self {
        Self {
            filter: None,
            show_failures_only: false,
            show_timing: false,
            parallel: false,
        }
    }
}

/// Detailed statistics about test execution.
#[derive(Debug, Clone)]
pub struct TestStats {
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub filtered_out: usize,
    pub total_duration: std::time::Duration,
}

impl TestStats {
    pub fn new() -> Self {
        Self {
            total_tests: 0,
            passed: 0,
            failed: 0,
            errors: 0,
            filtered_out: 0,
            total_duration: std::time::Duration::from_secs(0),
        }
    }
}

/// The result of a single test execution with timing information.
#[derive(Debug, Clone)]
pub struct TestExecutionResult {
    pub result: TestResult,
    pub duration: std::time::Duration,
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

    /// Filter tests by name pattern. Supports basic glob patterns:
    /// - `*` matches any sequence of characters
    /// - `?` matches any single character
    /// - Otherwise performs exact substring matching
    pub fn filter_tests(&self, pattern: &str) -> Vec<&TestItem> {
        if pattern.is_empty() {
            return self.discovered_tests.iter().collect();
        }

        // Convert simple glob patterns to regex
        let regex_pattern = if pattern.contains('*') || pattern.contains('?') {
            let escaped = regex::escape(pattern);
            let with_wildcards = escaped.replace("\\*", ".*").replace("\\?", ".");
            format!("^{}$", with_wildcards)
        } else {
            // Simple substring matching
            pattern.to_string()
        };

        self.discovered_tests
            .iter()
            .filter(|test| {
                if pattern.contains('*') || pattern.contains('?') {
                    // Use regex for glob patterns
                    if let Ok(re) = regex::Regex::new(&regex_pattern) {
                        re.is_match(&test.test_name)
                    } else {
                        // Fallback to substring matching if regex fails
                        test.test_name.contains(pattern)
                    }
                } else {
                    // Simple substring matching
                    test.test_name.contains(pattern)
                }
            })
            .collect()
    }

    /// Run tests with the given options and return detailed statistics.
    pub fn run_with_options(&self, options: &TestRunOptions) -> TestStats {
        let filtered_tests = if let Some(filter) = &options.filter {
            self.filter_tests(filter)
        } else {
            self.discovered_tests.iter().collect()
        };

        let mut stats = TestStats::new();
        stats.total_tests = self.discovered_tests.len();
        stats.filtered_out = stats.total_tests - filtered_tests.len();

        if filtered_tests.is_empty() {
            if let Some(filter) = &options.filter {
                println!("No tests match pattern '{}'", filter);
            } else {
                println!("No tests found.");
            }
            return stats;
        }

        println!(
            "\nRunning {} tests (filtered from {} total)",
            filtered_tests.len(),
            stats.total_tests
        );

        let start_time = Instant::now();
        let mut test_results = Vec::new();

        for item in &filtered_tests {
            let test_start = Instant::now();
            let test_name = format!("test {} ... ", item.test_name);

            if !options.show_failures_only {
                print!("{}", test_name);
            }

            let result = self.execute_test(item);
            let duration = test_start.elapsed();

            let execution_result = TestExecutionResult {
                result: result.clone(),
                duration,
            };
            test_results.push((item, execution_result));

            match result {
                TestResult::Pass => {
                    stats.passed += 1;
                    if !options.show_failures_only {
                        if options.show_timing {
                            println!("{} ({:.3}s)", "ok".green(), duration.as_secs_f64());
                        } else {
                            println!("{}", "ok".green());
                        }
                    }
                }
                TestResult::Fail => {
                    stats.failed += 1;
                    if options.show_failures_only {
                        print!("{}", test_name);
                    }
                    if options.show_timing {
                        println!("{} ({:.3}s)", "FAILED".red(), duration.as_secs_f64());
                    } else {
                        println!("{}", "FAILED".red());
                    }
                }
                TestResult::Error(ref e) => {
                    stats.errors += 1;
                    if options.show_failures_only {
                        print!("{}", test_name);
                    }
                    if options.show_timing {
                        println!("{} ({:.3}s)", "ERROR".red(), duration.as_secs_f64());
                    } else {
                        println!("{}", "ERROR".red());
                    }
                    println!("  Error: {}", e);
                }
            }
        }

        stats.total_duration = start_time.elapsed();

        // Print summary
        self.print_test_summary(&stats, &options);

        stats
    }

    /// The main entry point for the test runner. Returns `true` if all tests pass.
    /// This method is kept for backward compatibility.
    pub fn run(&self) -> bool {
        let options = TestRunOptions::default();
        let stats = self.run_with_options(&options);
        stats.failed == 0 && stats.errors == 0
    }

    /// Print a detailed test summary.
    fn print_test_summary(&self, stats: &TestStats, options: &TestRunOptions) {
        println!();

        if options.show_timing {
            println!(
                "Test execution completed in {:.3}s",
                stats.total_duration.as_secs_f64()
            );
        }

        if stats.filtered_out > 0 {
            println!("Filtered out {} tests", stats.filtered_out);
        }

        let _total_run = stats.passed + stats.failed + stats.errors;
        println!(
            "Test result: {}. {} passed; {} failed; {} errors.",
            if stats.failed == 0 && stats.errors == 0 {
                "ok".green()
            } else {
                "FAILED".red()
            },
            stats.passed,
            stats.failed,
            stats.errors,
        );
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

    /// Matches an LTerm against an AST Term, supporting wildcards.
    fn matches_pattern<U: User, E: Engine<U>>(lterm: &LTerm<U, E>, term: &ast::Term) -> bool {
        match term {
            ast::Term::Wildcard => true, // Wildcard matches anything
            ast::Term::Literal(literal) => Self::matches_literal(lterm, literal),
            ast::Term::Variable(_) => {
                // Variables in expected terms act as wildcards for matching
                true
            }
            ast::Term::List(list_construction) => {
                Self::matches_list_structure(lterm, list_construction)
            }
            ast::Term::NamedStruct(_) => {
                // TODO: Implement struct pattern matching if needed
                false
            }
            ast::Term::Compound(_) => {
                // TODO: Implement compound pattern matching if needed
                false
            }
            ast::Term::Parenthesized(inner) => Self::matches_pattern(lterm, inner),
            ast::Term::Interpolation(_) => {
                // TODO: Implement interpolation pattern matching
                false
            }
        }
    }

    /// Match an LTerm against a literal pattern.
    fn matches_literal<U: User, E: Engine<U>>(lterm: &LTerm<U, E>, literal: &ast::Literal) -> bool {
        match literal {
            ast::Literal::Boolean(expected) => {
                lterm.is_bool() && lterm.get_bool() == Some(*expected)
            }
            ast::Literal::Number(expected_str) => {
                if let Ok(expected_num) = expected_str.parse::<isize>() {
                    lterm.is_number() && lterm.get_number() == Some(expected_num)
                } else {
                    false
                }
            }
            ast::Literal::String(expected) => {
                // Check if the LTerm is a string value by examining the inner LValue
                if let LTermInner::Val(LValue::String(actual)) = lterm.as_ref() {
                    actual == expected
                } else {
                    false
                }
            }
            ast::Literal::Char(expected) => {
                // Check if the LTerm is a char value by examining the inner LValue
                if let LTermInner::Val(LValue::Char(actual)) = lterm.as_ref() {
                    actual == expected
                } else {
                    false
                }
            }
        }
    }

    /// Match an LTerm against a list pattern structure.
    fn matches_list_structure<U: User, E: Engine<U>>(
        lterm: &LTerm<U, E>,
        list_construction: &ast::ListConstruction,
    ) -> bool {
        // The LTerm must be a list
        if !lterm.is_list() {
            return false;
        }

        // Handle empty list case
        if list_construction.elements.is_empty() && list_construction.tail.is_none() {
            return lterm.is_empty();
        }

        // Collect LTerm elements into a vector for easier comparison
        let lterm_elements: Vec<&LTerm<U, E>> = lterm.iter().collect();

        // Check if we have enough elements for the pattern
        if lterm_elements.len() < list_construction.elements.len() {
            return false;
        }

        // Match each term element against corresponding LTerm element
        for (i, term_element) in list_construction.elements.iter().enumerate() {
            if !Self::matches_pattern(lterm_elements[i], term_element) {
                return false;
            }
        }

        // Handle tail pattern
        if let Some(tail_term) = &list_construction.tail {
            // If there's a tail term, create an LTerm for the remaining elements
            let remaining_count = lterm_elements.len() - list_construction.elements.len();

            if remaining_count == 0 {
                // No remaining elements - tail should match empty list
                let empty_list: LTerm<U, E> = LTerm::empty_list();
                return Self::matches_pattern(&empty_list, tail_term);
            } else if remaining_count == 1 {
                // Single remaining element - match directly
                let remaining_element = lterm_elements[list_construction.elements.len()];
                return Self::matches_pattern(remaining_element, tail_term);
            } else {
                // Multiple remaining elements - construct a list from them
                let remaining_elements: Vec<LTerm<U, E>> = lterm_elements
                    [list_construction.elements.len()..]
                    .iter()
                    .map(|&e| e.clone())
                    .collect();
                let remaining_list = LTerm::from_vec(remaining_elements);
                return Self::matches_pattern(&remaining_list, tail_term);
            }
        } else {
            // No tail term - list lengths must match exactly
            return lterm_elements.len() == list_construction.elements.len();
        }
    }

    fn execute_test(&self, item: &TestItem) -> TestResult {
        let mut interpreter = DefaultInterpreter::with_stdlib();

        // Manually register assertion builtins
        self.register_assertion_builtins(&mut interpreter.environment.borrow_mut());

        let file_contents = match fs::read_to_string(&item.file_path) {
            Ok(c) => c,
            Err(e) => return TestResult::Error(e.to_string()),
        };

        let program = match parse_str(&file_contents) {
            Ok(p) => p,
            Err(e) => {
                return TestResult::Error(format!(
                    "Parse error in file {}: {}",
                    item.file_path.display(),
                    e
                ))
            }
        };

        if let Err(e) = interpreter.load_program(program) {
            return TestResult::Error(format!(
                "Load error in file {}: {}",
                item.file_path.display(),
                e
            ));
        }

        let query_string = if let Some(var) = &item.query_variable {
            format!("{}({})", item.test_name, var)
        } else {
            format!("{}()", item.test_name)
        };

        // ARCHITECTURAL DISTINCTION: Here we implement the key difference between
        // @test(should_fail) and @test(expected = [])
        //
        // - Ok(results): Query executed successfully (may have 0 or more solutions)
        //   * If expected = Some([]), we check if results are empty (constraint solver found no solutions)
        //   * If expected = None and should_fail = false, we check if results are non-empty
        //   * If expected = None and should_fail = true, we fail because query should have thrown error
        //
        // - Err(e): Query failed during execution (goal resolution error)
        //   * If should_fail = true, this is expected behavior (TestResult::Pass)
        //   * If should_fail = false, this is an unexpected error (TestResult::Error)
        match interpreter.query(&query_string) {
            Ok(results) => {
                if let Some(expected_term) = &item.expected {
                    let query_variable = if let Some(v) = &item.query_variable {
                        v
                    } else {
                        return TestResult::Error(
                            "`expected` parameter requires a query variable in the test relation."
                                .to_string(),
                        );
                    };

                    let result_lterms: Vec<LTerm<_, _>> = results
                        .iter()
                        .filter_map(|r| r.bindings.get(query_variable).map(|res| res.0.clone()))
                        .collect();

                    let expected_ast_list = if let ast::Term::List(list) = expected_term {
                        &list.elements
                    } else {
                        return TestResult::Error(
                            "`expected` parameter must be a list term".to_string(),
                        );
                    };

                    if result_lterms.len() != expected_ast_list.len() {
                        // Convert results to strings for display
                        let result_lterms_str: Vec<String> =
                            result_lterms.iter().map(|t| t.to_string()).collect();
                        let expected_ast_list_str: Vec<String> =
                            expected_ast_list.iter().map(|t| format!("{}", t)).collect();

                        let msg = format!(
                            "Expected {} results, but got {}.\nExpected: {:?}\nActual:   {:?}",
                            expected_ast_list.len(),
                            result_lterms.len(),
                            expected_ast_list_str,
                            result_lterms_str
                        );
                        return if item.should_fail {
                            TestResult::Pass
                        } else {
                            TestResult::Error(msg)
                        };
                    }

                    let all_match = result_lterms.iter().zip(expected_ast_list.iter()).all(
                        |(result_lterm, expected_ast_term)| {
                            Self::matches_pattern(result_lterm, expected_ast_term)
                        },
                    );

                    if all_match != item.should_fail {
                        TestResult::Pass
                    } else {
                        if !all_match {
                            let result_lterms_str: Vec<String> =
                                result_lterms.iter().map(|t| t.to_string()).collect();
                            let expected_ast_list_str: Vec<String> =
                                expected_ast_list.iter().map(|t| format!("{}", t)).collect();

                            TestResult::Error(format!(
                                "Results did not match expected values.\nExpected: {:?}\nActual:   {:?}",
                                expected_ast_list_str, result_lterms_str
                            ))
                        } else {
                            TestResult::Error(
                                "Test succeeded but was marked as should_fail".to_string(),
                            )
                        }
                    }
                } else {
                    // Fallback to simple success/fail for tests without `expected`
                    // This handles the architectural distinction:
                    // - should_fail = true expects query to fail (Err), but we got Ok(results)
                    // - should_fail = false expects query to succeed with results
                    let successful_run = !results.is_empty();
                    if successful_run != item.should_fail {
                        TestResult::Pass
                    } else {
                        TestResult::Fail
                    }
                }
            }
            Err(e) => {
                // Query failed during execution (goal resolution error)
                if item.should_fail {
                    // This is expected for @test(should_fail) - goal was supposed to fail
                    TestResult::Pass
                } else {
                    // Unexpected error for regular tests
                    TestResult::Error(format!(
                        "Runtime error in test '{}' ({}): {}",
                        item.test_name,
                        item.file_path.display(),
                        e
                    ))
                }
            }
        }
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
                        let mut should_fail = false;
                        let mut expected = None;

                        for arg in &test_attr.args {
                            match arg {
                                ast::AttributeArg::Flag(name) if name == "should_fail" => {
                                    should_fail = true;
                                }
                                ast::AttributeArg::Named(name, value) if name == "expected" => {
                                    expected = Some(value.clone());
                                }
                                _ => {} // Ignore other args
                            }
                        }

                        let query_variable = rel_def.parameters.first().map(|p| p.name.clone());

                        tests.push(TestItem {
                            file_path: path.to_path_buf(),
                            test_name: rel_def.name.clone(),
                            should_fail,
                            expected,
                            query_variable,
                        });
                    }
                }
            }
        }

        Ok(tests)
    }
}
