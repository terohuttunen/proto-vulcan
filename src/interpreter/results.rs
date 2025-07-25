//! Result types for interpreter operations
//!
//! This module defines comprehensive result types for all interpreter operations,
//! providing detailed information about compilation, execution, and testing.

use std::time::Duration;
use std::collections::HashMap;
use crate::lresult::LResult;
use super::compiler::{CompileError, CompileWarning};
use super::parser::ast;

/// Result of loading a program into the interpreter
#[derive(Debug)]
pub struct LoadResult {
    /// Time taken to compile the program
    pub compilation_time: Duration,
    /// Compilation warnings encountered
    pub warnings: Vec<CompileWarning>,
    /// Number of modules in the compiled program
    pub module_count: usize,
    /// Number of predicates in the compiled program
    pub predicate_count: usize,
    /// Number of types in the compiled program
    pub type_count: usize,
    /// Check result if check_only was requested
    pub check_result: Option<CheckResult>,
}

/// Result of syntax and compilation checking
#[derive(Debug)]
pub struct CheckResult {
    /// Whether the program is syntactically and semantically valid
    pub is_valid: bool,
    /// Compilation errors encountered
    pub errors: Vec<CompileError>,
    /// Compilation warnings encountered
    pub warnings: Vec<CompileWarning>,
    /// Parsed AST if show_ast was requested
    pub ast: Option<ast::Program>,
    /// Time taken to parse the program
    pub parse_time: Duration,
}

/// Result of compiling a program to IR
#[derive(Debug)]
pub struct CompileResult {
    /// The compiled IR program
    pub ir_program: super::compiler::ir::Program,
    /// Time taken to compile to IR
    pub compilation_time: Duration,
    /// Compilation warnings encountered
    pub warnings: Vec<CompileWarning>,
    /// Whether IR should be displayed (controlled by show_ir config)
    pub show_ir: bool,
}

/// Results of test execution
#[derive(Debug)]
pub struct TestResults {
    /// Total number of tests found
    pub total_tests: usize,
    /// Number of tests that passed
    pub passed: usize,
    /// Number of tests that failed
    pub failed: usize,
    /// Number of tests that were skipped
    pub skipped: usize,
    /// Total time taken to execute all tests
    pub execution_time: Duration,
    /// Detailed results for each test
    pub test_details: Vec<TestDetail>,
}

impl TestResults {
    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0 && self.total_tests > 0
    }
    
    /// Get the overall test success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.passed as f64 / self.total_tests as f64) * 100.0
        }
    }
}

/// Detailed information about a single test
#[derive(Debug)]
pub struct TestDetail {
    /// Name of the test (predicate name)
    pub name: String,
    /// Test execution status
    pub status: TestStatus,
    /// Time taken to execute this test
    pub execution_time: Duration,
    /// Error message if the test failed
    pub error_message: Option<String>,
    /// Source location of the test if available
    pub location: Option<ast::Location>,
}

/// Status of a test execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestStatus {
    /// Test passed successfully
    Passed,
    /// Test failed with an error
    Failed,
    /// Test was skipped (e.g., due to filter)
    Skipped,
    /// Test timed out
    Timeout,
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Passed => write!(f, "PASSED"),
            TestStatus::Failed => write!(f, "FAILED"),
            TestStatus::Skipped => write!(f, "SKIPPED"),
            TestStatus::Timeout => write!(f, "TIMEOUT"),
        }
    }
}

/// Result of a single query execution
/// 
/// This preserves the existing QueryResult structure and LResult,
/// which already handle constraint information beautifully.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Variable bindings with full constraint information
    pub bindings: HashMap<String, LResult>,
}

impl QueryResult {
    /// Create a new empty query result
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }
    
    /// Check if this result has any variable bindings
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
    
    /// Get the number of variable bindings
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }
    
    /// Get a specific variable binding
    pub fn get_binding(&self, variable: &str) -> Option<&LResult> {
        self.bindings.get(variable)
    }
}

impl Default for QueryResult {
    fn default() -> Self {
        Self::new()
    }
}