use self::environment::Environment;
use self::parser::ast;
use std::cell::RefCell;
use std::fmt::{self, Display};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
// use std::collections::HashMap; // Not used in this module
use std::time::Duration;

mod assertions;
pub mod builtins;
pub mod compiler;
pub mod constraint_domains;
pub mod deferred;
pub mod environment;
pub mod import;
mod integration;
pub mod metaprogramming;
pub mod parser;
pub mod query;
pub mod runtime;
mod runtime_value;
#[cfg(test)]
mod struct_tests;
pub mod symbol_table;
pub mod test_runner;
pub mod trace;

// New streaming API modules
mod config;
mod iterator;
mod results;

// Re-export new API types
pub use config::*;
pub use iterator::*;
pub use results::*;

// Export builder patterns and constraint plugin support
pub use crate::state::{ConstraintPlugin, ConstraintPluginRegistry};
pub use compiler::CompilerBuilder;

/// Validation functions for @main relations
pub fn find_main_relation(
    program: &ast::Program,
) -> Result<Option<&ast::PredicateDefinition>, String> {
    let main_relations: Vec<&ast::PredicateDefinition> = program
        .items
        .iter()
        .filter_map(|item| {
            if let ast::Item::Predicate(rel_def) = item {
                if rel_def.attributes.iter().any(|attr| attr.name == "main") {
                    Some(rel_def)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    match main_relations.len() {
        0 => Ok(None),
        1 => {
            let main_rel = main_relations[0];
            validate_main_relation(main_rel)?;
            Ok(Some(main_rel))
        }
        n => Err(format!(
            "Found {} relations with @main attribute, but only one is allowed per file. Relations: {}",
            n,
            main_relations.iter().map(|r| &*r.name).collect::<Vec<_>>().join(", ")
        )),
    }
}

/// Validates that a relation marked with @main meets the requirements
pub fn validate_main_relation(rel_def: &ast::PredicateDefinition) -> Result<(), String> {
    // Check that @main relations cannot have typed parameters (templates)
    for param in &rel_def.parameters {
        if let Some(type_annotation) = &param.type_annotation {
            let type_str = match type_annotation {
                metaprogramming::TypeAnnotation::Int => "int".to_string(),
                metaprogramming::TypeAnnotation::String => "string".to_string(),
                metaprogramming::TypeAnnotation::Bool => "bool".to_string(),
                metaprogramming::TypeAnnotation::RelInt => "Int".to_string(),
                metaprogramming::TypeAnnotation::RelString => "String".to_string(),
                metaprogramming::TypeAnnotation::RelBool => "Bool".to_string(),
                metaprogramming::TypeAnnotation::RelChar => "Char".to_string(),
                metaprogramming::TypeAnnotation::LTerm => "LTerm".to_string(),
                metaprogramming::TypeAnnotation::Relation(arity) => format!("rel({})", arity),
                metaprogramming::TypeAnnotation::Custom(name) => name.to_string(),
            };

            return Err(format!(
                "Relation '{}' marked with @main cannot have typed parameters. \
                Parameter '{}' has type annotation '{}', but @main relations can only have \
                untyped parameters for maximum compatibility",
                rel_def.name, param.name, type_str
            ));
        }
    }

    Ok(())
}

/// Creates a query string for the main relation with appropriate variable bindings
pub fn create_main_query(main_rel: &ast::PredicateDefinition) -> String {
    if main_rel.parameters.is_empty() {
        format!("{}()", main_rel.name)
    } else {
        let param_vars: Vec<String> = main_rel
            .parameters
            .iter()
            .map(|p| p.name.to_string())
            .collect();
        format!("{}({})", main_rel.name, param_vars.join(", "))
    }
}

#[derive(Debug, Clone)]
pub enum InterpreterError {
    ParseError(String),
    RuntimeError(String),
    QueryExecutionError(String),
    UnknownRelation(String),
    UnknownVariable(String),
    UnknownType(String),
    NotAType(String),
    DuplicateDefinition(String),
    ModuleNotFound(PathBuf),
    IoError(String),
    IllegalSearchStrategyEmbedding {
        attempted: String,
        current_context: String,
    },
    SearchStrategyWarning {
        attempted: String,
        current_context: String,
        reason: String,
    },
    UnknownConstraintDomain(String),
    UnknownGlobalConstraint(String),
    InvalidConstraintArgs(String),
    InvalidConstraintSyntax {
        domain: String,
        error: String,
    },
    ExpectedNumber,
    ArityMismatch {
        relation_name: String,
        expected: usize,
        actual: usize,
    },
    UnboundRelationParameter(String),
    InvalidRelationParameter {
        parameter_name: String,
        reason: String,
    },
}

impl Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterpreterError::ParseError(e) => write!(f, "Parse error: {}", e),
            InterpreterError::RuntimeError(e) => write!(f, "Runtime error: {}", e),
            InterpreterError::QueryExecutionError(e) => write!(f, "Query execution error: {}", e),
            InterpreterError::UnknownRelation(name) => write!(f, "Unknown predicate: {}", name),
            InterpreterError::UnknownVariable(name) => write!(f, "Unknown variable: {}", name),
            InterpreterError::UnknownType(name) => write!(f, "Unknown type: {}", name),
            InterpreterError::NotAType(name) => write!(f, "{} is not a type", name),
            InterpreterError::DuplicateDefinition(name) => {
                write!(f, "Duplicate definition: {}", name)
            }
            InterpreterError::ModuleNotFound(path) => {
                write!(f, "Module not found: {}", path.display())
            }
            InterpreterError::IoError(msg) => write!(f, "I/O error: {}", msg),
            InterpreterError::IllegalSearchStrategyEmbedding {
                attempted,
                current_context,
            } => write!(
                f,
                "Illegal search strategy embedding: Cannot use {} search within {} context",
                attempted, current_context
            ),
            InterpreterError::SearchStrategyWarning {
                attempted,
                current_context,
                reason,
            } => write!(
                f,
                "Search strategy warning: Using {} search within {} context - {}",
                attempted, current_context, reason
            ),
            InterpreterError::UnknownConstraintDomain(domain) => {
                write!(f, "Unknown constraint domain: {}", domain)
            }
            InterpreterError::UnknownGlobalConstraint(constraint) => {
                write!(f, "Unknown global constraint: {}", constraint)
            }
            InterpreterError::InvalidConstraintArgs(msg) => {
                write!(f, "Invalid constraint arguments: {}", msg)
            }
            InterpreterError::InvalidConstraintSyntax { domain, error } => {
                write!(
                    f,
                    "Invalid constraint syntax in domain '{}': {}",
                    domain, error
                )
            }
            InterpreterError::ExpectedNumber => {
                write!(f, "Expected a number value")
            }
            InterpreterError::ArityMismatch {
                relation_name,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Arity mismatch for relation '{}': expected {}, got {}",
                    relation_name, expected, actual
                )
            }
            InterpreterError::UnboundRelationParameter(param_name) => {
                write!(f, "Unbound relation parameter: {}", param_name)
            }
            InterpreterError::InvalidRelationParameter {
                parameter_name,
                reason,
            } => {
                write!(
                    f,
                    "Invalid relation parameter '{}': {}",
                    parameter_name, reason
                )
            }
        }
    }
}

impl std::error::Error for InterpreterError {}

impl From<compiler::errors::CompileError> for InterpreterError {
    fn from(err: compiler::errors::CompileError) -> Self {
        InterpreterError::RuntimeError(format!("Compilation error: {:?}", err))
    }
}

/// Builder for configuring an Interpreter with custom options
pub struct InterpreterBuilder {
    with_stdlib: bool,
    plugin_registry: crate::state::ConstraintPluginRegistry,
}

impl InterpreterBuilder {
    /// Create a new InterpreterBuilder with default settings
    pub fn new() -> Self {
        Self {
            with_stdlib: false,
            plugin_registry: crate::state::ConstraintPluginRegistry::new(),
        }
    }

    /// Enable standard library builtin predicates
    pub fn with_stdlib(mut self) -> Self {
        self.with_stdlib = true;
        self
    }

    /// Add a constraint plugin (which may provide both state handling and compilation)
    pub fn with_constraint_plugin(mut self, plugin: Box<dyn crate::state::ConstraintPlugin>) -> Self {
        self.plugin_registry.register(plugin);
        self
    }

    /// Build the configured Interpreter
    pub fn build(self) -> Interpreter {
        let interpreter = Interpreter {
            environment: Rc::new(RefCell::new(Environment::new())),
            base_program: None,
            constraint_plugins: Some(Rc::new(self.plugin_registry)),
        };

        // Register builtins if requested
        if self.with_stdlib {
            builtins::register_builtins(&mut interpreter.environment.borrow_mut());
        }

        interpreter
    }
}

/// The main interpreter struct
pub struct Interpreter {
    pub environment: Rc<RefCell<Environment>>,
    /// Base program stored for query execution using copy-on-write semantics
    pub base_program: Option<Rc<compiler::ir::Program>>,
    /// Custom constraint plugins for enhanced constraint domain support
    constraint_plugins: Option<Rc<crate::state::ConstraintPluginRegistry>>,
}

impl Interpreter {
    /// Create a new interpreter
    pub fn new() -> Self {
        Self {
            environment: Rc::new(RefCell::new(Environment::new())),
            base_program: None,
            constraint_plugins: None,
        }
    }

    /// Creates a new interpreter with core builtins registered.
    /// Note: Standard library is now loaded automatically via import resolution when needed.
    pub fn with_stdlib() -> Self {
        let interpreter = Self::new();

        // Register all builtin relations
        builtins::register_builtins(&mut interpreter.environment.borrow_mut());

        // Note: No longer loading stdlib here - it will be loaded automatically
        // via the new IR-based import resolution when std predicates are imported
        interpreter
    }

    /// Create a new InterpreterBuilder for fluent configuration
    pub fn builder() -> InterpreterBuilder {
        InterpreterBuilder::new()
    }

    /// Get a reference to the environment
    pub fn environment(&self) -> std::cell::Ref<Environment> {
        self.environment.borrow()
    }

    /// Set command line arguments for the interpreter
    pub fn set_argv(&self, argv: Vec<String>) {
        self.environment.borrow_mut().set_argv(argv);
    }

    /// Get a reference to the base program
    pub fn base_program(&self) -> Option<&Rc<compiler::ir::Program>> {
        self.base_program.as_ref()
    }

    /// Check if a predicate exists in the loaded program (IR-based)
    pub fn has_predicate(&self, name: &str) -> bool {
        if let Some(program) = &self.base_program {
            let predicate_id = compiler::ir::PredicateId::with_parent(
                std::rc::Rc::new(compiler::ir::ModulePath::root()),
                name,
            );
            program.registry.get_predicate(&predicate_id).is_some()
        } else {
            false
        }
    }

    /// Check if a type exists in the loaded program (IR-based)
    pub fn has_type(&self, name: &str) -> bool {
        if let Some(program) = &self.base_program {
            let type_id =
                compiler::ir::TypeId::with_parent(std::rc::Rc::new(compiler::ir::ModulePath::root()), name);
            program.registry.get_type(&type_id).is_some()
        } else {
            false
        }
    }

    /// Get struct field information (IR-based)
    pub fn get_struct_fields(&self, name: &str) -> Option<Vec<String>> {
        if let Some(program) = &self.base_program {
            let type_id =
                compiler::ir::TypeId::with_parent(std::rc::Rc::new(compiler::ir::ModulePath::root()), name);
            if let Some(type_def) = program.registry.get_type(&type_id) {
                match &type_def.kind {
                    compiler::ir::TypeKind::Struct(struct_def) => match &struct_def.fields {
                        compiler::ir::StructFields::Named(fields) => {
                            Some(fields.iter().map(|f| f.name.to_string()).collect())
                        }
                        compiler::ir::StructFields::Tuple(_) => Some(Vec::new()),
                    },
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get predicate arity (IR-based)
    pub fn get_predicate_arity(&self, name: &str) -> Option<usize> {
        if let Some(program) = &self.base_program {
            let predicate_id = compiler::ir::PredicateId::with_parent(
                std::rc::Rc::new(compiler::ir::ModulePath::root()),
                name,
            );
            if let Some(predicate) = program.registry.get_predicate(&predicate_id) {
                Some(predicate.parameters.len())
            } else {
                None
            }
        } else {
            None
        }
    }


    /// Load a program as the base program for subsequent queries
    pub fn load_program(
        &mut self,
        program_source: &str,
        config: ExecutionConfig,
    ) -> Result<LoadResult, InterpreterError> {
        let start_time = std::time::Instant::now();

        // Parse program
        let program_ast = parser::parse_str(program_source)
            .map_err(|e| InterpreterError::ParseError(e.to_string()))?;

        if config.check_only {
            return Ok(LoadResult {
                compilation_time: Duration::default(),
                warnings: Vec::new(),
                module_count: 0,
                predicate_count: 0,
                type_count: 0,
                check_result: Some(CheckResult {
                    is_valid: true,
                    errors: Vec::new(),
                    warnings: Vec::new(),
                    ast: if config.show_ast {
                        Some(program_ast)
                    } else {
                        None
                    },
                    parse_time: start_time.elapsed(),
                }),
            });
        }

        // Compile program
        let ir_program = self.compile_program_with_config(program_ast, &config)?;
        let compilation_time = start_time.elapsed();

        // Count items in the program (simplified - actual count methods need to be implemented)
        let module_count = 1; // TODO: Get actual count from ir_program.registry
        let predicate_count = 0; // TODO: Get actual count from ir_program.registry
        let type_count = 0; // TODO: Get actual count from ir_program.registry

        // Store as base program wrapped in Rc for copy-on-write semantics
        self.base_program = Some(Rc::new(ir_program));

        Ok(LoadResult {
            compilation_time,
            warnings: Vec::new(), // TODO: Get from compiler
            module_count,
            predicate_count,
            type_count,
            check_result: None,
        })
    }

    /// Load a program from AST (for backward compatibility)
    pub fn load_program_ast(&mut self, program: ast::Program) -> Result<(), InterpreterError> {
        // Always use the standard loading method - don't try to merge programs
        self.load_program_from_ast_with_config(program, ExecutionConfig::default())?;
        Ok(())
    }

    /// Load a program from AST with configuration
    pub fn load_program_from_ast_with_config(
        &mut self,
        program_ast: ast::Program,
        config: ExecutionConfig,
    ) -> Result<LoadResult, InterpreterError> {
        let start_time = std::time::Instant::now();

        if config.check_only {
            return Ok(LoadResult {
                compilation_time: Duration::default(),
                warnings: Vec::new(),
                module_count: 0,
                predicate_count: 0,
                type_count: 0,
                check_result: Some(CheckResult {
                    is_valid: true,
                    errors: Vec::new(),
                    warnings: Vec::new(),
                    ast: if config.show_ast {
                        Some(program_ast)
                    } else {
                        None
                    },
                    parse_time: start_time.elapsed(),
                }),
            });
        }

        // Compile program
        let ir_program = self.compile_program_with_config(program_ast, &config)?;
        let compilation_time = start_time.elapsed();

        // Store as base program wrapped in Rc for copy-on-write semantics
        self.base_program = Some(Rc::new(ir_program));

        Ok(LoadResult {
            compilation_time,
            warnings: Vec::new(),
            module_count: 1,
            predicate_count: 0,
            type_count: 0,
            check_result: None,
        })
    }

    /// Load a specific module into the interpreter
    pub fn load_module(&mut self, _name: &str, path: &Path) -> Result<(), InterpreterError> {
        let source =
            fs::read_to_string(path).map_err(|e| InterpreterError::IoError(e.to_string()))?;
        self.load_program(&source, ExecutionConfig::default())?;
        Ok(())
    }

    /// Load the standard library.
    pub fn load_stdlib(&mut self) -> Result<(), InterpreterError> {
        // Use the environment's load_std_library method
        let mut env = self.environment.borrow_mut();
        env.load_std_library()
    }

    /// Load the standard library as the base program for import resolution
    pub fn load_stdlib_as_base_program(&mut self) -> Result<(), InterpreterError> {
        use crate::interpreter::parser;
        use std::fs;

        // Load std/mod.pv as the base
        let std_mod_path = PathBuf::from("std/mod.pv");
        if !std_mod_path.exists() {
            return Err(InterpreterError::IoError(
                "Standard library std/mod.pv not found".to_string(),
            ));
        }

        let std_mod_source = fs::read_to_string(&std_mod_path)
            .map_err(|e| InterpreterError::IoError(e.to_string()))?;

        let std_mod_ast = parser::parse_str(&std_mod_source).map_err(|e| {
            InterpreterError::RuntimeError(format!("Failed to parse std/mod.pv: {:?}", e))
        })?;

        // Create a wrapper program that declares std as a module
        let wrapper_program = self.create_stdlib_wrapper_program(std_mod_ast)?;

        // Load wrapper program as base program - this puts stdlib in ::std namespace
        self.load_program_from_ast_with_config(wrapper_program, ExecutionConfig::default())?;

        Ok(())
    }

    /// Create a wrapper program that loads stdlib into the ::std namespace
    fn create_stdlib_wrapper_program(
        &self,
        std_mod_ast: ast::Program,
    ) -> Result<ast::Program, InterpreterError> {
        use crate::interpreter::parser::ast;

        // Create a module declaration for std that contains the std/mod.pv content
        let std_module = ast::Item::Module(ast::ModuleDefinition {
            name: symbol_table::InternedSymbol::from_text("std"),
            visibility: ast::Visibility::Public,
            search_strategy: None,
            items: std_mod_ast.items,
            span: ast::Location::dummy(),
        });

        // Create the wrapper program with just the std module
        Ok(ast::Program {
            items: vec![std_module],
            span: ast::Location::dummy(),
        })
    }

    /// Generic method to append AST items to the current base program using CoW
    pub fn append_items_to_base_program(
        &mut self,
        items: Vec<ast::Item>,
    ) -> Result<(), InterpreterError> {
        // Get the current base program, or create empty one if none exists
        let mut current_program = if let Some(base) = &self.base_program {
            // Clone the program for CoW modification
            (**base).clone()
        } else {
            // Create new empty program
            compiler::ir::Program::new()
        };

        // Create an AST program with the new items
        let ast_program = ast::Program {
            items,
            span: ast::Location::dummy(),
        };

        // Compile the new items and add them to the program
        let _new_ir_items =
            self.compile_program_with_config(ast_program, &ExecutionConfig::default())?;

        // Add the new items to the current program using CoW
        let _registry_mut = current_program.registry_mut();

        // Add all items from the new program to the current program
        // Use proper registry methods instead of accessing private fields
        // For now, this method is deprecated - use compile_items_into instead
        // TODO: Remove this method and use compile_items_into for incremental compilation

        // Update the base program
        self.base_program = Some(Rc::new(current_program));

        Ok(())
    }

    /// Append individual predicate to base program using CoW
    pub fn append_predicate_to_base_program(
        &mut self,
        predicate: ast::PredicateDefinition,
    ) -> Result<(), InterpreterError> {
        let predicate_item = ast::Item::Predicate(predicate);
        self.append_items_to_base_program(vec![predicate_item])
    }

    /// Find the standard library path by trying different locations
    pub fn find_stdlib_path() -> Result<PathBuf, InterpreterError> {
        // Try different locations for the standard library
        let candidates = vec![
            // 1. Proto-vulcan's std directory (compile-time path - always points to proto-vulcan root)
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("std"),
            // 2. Relative to current directory (current behavior)
            PathBuf::from("std"),
            // 3. Relative to executable (preferred for installed binaries)
            Self::executable_relative_path("std"),
            // 4. Relative to executable's parent directory (for development)
            Self::executable_relative_path("../std"),
            // 5. In parent of executable's parent (for target/release structure)
            Self::executable_relative_path("../../std"),
        ];
        
        for candidate in &candidates {
            if candidate.exists() && candidate.is_dir() {
                // Found a valid std directory, return it
                return Ok(candidate.clone());
            }
        }

        Err(InterpreterError::IoError(
            format!("Standard library not found. Tried paths: {:?}", candidates)
        ))
    }

    /// Get a path relative to the current executable
    fn executable_relative_path(relative_path: &str) -> PathBuf {
        match std::env::current_exe() {
            Ok(exe_path) => {
                if let Some(exe_dir) = exe_path.parent() {
                    exe_dir.join(relative_path)
                } else {
                    PathBuf::from(relative_path)
                }
            }
            Err(_) => PathBuf::from(relative_path),
        }
    }

    // ===== NEW STREAMING API METHODS =====

    /// Execute a query against the loaded base program
    pub fn query(
        &mut self,
        query: &str,
        config: ExecutionConfig,
    ) -> Result<QueryResultIterator, InterpreterError> {
        // Require loaded base program
        let base_program = self.base_program.as_ref().ok_or_else(|| {
            InterpreterError::RuntimeError(
                "No base program loaded. Call load_program() first.".to_string(),
            )
        })?;

        // Parse query
        let query_goal = query::parse_query(query)?;

        // Convert ExecutionConfig to QueryConfig
        let query_config = query::QueryConfig {
            timeout: config.timeout,
            trace: config.trace,
        };

        // Use unified execute_query_ir with reification
        query::execute_query_ir(
            base_program.clone(),
            self.environment.clone(),
            query_goal,
            query_config,
        )
    }

    /// Execute a program with a specific query (no pre-loading required)
    pub fn run_program(
        &mut self,
        program_source: &str,
        query: &str,
        config: ExecutionConfig,
    ) -> Result<QueryResultIterator, InterpreterError> {
        // Parse program and query
        let program_ast = parser::parse_str(program_source)
            .map_err(|e| InterpreterError::ParseError(e.to_string()))?;
        let query_goal = query::parse_query(query)?;

        // Compile program
        let runtime_program = self.compile_program_with_config(program_ast, &config)?;

        // Convert ExecutionConfig to QueryConfig
        let query_config = query::QueryConfig {
            timeout: config.timeout,
            trace: config.trace,
        };

        // Use unified execute_query_ir with reification
        query::execute_query_ir(
            Rc::new(runtime_program),
            self.environment.clone(),
            query_goal,
            query_config,
        )
    }

    /// Execute a program's @main relation
    pub fn run_main(
        &mut self,
        program_source: &str,
        config: ExecutionConfig,
    ) -> Result<QueryResultIterator, InterpreterError> {
        // Parse program
        let program_ast = parser::parse_str(program_source)
            .map_err(|e| InterpreterError::ParseError(e.to_string()))?;

        // Find @main relation
        let main_relation = find_main_relation(&program_ast)
            .map_err(|e| InterpreterError::RuntimeError(e))?
            .ok_or_else(|| InterpreterError::RuntimeError("No @main relation found".to_string()))?;

        // Create query for main relation
        let main_query_str = create_main_query(main_relation);
        let main_query_goal = query::parse_query(&main_query_str)?;

        // Compile program
        let runtime_program = self.compile_program_with_config(program_ast, &config)?;

        // Convert ExecutionConfig to QueryConfig
        let query_config = query::QueryConfig {
            timeout: config.timeout,
            trace: config.trace,
        };

        // Use unified execute_query_ir with reification
        query::execute_query_ir(
            Rc::new(runtime_program),
            self.environment.clone(),
            main_query_goal,
            query_config,
        )
    }

    /// Execute tests in a program
    pub fn run_tests(
        &mut self,
        program_source: &str,
        _config: ExecutionConfig,
    ) -> Result<TestResults, InterpreterError> {
        // For now, return a placeholder implementation
        // TODO: Implement proper test discovery and execution
        let start_time = std::time::Instant::now();

        // Parse program to validate it
        let _program_ast = parser::parse_str(program_source)
            .map_err(|e| InterpreterError::ParseError(e.to_string()))?;

        // Return basic test results for now
        Ok(TestResults {
            total_tests: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            execution_time: start_time.elapsed(),
            test_details: Vec::new(),
        })
    }

    /// Check program syntax and compilation without execution
    pub fn check_program(
        &mut self,
        program_source: &str,
        config: ExecutionConfig,
    ) -> Result<CheckResult, InterpreterError> {
        let start_time = std::time::Instant::now();

        // Parse
        let program_ast = match parser::parse_str(program_source) {
            Ok(ast) => ast,
            Err(e) => {
                return Ok(CheckResult {
                    is_valid: false,
                    errors: vec![compiler::CompileError::SemanticError {
                        message: format!("Parse error: {}", e),
                        symbol: symbol_table::InternedSymbol::from_text("unknown"),
                    }],
                    warnings: Vec::new(),
                    ast: None,
                    parse_time: start_time.elapsed(),
                });
            }
        };

        let parse_time = start_time.elapsed();

        // Compile
        let mut compiler = compiler::Compiler::new();
        let compile_result = compiler.compile_from_ast(program_ast.clone());

        match compile_result {
            Ok(_) => Ok(CheckResult {
                is_valid: true,
                errors: Vec::new(),
                warnings: compiler.get_warnings().to_vec(),
                ast: if config.show_ast {
                    Some(program_ast)
                } else {
                    None
                },
                parse_time,
            }),
            Err(e) => Ok(CheckResult {
                is_valid: false,
                errors: vec![e],
                warnings: compiler.get_warnings().to_vec(),
                ast: if config.show_ast {
                    Some(program_ast)
                } else {
                    None
                },
                parse_time,
            }),
        }
    }

    /// Compile program to IR without execution
    pub fn compile_program(
        &mut self,
        program_source: &str,
        config: ExecutionConfig,
    ) -> Result<CompileResult, InterpreterError> {
        let program_ast = parser::parse_str(program_source)
            .map_err(|e| InterpreterError::ParseError(e.to_string()))?;

        let start_time = std::time::Instant::now();
        let ir_program = self.compile_program_with_config(program_ast, &config)?;

        Ok(CompileResult {
            ir_program,
            compilation_time: start_time.elapsed(),
            warnings: Vec::new(), // TODO: Get from compiler
            show_ir: config.show_ir,
        })
    }

    // ===== INTERNAL HELPER METHODS =====

    /// Internal method to compile a program with configuration
    fn compile_program_with_config(
        &self,
        program_ast: ast::Program,
        _config: &ExecutionConfig,
    ) -> Result<compiler::ir::Program, InterpreterError> {
        let mut compiler = compiler::Compiler::new();

        // Set compilation options based on config
        // TODO: Set compilation options from config when available
        // For now, use default options

        compiler
            .compile_from_ast(program_ast)
            .map_err(|e| InterpreterError::RuntimeError(format!("Compilation failed: {:?}", e)))
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
