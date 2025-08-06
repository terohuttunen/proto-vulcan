//! This module contains the logic for discovering and running Proto-Vulcan tests.
//!
//! # Test Attributes: Architectural Distinction
//!
//! Proto-Vulcan supports multiple test patterns for handling different execution outcomes:
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
//! ## `@test(should_timeout)` - Expected Timeout Scenarios
//!
//! Use this attribute when you expect the **query execution to exceed the timeout limit**.
//! This is appropriate for:
//! - Testing infinite loops or long-running computations
//! - Verifying that certain queries don't hang indefinitely
//! - Testing timeout handling in constraint solvers
//!
//! Example:
//! ```prolog
//! @test(should_timeout, timeout="1s")
//! rel test_infinite_loop() {
//!     test_infinite_loop()  // Infinite recursion - should timeout
//! }
//! ```
//!
//! ## `@test(timeout="10s")` - Custom Timeout Duration
//!
//! Use this attribute to override the global timeout for a specific test.
//! Supports time units: "1s", "500ms", "30s", etc.
//!
//! Example:
//! ```prolog
//! @test(timeout="30s")
//! rel test_long_computation() {
//!     // This test needs more time than the default timeout
//! }
//! ```
//!
//! ## The Key Architectural Differences
//!
//! - **`should_fail`**: Query execution throws an exception (`interpreter.query()` returns `Err`)
//! - **`expected = []`**: Query execution succeeds but finds no valid solutions (`interpreter.query()` returns `Ok(empty_vector)`)
//! - **`should_timeout`**: Query execution exceeds timeout limit (`TestResult::Timeout` becomes `TestResult::Pass`)
//!
//! ## Important Note on Constraint Domains
//!
//! Constraint domain solvers (CLPFD, CLPZ) do NOT throw exceptions when constraints cannot be satisfied.
//! Instead, they return empty result sets. Therefore, impossible constraint scenarios should use
//! `@test(expected = [])`, not `@test(should_fail)`.

use super::assertions::{assert_bound, assert_domain_size, assert_eq, assert_neq, assert_unbound};
use super::environment::Environment;
use super::parser::{
    ast::{self, Item},
    parse_str,
};
use super::{ExecutionConfig, Interpreter, InterpreterError};
use crate::goal::{Goal, GoalCast};
use crate::lterm::{LTerm, LTermInner, LValue};
use colored::*;
use regex;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;
use walkdir::WalkDir;

type DefaultInterpreter = Interpreter;

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
    /// Whether the test is expected to timeout during execution.
    ///
    /// When `true`, the test is expected to exceed its timeout limit and return a timeout result.
    /// This is useful for testing infinite loops or long-running computations.
    pub should_timeout: bool,
    /// Test-specific timeout in milliseconds, overriding the global timeout.
    ///
    /// When `Some(ms)`, this test will use the specified timeout instead of the global default.
    /// When `None`, the test uses the global timeout setting.
    pub timeout_ms: Option<u64>,
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
    Timeout,
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
    /// Maximum time to allow for a single test before timing out (in milliseconds)
    pub timeout_ms: Option<u64>,
}

impl Default for TestRunOptions {
    fn default() -> Self {
        Self {
            filter: None,
            show_failures_only: false,
            show_timing: false,
            parallel: false,
            timeout_ms: Some(30000), // Default 30 second timeout (30000ms)
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

    /// Parse a timeout duration string like "1s", "10s", "500ms" into milliseconds.
    /// Returns the number of milliseconds as u64, or None if parsing fails.
    fn parse_timeout_duration(duration_str: &str) -> Option<u64> {
        let duration_str = duration_str.trim();

        if duration_str.ends_with("ms") {
            // Parse milliseconds
            let ms_str = &duration_str[..duration_str.len() - 2];
            ms_str.parse::<u64>().ok()
        } else if duration_str.ends_with("s") {
            // Parse seconds and convert to milliseconds
            let s_str = &duration_str[..duration_str.len() - 1];
            if let Ok(s) = s_str.parse::<u64>() {
                Some(s * 1000)
            } else {
                None
            }
        } else {
            // Try parsing as plain number (assume seconds) and convert to milliseconds
            if let Ok(s) = duration_str.parse::<u64>() {
                Some(s * 1000)
            } else {
                None
            }
        }
    }

    /// Creates a new `TestRunner` from a single file.
    pub fn from_file(file_path: &Path) -> Result<Self, InterpreterError> {
        let discovered_tests = Self::discover_from_file(file_path)?;
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

        println!("\nrunning {} tests", filtered_tests.len());

        let start_time = Instant::now();
        let mut test_results = Vec::new();

        for item in &filtered_tests {
            let test_start = Instant::now();
            let test_name = format!("test {} ... ", item.test_name);

            if !options.show_failures_only {
                print!("{}", test_name);
                io::stdout().flush().unwrap(); // Ensure test name appears immediately
            }

            let result = self.execute_test(item, options.timeout_ms);
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
                        io::stdout().flush().unwrap();
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
                        io::stdout().flush().unwrap();
                    }
                    if options.show_timing {
                        println!("{} ({:.3}s)", "ERROR".red(), duration.as_secs_f64());
                    } else {
                        println!("{}", "ERROR".red());
                    }
                    println!("  Error: {}", e);
                }
                TestResult::Timeout => {
                    stats.failed += 1;
                    if options.show_failures_only {
                        print!("{}", test_name);
                        io::stdout().flush().unwrap();
                    }
                    if options.show_timing {
                        println!("{} ({:.3}s)", "TIMEOUT".red(), duration.as_secs_f64());
                    } else {
                        println!("{}", "TIMEOUT".red());
                    }
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
            "test result: {}. {} passed; {} failed; {} errors.",
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

    fn register_assertion_builtins(&self, env: &mut Environment) {
        let assert_eq_rel = Rc::new(move |args: Vec<LTerm>| -> Goal {
            assert_eq(args[0].clone(), args[1].clone())
        });
        env.add_builtin_relation("assert_eq".to_string(), assert_eq_rel, 2);

        let assert_neq_rel = Rc::new(move |args: Vec<LTerm>| -> Goal {
            assert_neq(args[0].clone(), args[1].clone())
        });
        env.add_builtin_relation("assert_neq".to_string(), assert_neq_rel, 2);

        let assert_bound_rel = Rc::new(move |args: Vec<LTerm>| -> Goal {
            if args.len() != 1 {
                crate::relation::fail().cast_into()
            } else {
                assert_bound(args[0].clone())
            }
        });
        env.add_builtin_relation("assert_bound".to_string(), assert_bound_rel, 1);

        let assert_unbound_rel = Rc::new(move |args: Vec<LTerm>| -> Goal {
            if args.len() != 1 {
                crate::relation::fail().cast_into()
            } else {
                assert_unbound(args[0].clone())
            }
        });
        env.add_builtin_relation("assert_unbound".to_string(), assert_unbound_rel, 1);

        let assert_domain_size_rel = Rc::new(move |args: Vec<LTerm>| -> Goal {
            if args.len() != 2 {
                crate::relation::fail().cast_into()
            } else {
                // Extract the expected size from the second argument
                if let Some(size_val) = args[1].get_number() {
                    if size_val >= 0 {
                        let size = size_val as usize;
                        assert_domain_size(args[0].clone(), size)
                    } else {
                        crate::relation::fail().cast_into()
                    }
                } else {
                    crate::relation::fail().cast_into()
                }
            }
        });
        env.add_builtin_relation("assert_domain_size".to_string(), assert_domain_size_rel, 2);
    }

    /// Matches an LTerm against an AST Term, supporting wildcards.
    fn matches_pattern(lterm: &LTerm, term: &ast::Term) -> bool {
        match term {
            ast::Term::Wildcard(_) => true, // Wildcard matches anything
            ast::Term::Literal(literal, _) => Self::matches_literal(lterm, literal),
            ast::Term::Variable(_) => true, // Variables in expected terms act as wildcards for matching
            ast::Term::List(list_construction, _) => {
                Self::matches_list_structure(lterm, list_construction)
            }
            ast::Term::NamedStruct(named_struct, _) => {
                Self::matches_named_struct(lterm, named_struct)
            }
            ast::Term::TupleStruct(tuple_struct, _) => {
                Self::matches_tuple_struct(lterm, tuple_struct)
            }
            ast::Term::Parenthesized(inner, _) => Self::matches_pattern(lterm, inner),
            ast::Term::Interpolation(..) => {
                // TODO: Implement interpolation pattern matching
                false
            }
            ast::Term::EnumVariant(enum_variant, _) => {
                Self::matches_enum_variant(lterm, enum_variant)
            }
        }
    }

    /// Match an LTerm against a literal pattern.
    fn matches_literal(lterm: &LTerm, literal: &ast::Literal) -> bool {
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
    fn matches_list_structure(lterm: &LTerm, list_construction: &ast::ListConstruction) -> bool {
        // The LTerm must be a list
        if !lterm.is_list() {
            return false;
        }

        // Handle empty list case
        if list_construction.elements.is_empty() && list_construction.tail.is_none() {
            return lterm.is_empty();
        }

        // Collect LTerm elements into a vector for easier comparison
        let lterm_elements: Vec<&LTerm> = lterm.iter().collect();

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
                let empty_list: LTerm = LTerm::empty_list();
                return Self::matches_pattern(&empty_list, tail_term);
            } else if remaining_count == 1 {
                // Single remaining element - match directly
                let remaining_element = lterm_elements[list_construction.elements.len()];
                return Self::matches_pattern(remaining_element, tail_term);
            } else {
                // Multiple remaining elements - construct a list from them
                let remaining_elements: Vec<LTerm> = lterm_elements
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

    /// Match an LTerm against an enum variant pattern.
    fn matches_enum_variant(lterm: &LTerm, enum_variant: &ast::EnumVariantConstruction) -> bool {
        use super::runtime::{RegistryEnumVariant, VariantData};
        use crate::lterm::LTermInner;

        // Check if the LTerm is a compound object representing an enum variant
        if let LTermInner::Compound(compound_obj) = lterm.as_ref() {
            if let Some(enum_var) = compound_obj.as_any().downcast_ref::<RegistryEnumVariant>() {
                // Check variant name matches
                if enum_var.variant_name != enum_variant.variant_name.to_string() {
                    return false;
                }

                // For now, skip enum name verification since we'd need environment access
                // TODO: Add environment context to enable full enum name verification

                // Match the construction kind
                match (&enum_var.variant_data, &enum_variant.kind) {
                    // Both unit variants
                    (VariantData::Unit, ast::EnumVariantConstructionKind::Unit) => true,

                    // Both tuple variants - check arguments
                    (
                        VariantData::Tuple(lterm_args),
                        ast::EnumVariantConstructionKind::Tuple(ast_args),
                    ) => {
                        if lterm_args.len() != ast_args.len() {
                            return false;
                        }
                        lterm_args
                            .iter()
                            .zip(ast_args.iter())
                            .all(|(lterm_arg, ast_arg)| Self::matches_pattern(lterm_arg, ast_arg))
                    }

                    // Both named variants - check fields
                    (
                        VariantData::Named(lterm_fields),
                        ast::EnumVariantConstructionKind::Named(ast_fields),
                    ) => {
                        if lterm_fields.len() != ast_fields.len() {
                            return false;
                        }

                        // Check that all AST fields exist in LTerm fields and match
                        ast_fields.iter().all(|ast_field| {
                            lterm_fields
                                .iter()
                                .find(|(name, _)| name == &*ast_field.name)
                                .map(|(_, field_value)| {
                                    Self::matches_pattern(field_value, &ast_field.value)
                                })
                                .unwrap_or(false)
                        })
                    }

                    // Mismatched kinds
                    _ => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Match an LTerm against a named struct pattern.
    fn matches_named_struct(lterm: &LTerm, named_struct: &ast::NamedStructConstruction) -> bool {
        use super::runtime::RegistryNamedStruct;
        use crate::lterm::LTermInner;

        // Check if the LTerm is a compound object representing a named struct
        if let LTermInner::Compound(compound_obj) = lterm.as_ref() {
            if let Some(named_struct_obj) =
                compound_obj.as_any().downcast_ref::<RegistryNamedStruct>()
            {
                // Check field count matches
                if named_struct_obj.fields.len() != named_struct.fields.len() {
                    return false;
                }

                // Check that all AST fields exist in LTerm fields and match
                named_struct.fields.iter().all(|ast_field| {
                    named_struct_obj
                        .fields
                        .iter()
                        .find(|(name, _)| name == &*ast_field.name)
                        .map(|(_, field_value)| {
                            Self::matches_pattern(field_value, &ast_field.value)
                        })
                        .unwrap_or(false)
                })
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Match an LTerm against a tuple struct pattern.
    fn matches_tuple_struct(lterm: &LTerm, tuple_struct: &ast::TupleStructConstruction) -> bool {
        use super::runtime::{RegistryEnumVariant, RegistryTupleStruct, VariantData};
        use crate::lterm::LTermInner;

        // Check if the LTerm is a compound object representing a tuple struct
        if let LTermInner::Compound(compound_obj) = lterm.as_ref() {
            if let Some(tuple_struct_obj) =
                compound_obj.as_any().downcast_ref::<RegistryTupleStruct>()
            {
                // Check argument count matches
                if tuple_struct_obj.args.len() != tuple_struct.args.len() {
                    return false;
                }

                // Check that all arguments match
                tuple_struct_obj
                    .args
                    .iter()
                    .zip(tuple_struct.args.iter())
                    .all(|(lterm_arg, ast_arg)| Self::matches_pattern(lterm_arg, ast_arg))
            } else if let Some(enum_variant_obj) =
                compound_obj.as_any().downcast_ref::<RegistryEnumVariant>()
            {
                // Check if this TupleStruct might actually be an enum variant like Color::Red
                // The tuple struct name should be in the form "EnumName::VariantName"
                if let Some((_enum_name, variant_name)) =
                    tuple_struct.name.to_string().rsplit_once("::")
                {
                    // Check that the variant name matches
                    if enum_variant_obj.variant_name == variant_name {
                        // Match the construction arguments based on variant data
                        match &enum_variant_obj.variant_data {
                            VariantData::Unit => tuple_struct.args.is_empty(),
                            VariantData::Tuple(lterm_args) => {
                                if lterm_args.len() != tuple_struct.args.len() {
                                    return false;
                                }
                                lterm_args.iter().zip(tuple_struct.args.iter()).all(
                                    |(lterm_arg, ast_arg)| {
                                        Self::matches_pattern(lterm_arg, ast_arg)
                                    },
                                )
                            }
                            VariantData::Named(_) => {
                                // Named variants can't be constructed with tuple syntax
                                false
                            }
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Execute a test, with optional timeout support
    fn execute_test(&self, item: &TestItem, global_timeout_ms: Option<u64>) -> TestResult {
        // Use test-specific timeout if specified, otherwise use global timeout
        let effective_timeout = item.timeout_ms.or(global_timeout_ms);

        if let Some(timeout) = effective_timeout {
            self.execute_test_with_timeout(item, timeout)
        } else {
            self.execute_test_internal(item)
        }
    }

    /// Execute a test with timeout support using solver-based timeout
    fn execute_test_with_timeout(&self, item: &TestItem, timeout_ms: u64) -> TestResult {
        // Use the solver-based timeout directly instead of threads
        let result = self.execute_test_internal_with_timeout(item, Some(timeout_ms));

        // Handle should_timeout logic: if test is expected to timeout and it did, that's a pass
        match (&result, item.should_timeout) {
            (TestResult::Timeout, true) => TestResult::Pass,
            (TestResult::Timeout, false) => TestResult::Timeout, // Keep as timeout (failure)
            (TestResult::Pass, true) => TestResult::Error(
                "Test was expected to timeout but completed successfully".to_string(),
            ),
            (TestResult::Fail, true) => {
                TestResult::Error("Test was expected to timeout but failed normally".to_string())
            }
            (other_result, _) => other_result.clone(),
        }
    }

    fn execute_test_internal(&self, item: &TestItem) -> TestResult {
        let result = self.execute_test_internal_with_timeout(item, None);

        // Handle should_timeout logic: if test expected to timeout but didn't, that's a failure
        match (&result, item.should_timeout) {
            (TestResult::Pass, true) => TestResult::Error(
                "Test was expected to timeout but completed successfully".to_string(),
            ),
            (TestResult::Fail, true) => {
                TestResult::Error("Test was expected to timeout but failed normally".to_string())
            }
            (other_result, _) => other_result.clone(),
        }
    }

    fn execute_test_internal_with_timeout(
        &self,
        item: &TestItem,
        timeout_ms: Option<u64>,
    ) -> TestResult {
        let test_start_time = std::time::Instant::now();

        // Step 1: Create base interpreter with stdlib (like CLI)
        let mut interpreter = DefaultInterpreter::with_stdlib();

        // Manually register assertion builtins in the environment
        self.register_assertion_builtins(&mut interpreter.environment.borrow_mut());

        // Load the standard library as base program (compiles std crate properly)
        use crate::interpreter::compiler::Compiler;
        match Compiler::create_stdlib_program() {
            Ok(stdlib_program) => {
                interpreter.base_program = Some(std::rc::Rc::new(stdlib_program));
            }
            Err(e) => {
                return TestResult::Error(format!("Failed to load standard library: {:?}", e));
            }
        }

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

        // Step 2: Compile test program with stdlib as base context
        // Use compile_items_into to add test items to the existing stdlib program
        let mut compiler = Compiler::new();

        // Get a mutable copy of the base program
        let mut combined_program = if let Some(base) = &interpreter.base_program {
            (**base).clone()
        } else {
            return TestResult::Error("No base program available".to_string());
        };

        // Compile test program items into the combined program
        if let Err(e) = compiler.compile_items_into(&mut combined_program, &program.items, None) {
            return TestResult::Error(format!(
                "Compilation error in file {}: {:?}",
                item.file_path.display(),
                e
            ));
        }

        // Update the interpreter's base program with the combined program
        interpreter.base_program = Some(std::rc::Rc::new(combined_program));

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
        let _test_timeout_info = timeout_ms.map(|ms| (test_start_time, ms));

        // Use the same query path as CLI to ensure proper variable resolution
        let config = ExecutionConfig {
            timeout: timeout_ms,
            ..Default::default()
        };

        match interpreter
            .query(&query_string, config)
            .map(|iter| iter.collect_limited(100).unwrap_or_default())
        {
            Ok(results) => {
                if let Some(expected) = &item.expected {
                    // Query-based test: compare results with expected values
                    let query_variable = if let Some(v) = &item.query_variable {
                        v
                    } else {
                        return TestResult::Error(
                            "`expected` parameter requires a query variable in the test relation."
                                .to_string(),
                        );
                    };

                    /*
                    for (i, r) in results.iter().enumerate() {
                        for (k, v) in &r.bindings {
                            eprintln!("  '{}' -> {:?}", k, v.0);
                        }
                    }
                    */

                    let result_lterms: Vec<LTerm> = results
                        .iter()
                        .filter_map(|r| r.bindings.get(query_variable).map(|res| res.0.clone()))
                        .collect();

                    let expected_ast_list = match expected {
                        ast::Term::List(list_construction, _) => list_construction.elements.clone(),
                        _ => {
                            return TestResult::Error(
                                "Expected term must be a list for query-based tests".to_string(),
                            )
                        }
                    };

                    if result_lterms.len() != expected_ast_list.len() {
                        // Convert results to strings for display
                        let result_lterms_str: Vec<String> = results
                            .iter()
                            .filter_map(|r| {
                                r.bindings.get(query_variable).map(|lresult| {
                                    // Extract just the value part (LTerm) from LResult for comparison
                                    // This mimics how CLI extracts values for display
                                    format!("{}", lresult.0)
                                })
                            })
                            .collect();
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
                            let result_lterms_str: Vec<String> = results
                                .iter()
                                .filter_map(|r| {
                                    r.bindings
                                        .get(query_variable)
                                        .map(|lresult| lresult.to_string())
                                })
                                .collect();
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
                // Check if this is a timeout error
                let error_message = e.to_string();
                if error_message.contains("Query execution timed out")
                    || error_message.contains("timed out")
                {
                    TestResult::Timeout
                } else if item.should_fail {
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

    /// Enhanced failure context method to provide better debugging information
    fn enhanced_failure_context(&self, item: &TestItem) -> TestResult {
        // Read the test file and analyze its structure
        let file_contents = match fs::read_to_string(&item.file_path) {
            Ok(c) => c,
            Err(e) => return TestResult::Error(format!("Could not read file for debug: {}", e)),
        };

        // Look for assertions and constraint blocks in the test
        let lines: Vec<&str> = file_contents.lines().collect();
        let mut assertions = Vec::new();
        let mut constraint_blocks = Vec::new();
        let mut fresh_variables = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("assert_") {
                assertions.push(format!("Line {}: {}", line_num + 1, trimmed));
            } else if trimmed.contains("constraint(domain=") {
                constraint_blocks.push(format!("Line {}: {}", line_num + 1, trimmed));
            } else if trimmed.starts_with("|") && trimmed.contains("|") {
                fresh_variables.push(format!("Line {}: {}", line_num + 1, trimmed));
            }
        }

        let mut debug_info = vec![
            format!("Test '{}' failed with no solutions found.", item.test_name),
            format!("File: {}", item.file_path.display()),
        ];

        if !constraint_blocks.is_empty() {
            debug_info.push("".to_string());
            debug_info.push("🔍 CONSTRAINT BLOCKS FOUND:".to_string());
            debug_info.extend(constraint_blocks);
            debug_info.push("".to_string());
            debug_info.push(
                "💡 LIKELY ISSUE: Fresh variables inside constraint blocks may not work correctly."
                    .to_string(),
            );
            debug_info.push(
                "   Consider moving variable declarations outside constraint blocks.".to_string(),
            );
        }

        if !fresh_variables.is_empty() {
            debug_info.push("".to_string());
            debug_info.push("🔍 FRESH VARIABLE DECLARATIONS:".to_string());
            debug_info.extend(fresh_variables);
        }

        if !assertions.is_empty() {
            debug_info.push("".to_string());
            debug_info.push("🔍 ASSERTIONS IN TEST:".to_string());
            debug_info.extend(assertions);
            debug_info.push("".to_string());
            debug_info
                .push("💡 DEBUG TIP: One or more of these assertions is failing.".to_string());
            debug_info.push(
                "   The most likely cause is unbound variables from constraint domains."
                    .to_string(),
            );
        }

        TestResult::Error(debug_info.join("\n"))
    }

    /// Extract a simple assertion from a line of code
    fn extract_assertion(&self, line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.ends_with(',') || trimmed.ends_with(';') {
            Some(trimmed[..trimmed.len() - 1].to_string())
        } else {
            Some(trimmed.to_string())
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
                if let Item::Predicate(rel_def) = item {
                    if let Some(test_attr) = rel_def.attributes.iter().find(|a| a.name == "test") {
                        let mut should_fail = false;
                        let mut should_timeout = false;
                        let mut timeout_ms = None;
                        let mut expected = None;

                        for arg in &test_attr.args {
                            match arg {
                                ast::AttributeArg::Flag(name) if name == "should_fail" => {
                                    should_fail = true;
                                }
                                ast::AttributeArg::Flag(name) if name == "should_timeout" => {
                                    should_timeout = true;
                                }
                                ast::AttributeArg::Named(name, value) if name == "expected" => {
                                    expected = Some(value.clone());
                                }
                                ast::AttributeArg::Named(name, value) if name == "timeout" => {
                                    // Parse timeout value - expect a string literal like "1s", "10s"
                                    if let ast::Term::Literal(
                                        ast::Literal::String(timeout_str),
                                        _,
                                    ) = value
                                    {
                                        timeout_ms = Self::parse_timeout_duration(timeout_str);
                                    }
                                }
                                _ => {} // Ignore other args
                            }
                        }

                        let query_variable = rel_def.parameters.first().map(|p| p.name.to_string());

                        tests.push(TestItem {
                            file_path: path.to_path_buf(),
                            test_name: rel_def.name.to_string(),
                            should_fail,
                            should_timeout,
                            timeout_ms,
                            expected,
                            query_variable,
                        });
                    }
                }
            }
        }

        Ok(tests)
    }

    /// Discovers all tests within a single file.
    fn discover_from_file(file_path: &Path) -> Result<Vec<TestItem>, InterpreterError> {
        let mut tests = Vec::new();

        // Check if the file has the correct extension
        if !file_path.extension().map_or(false, |ext| ext == "pv") {
            return Err(InterpreterError::IoError(format!(
                "File {} is not a .pv file",
                file_path.display()
            )));
        }

        let content =
            fs::read_to_string(file_path).map_err(|e| InterpreterError::IoError(e.to_string()))?;

        let program = match parse_str(&content) {
            Ok(prog) => prog,
            Err(e) => {
                return Err(InterpreterError::ParseError(format!(
                    "Could not parse file {}: {}",
                    file_path.display(),
                    e
                )));
            }
        };

        for item in &program.items {
            if let Item::Predicate(rel_def) = item {
                if let Some(test_attr) = rel_def.attributes.iter().find(|a| a.name == "test") {
                    let mut should_fail = false;
                    let mut should_timeout = false;
                    let mut timeout_ms = None;
                    let mut expected = None;

                    for arg in &test_attr.args {
                        match arg {
                            ast::AttributeArg::Flag(name) if name == "should_fail" => {
                                should_fail = true;
                            }
                            ast::AttributeArg::Flag(name) if name == "should_timeout" => {
                                should_timeout = true;
                            }
                            ast::AttributeArg::Named(name, value) if name == "expected" => {
                                expected = Some(value.clone());
                            }
                            ast::AttributeArg::Named(name, value) if name == "timeout" => {
                                // Parse timeout value - expect a string literal like "1s", "10s"
                                if let ast::Term::Literal(ast::Literal::String(timeout_str), _) =
                                    value
                                {
                                    timeout_ms = Self::parse_timeout_duration(timeout_str);
                                }
                            }
                            _ => {
                                // Ignore unknown args
                            }
                        }
                    }

                    let query_variable = rel_def.parameters.first().map(|p| p.name.to_string());

                    tests.push(TestItem {
                        file_path: file_path.to_path_buf(),
                        test_name: rel_def.name.to_string(),
                        should_fail,
                        should_timeout,
                        timeout_ms,
                        expected,
                        query_variable,
                    });
                }
            }
        }

        Ok(tests)
    }
}
