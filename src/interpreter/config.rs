//! Configuration types for the Proto-Vulcan interpreter
//!
//! This module defines ExecutionConfig and related configuration types
//! used to control interpreter behavior across all execution methods.

use std::sync::{Arc, atomic::AtomicBool};

/// Comprehensive configuration for interpreter execution
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    // === Core execution settings ===
    /// Timeout in milliseconds for query execution
    pub timeout: Option<u64>,
    /// Maximum number of results to return
    pub result_limit: Option<usize>,
    /// Atomic boolean for interruption handling (Ctrl+C support)
    pub interruption_handler: Option<Arc<AtomicBool>>,
    
    // === Debugging and analysis ===
    /// Execution tracing configuration
    pub trace: Option<super::trace::TraceConfig>,
    /// Enable debug output during execution
    pub debug_enabled: bool,
    /// Show parsed AST in output
    pub show_ast: bool,
    /// Show compiled IR in output
    pub show_ir: bool,
    /// Enable performance profiling
    pub profile: bool,
    /// Suppress non-essential output
    pub quiet: bool,
    /// Show extra diagnostic output
    pub verbose: bool,
    
    // === Compilation settings ===
    /// Treat compilation warnings as errors
    pub warnings_as_errors: bool,
    /// Only check syntax and compilation, don't execute
    pub check_only: bool,
    /// Compile to IR only, don't execute
    pub compile_only: bool,
    
    // === Test-specific settings ===
    /// Filter tests by pattern (e.g., "test_*")
    pub test_filter: Option<String>,
    /// Timeout for individual tests in milliseconds
    pub test_timeout: Option<u64>,
    /// Stop test execution on first failure
    pub fail_fast: bool,
    /// Number of threads for parallel test execution
    pub test_threads: Option<usize>,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            timeout: None,
            result_limit: None,
            interruption_handler: None,
            trace: None,
            debug_enabled: false,
            show_ast: false,
            show_ir: false,
            profile: false,
            quiet: false,
            verbose: false,
            warnings_as_errors: false,
            check_only: false,
            compile_only: false,
            test_filter: None,
            test_timeout: None,
            fail_fast: false,
            test_threads: None,
        }
    }
}

impl ExecutionConfig {
    /// Create a config suitable for development
    pub fn development() -> Self {
        Self {
            debug_enabled: true,
            verbose: true,
            show_ast: false,
            show_ir: false,
            warnings_as_errors: false,
            ..Default::default()
        }
    }
    
    /// Create a config suitable for production
    pub fn production() -> Self {
        Self {
            quiet: true,
            warnings_as_errors: true,
            ..Default::default()
        }
    }
    
    /// Create a config suitable for testing
    pub fn testing() -> Self {
        Self {
            test_timeout: Some(5_000),
            fail_fast: false,
            quiet: true,
            ..Default::default()
        }
    }
    
    /// Create a config suitable for interactive use
    pub fn interactive() -> Self {
        Self {
            verbose: true,
            ..Default::default()
        }
    }
    
    /// Builder method to set timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout = Some(timeout_ms);
        self
    }
    
    /// Builder method to set result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.result_limit = Some(limit);
        self
    }
    
    /// Builder method to enable trace
    pub fn with_trace(mut self, trace: super::trace::TraceConfig) -> Self {
        self.trace = Some(trace);
        self
    }
    
    /// Builder method to enable debug
    pub fn with_debug(mut self) -> Self {
        self.debug_enabled = true;
        self
    }
    
    /// Builder method to set interruption handler
    pub fn with_interruption_handler(mut self, handler: Arc<AtomicBool>) -> Self {
        self.interruption_handler = Some(handler);
        self
    }
}