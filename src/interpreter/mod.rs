use self::environment::Environment;
use self::parser::ast;
use crate::lterm::{LTerm, LTermInner};
use std::cell::RefCell;
use std::fmt::{self, Display};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
// use std::collections::HashMap; // Not used in this module
use std::time::Duration;
use crate::user::DefaultUser;

mod assertions;
pub mod compiler;
pub mod constraint_domains;
pub mod deferred;
mod environment;
pub mod import;
mod integration;
pub mod metaprogramming;
pub mod parser;
pub mod query;
pub mod runtime;
mod runtime_value;
mod semantic_analysis;
#[cfg(test)]
mod struct_tests;
pub mod symbol_table;
pub mod test_runner;
pub mod trace;

// New streaming API modules
mod config;
mod results;
mod iterator;

// Re-export new API types
pub use config::*;
pub use results::*;
pub use iterator::*;

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

/// The main interpreter struct
pub struct Interpreter {
    pub environment: Rc<RefCell<Environment>>,
    /// Base program stored for query execution (cloned for each execution)
    /// This is never accessed at runtime - always cloned first
    pub base_program: Option<compiler::ir::Program>,
}

impl Interpreter {
    /// Create a new interpreter
    pub fn new() -> Self {
        Self {
            environment: Rc::new(RefCell::new(Environment::new())),
            base_program: None,
        }
    }

    /// Creates a new interpreter and loads the standard library.
    pub fn with_stdlib() -> Self {
        let mut interpreter = Self::new();

        // Register core builtin relations
        interpreter.register_core_builtins();

        if let Err(e) = interpreter.load_stdlib() {
            eprintln!("Fatal: Failed to load standard library: {:?}", e);
        }
        interpreter
    }

    /// Get a reference to the environment
    pub fn environment(&self) -> std::cell::Ref<Environment> {
        self.environment.borrow()
    }

    /// Get a reference to the base program
    pub fn base_program(&self) -> Option<&compiler::ir::Program> {
        self.base_program.as_ref()
    }

    /// Check if a predicate exists in the loaded program (IR-based)
    pub fn has_predicate(&self, name: &str) -> bool {
        if let Some(program) = &self.base_program {
            let predicate_id = compiler::ir::PredicateId::new(format!("::{}", name));
            program.registry.get_predicate(&predicate_id).is_some()
        } else {
            false
        }
    }

    /// Check if a type exists in the loaded program (IR-based)
    pub fn has_type(&self, name: &str) -> bool {
        if let Some(program) = &self.base_program {
            let type_id = compiler::ir::TypeId::new(format!("::{}", name));
            program.registry.get_type(&type_id).is_some()
        } else {
            false
        }
    }

    /// Get struct field information (IR-based)
    pub fn get_struct_fields(&self, name: &str) -> Option<Vec<String>> {
        if let Some(program) = &self.base_program {
            let type_id = compiler::ir::TypeId::new(format!("::{}", name));
            if let Some(type_def) = program.registry.get_type(&type_id) {
                match &type_def.kind {
                    compiler::ir::TypeKind::Struct(struct_def) => {
                        match &struct_def.fields {
                            compiler::ir::StructFields::Named(fields) => {
                                Some(fields.iter().map(|f| f.name.to_string()).collect())
                            }
                            compiler::ir::StructFields::Tuple(_) => Some(Vec::new()),
                        }
                    }
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
            let predicate_id = compiler::ir::PredicateId::new(format!("::{}", name));
            if let Some(predicate) = program.registry.get_predicate(&predicate_id) {
                Some(predicate.parameters.len())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Register core builtin relations
    fn register_core_builtins(&mut self) {
        use crate::goal::{AnyGoal, Goal, GoalCast};
        use crate::lterm::LValue;
        use crate::relation::fail;

        // Builtin length predicate - efficiently calculates list length
        // Prefixed with __builtin_ to avoid conflicts with library predicates
        let length_rel = Rc::new(move |args: Vec<LTerm>| -> Goal {
            if args.len() != 2 {
                return fail().cast_into();
            }

            use crate::goal::Goal;
            use crate::solver::{Solve, Solver};
            use crate::state::State;
            use crate::stream::Stream;

            #[derive(Debug)]
            struct BuiltinLengthGoal {
                list: LTerm,
                length: LTerm,
            }

            impl Solve for BuiltinLengthGoal {
                fn solve(&self, _solver: &Solver, state: State) -> Stream {
                    // Walk the substitution map to get resolved values (same as assertions)
                    let list_walked = state.smap_ref().walk(&self.list).clone();
                    let length_walked = state.smap_ref().walk(&self.length).clone();

                    // Check if list is now a concrete list
                    if list_walked.is_list() {
                        let count = list_walked.iter().count();
                        let count_term: LTerm =
                            LTerm::from(LTermInner::Val(LValue::Number(count as isize)));

                        // Use the constraint system's unification (same as assertions)
                        match state.unify(&length_walked, &count_term) {
                            Ok(new_state) => Stream::unit(Box::new(new_state)),
                            Err(_) => Stream::empty(),
                        }
                    } else {
                        // If list is still a variable, we can't compute length yet
                        // A more sophisticated implementation would add length constraints
                        Stream::empty()
                    }
                }
            }

            // Return the goal using the same pattern as assertions
            Goal::dynamic(Rc::new(BuiltinLengthGoal {
                list: args[0].clone(),
                length: args[1].clone(),
            }))
        });

        self.environment.borrow_mut().add_builtin_relation(
            "__builtin_length".to_string(),
            length_rel,
            2,
        );
    }

    /// Load a program as the base program for subsequent queries
    pub fn load_program(
        &mut self, 
        program_source: &str, 
        config: ExecutionConfig
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
                    ast: if config.show_ast { Some(program_ast) } else { None },
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
        
        // Store as base program
        self.base_program = Some(ir_program);
        
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
        // Use the new API with default config
        self.load_program_from_ast_with_config(program, ExecutionConfig::default())?;
        Ok(())
    }
    
    /// Load a program from AST with configuration
    pub fn load_program_from_ast_with_config(&mut self, program_ast: ast::Program, config: ExecutionConfig) -> Result<LoadResult, InterpreterError> {
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
                    ast: if config.show_ast { Some(program_ast) } else { None },
                    parse_time: start_time.elapsed(),
                }),
            });
        }
        
        // Compile program
        let ir_program = self.compile_program_with_config(program_ast, &config)?;
        let compilation_time = start_time.elapsed();
        
        // Store as base program
        self.base_program = Some(ir_program);
        
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

    /// Find the standard library path by trying different locations
    fn find_stdlib_path() -> Result<PathBuf, InterpreterError> {
        // Try different locations for the standard library
        let candidates = vec![
            // 1. Relative to current directory (current behavior)
            PathBuf::from("std"),
            // 2. Relative to executable (preferred for installed binaries)
            Self::executable_relative_path("std"),
            // 3. Relative to executable's parent directory (for development)
            Self::executable_relative_path("../std"),
            // 4. In parent of executable's parent (for target/release structure)
            Self::executable_relative_path("../../std"),
        ];

        for candidate in candidates {
            if candidate.exists() && candidate.is_dir() {
                // Found a valid std directory, return it
                return Ok(candidate);
            }
        }

        Err(InterpreterError::IoError(
            "Standard library not found. Tried searching relative to current directory and executable location.".to_string()
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
        config: ExecutionConfig
    ) -> Result<QueryResultIterator, InterpreterError> {
        // Require loaded base program
        let base_program = self.base_program.as_ref()
            .ok_or_else(|| InterpreterError::RuntimeError("No base program loaded. Call load_program() first.".to_string()))?;

        // Clone base program for this execution (cheap due to Rc/im-rc)
        let mut runtime_program = base_program.clone();

        // Parse query
        let query_goal = query::parse_query(query)?;

        // Compile query into the runtime program
        self.compile_query_into_runtime_program(&mut runtime_program, query_goal, &config)?;

        // Execute with runtime program
        self.execute_with_runtime_program(runtime_program, "__query__", config)
    }

    /// Execute a program with a specific query (no pre-loading required)
    pub fn run_program(
        &mut self,
        program_source: &str,
        query: &str,
        config: ExecutionConfig
    ) -> Result<QueryResultIterator, InterpreterError> {
        // Parse program and query
        let program_ast = parser::parse_str(program_source)
            .map_err(|e| InterpreterError::ParseError(e.to_string()))?;
        let query_goal = query::parse_query(query)?;

        // Compile program
        let mut runtime_program = self.compile_program_with_config(program_ast, &config)?;

        // Compile query into the program
        self.compile_query_into_runtime_program(&mut runtime_program, query_goal, &config)?;

        // Execute
        self.execute_with_runtime_program(runtime_program, "__query__", config)
    }

    /// Execute a program's @main relation
    pub fn run_main(
        &mut self, 
        program_source: &str, 
        config: ExecutionConfig
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
        let mut runtime_program = self.compile_program_with_config(program_ast, &config)?;

        // Compile main query into the program
        self.compile_query_into_runtime_program(&mut runtime_program, main_query_goal, &config)?;

        // Execute
        self.execute_with_runtime_program(runtime_program, "__query__", config)
    }

    /// Execute tests in a program
    pub fn run_tests(
        &mut self, 
        program_source: &str, 
        config: ExecutionConfig
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
        config: ExecutionConfig
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
                ast: if config.show_ast { Some(program_ast) } else { None },
                parse_time,
            }),
            Err(e) => Ok(CheckResult {
                is_valid: false,
                errors: vec![e],
                warnings: compiler.get_warnings().to_vec(),
                ast: if config.show_ast { Some(program_ast) } else { None },
                parse_time,
            }),
        }
    }

    /// Compile program to IR without execution
    pub fn compile_program(
        &mut self, 
        program_source: &str, 
        config: ExecutionConfig
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
        config: &ExecutionConfig
    ) -> Result<compiler::ir::Program, InterpreterError> {
        let mut compiler = compiler::Compiler::new();
        
        // Set compilation options based on config
        // TODO: Set compilation options from config when available
        // For now, use default options
        
        compiler.compile_from_ast(program_ast)
            .map_err(|e| InterpreterError::RuntimeError(format!("Compilation failed: {:?}", e)))
    }

    /// Internal method to compile a query into a runtime program
    fn compile_query_into_runtime_program(
        &self,
        runtime_program: &mut compiler::ir::Program,
        query_goal: ast::Goal,
        _config: &ExecutionConfig,
    ) -> Result<(), InterpreterError> {
        // Compile the query into the runtime program
        
        // Use the improved add_query_to_program method that doesn't require AST conversion
        let final_program = compiler::Compiler::add_query_to_program(
            runtime_program.clone(), 
            query_goal
        ).map_err(|e| InterpreterError::RuntimeError(format!("Query compilation failed: {:?}", e)))?;
        
        *runtime_program = final_program;
        Ok(())
    }

    /// Internal method to execute with a runtime program
    fn execute_with_runtime_program(
        &self,
        runtime_program: compiler::ir::Program,
        query_predicate_name: &str,
        config: ExecutionConfig,
    ) -> Result<QueryResultIterator, InterpreterError> {
        // Create execution context with runtime program
        let mut execution_context = runtime::context::ExecutionContext::new(
            Rc::new(runtime_program), 
            self.environment.clone()
        );

        // Extract query predicate
        let query_predicate_id = compiler::ir::PredicateId::new(format!("::{}", query_predicate_name));
        let query_predicate = execution_context.program().registry.get_predicate(&query_predicate_id)
            .ok_or_else(|| InterpreterError::RuntimeError(format!("Query predicate '{}' not found", query_predicate_name)))?
            .clone();

        // Create solver
        let user_state = DefaultUser::default();
        let user_globals = <DefaultUser as crate::user::User>::UserContext::default();
        let mut solver = crate::solver::Solver::new(user_globals, false);

        // Set timeout if configured
        if let Some(timeout_ms) = config.timeout {
            solver.set_timeout(std::time::Instant::now(), timeout_ms);
        }

        // Create initial state and stream
        let initial_state = crate::state::State::new(user_state);
        
        // Create captured arguments for query predicate parameters
        let captured_args: Vec<runtime::context::ArgumentValue> = query_predicate.parameters.iter()
            .map(|param| {
                let named_var = execution_context.create_named_fresh_var(&param.name);
                execution_context.bind_var(param.name.clone(), named_var.clone());
                runtime::context::ArgumentValue::Relational(named_var)
            })
            .collect();

        // Create predicate closure and execute
        let predicate_closure = runtime::context::PredicateClosure::new(
            Rc::new(query_predicate),
            captured_args,
            Rc::new(execution_context.program().clone()),
            self.environment.clone(),
        );

        let stream = predicate_closure.expand_and_solve(&solver, initial_state);

        Ok(QueryResultIterator::new(solver, stream, config))
    }

    // ===== BACKWARD COMPATIBILITY METHODS =====

    /// Execute a program with a query (legacy method for backward compatibility)
    pub fn execute_program_with_query(
        &mut self,
        program_source: &str,
        query_str: &str,
    ) -> Result<Vec<QueryResult>, InterpreterError> {
        // Use the new streaming API but collect results to Vec
        let config = ExecutionConfig::default();
        let results = self.run_program(program_source, query_str, config)?;
        results.collect_limited(100)
    }

    /// Execute a query with test timeout (legacy method for backward compatibility)
    pub fn query_with_test_timeout(
        &mut self,
        query_str: &str,
        timeout_ms: Option<u64>,
        test_timeout_info: Option<(std::time::Instant, u64)>,
    ) -> Result<Vec<QueryResult>, InterpreterError> {
        // Create config from parameters
        let timeout = test_timeout_info
            .map(|(_, timeout_ms)| timeout_ms)
            .or(timeout_ms);
        
        let config = ExecutionConfig {
            timeout,
            ..Default::default()
        };
        
        // Use the new streaming API
        let results = self.query(query_str, config)?;
        results.collect_limited(100)
    }
    
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
