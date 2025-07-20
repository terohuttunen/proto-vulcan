use self::environment::Environment;
use self::parser::ast;
use self::query::QueryResult;
use crate::engine::Engine;
use crate::lterm::{LTerm, LTermInner};
use crate::user::User;
use std::cell::RefCell;
use std::fmt::{self, Display};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

mod assertions;
pub mod constraint_domains;
pub mod deferred;
mod environment;
mod execution;
pub mod import;
mod integration;
pub mod metaprogramming;
pub mod parser;
pub mod query;
mod runtime_value;
mod semantic_analysis;
#[cfg(test)]
mod struct_tests;
pub mod test_runner;
pub mod trace;

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
            main_relations.iter().map(|r| r.name.as_str()).collect::<Vec<_>>().join(", ")
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
                metaprogramming::TypeAnnotation::Custom(name) => name.clone(),
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
        let param_vars: Vec<String> = main_rel.parameters.iter().map(|p| p.name.clone()).collect();
        format!("{}({})", main_rel.name, param_vars.join(", "))
    }
}

#[derive(Debug, Clone)]
pub enum InterpreterError {
    ParseError(String),
    RuntimeError(String),
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

/// The main interpreter struct
pub struct Interpreter<U: User, E: Engine<U>> {
    pub environment: Rc<RefCell<Environment<U, E>>>,
}

impl<U, E> Interpreter<U, E>
where
    U: User,
    E: Engine<U>,
{
    /// Create a new interpreter
    pub fn new() -> Self {
        Self {
            environment: Rc::new(RefCell::new(Environment::new())),
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
    pub fn environment(&self) -> std::cell::Ref<Environment<U, E>> {
        self.environment.borrow()
    }

    /// Register core builtin relations
    fn register_core_builtins(&mut self) {
        use crate::goal::{AnyGoal, Goal, GoalCast};
        use crate::lterm::LValue;
        use crate::relation::fail;

        // Builtin length predicate - efficiently calculates list length
        // Prefixed with __builtin_ to avoid conflicts with library predicates
        let length_rel = Rc::new(move |args: Vec<LTerm<U, E>>| -> Goal<U, E> {
            if args.len() != 2 {
                return fail().cast_into();
            }

            use crate::goal::Goal;
            use crate::solver::{Solve, Solver};
            use crate::state::State;
            use crate::stream::Stream;
            use derivative::Derivative;

            #[derive(Derivative)]
            #[derivative(Debug(bound = "U: User"))]
            struct BuiltinLengthGoal<U: User, E: Engine<U>> {
                list: LTerm<U, E>,
                length: LTerm<U, E>,
            }

            impl<U: User, E: Engine<U>> Solve<U, E> for BuiltinLengthGoal<U, E> {
                fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
                    // Walk the substitution map to get resolved values (same as assertions)
                    let list_walked = state.smap_ref().walk(&self.list).clone();
                    let length_walked = state.smap_ref().walk(&self.length).clone();

                    // Check if list is now a concrete list
                    if list_walked.is_list() {
                        let count = list_walked.iter().count();
                        let count_term =
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

    /// Load a program into the interpreter
    pub fn load_program(&mut self, mut program: ast::Program) -> Result<(), InterpreterError> {
        // First pass: Load type definitions (enums, structs) into the environment
        // This is needed so semantic analysis can resolve type names
        let mut env = self.environment.borrow_mut();
        for item in &program.items {
            match item {
                ast::Item::Enum(enum_def) => env.load_enum(enum_def.clone())?,
                ast::Item::Struct(struct_def) => env.load_struct(struct_def.clone())?,
                _ => {} // Skip other items in first pass
            }
        }
        drop(env);
        
        // Second pass: Perform semantic analysis to disambiguate enum variants
        // Now that types are loaded, semantic analysis can resolve enum variants
        semantic_analysis::analyze_program(&mut program, self.environment.clone())?;
        
        // Third pass: Load the remaining items (relations, modules, etc.)
        let mut env = self.environment.borrow_mut();
        for item in program.items {
            match item {
                ast::Item::Predicate(rel) => env.load_predicate(rel)?,
                ast::Item::Module(module) => env.load_module(module)?,
                ast::Item::ModuleDeclaration(mod_decl) => {
                    env.load_module_declaration(&mod_decl, None, "global")?
                }
                ast::Item::Use(use_stmt) => env.load_use_statement(use_stmt)?,
                ast::Item::Impl(_) => {}, // TODO: Handle impl blocks
                ast::Item::Enum(_) | ast::Item::Struct(_) => {
                    // Already loaded in first pass
                }
            }
        }
        
        Ok(())
    }

    /// Load a specific module into the interpreter
    pub fn load_module(&mut self, _name: &str, path: &Path) -> Result<(), InterpreterError> {
        let source =
            fs::read_to_string(path).map_err(|e| InterpreterError::IoError(e.to_string()))?;
        let program =
            parser::parse_str(&source).map_err(|e| InterpreterError::ParseError(e.to_string()))?;
        self.load_program(program)
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

    /// Execute a query string
    pub fn query(&mut self, query_str: &str) -> Result<Vec<QueryResult<U, E>>, InterpreterError>
    where
        U::UserContext: Default,
    {
        self.query_with_timeout(query_str, None)
    }

    /// Execute a query string with optional timeout
    pub fn query_with_timeout(
        &mut self,
        query_str: &str,
        timeout_ms: Option<u64>,
    ) -> Result<Vec<QueryResult<U, E>>, InterpreterError>
    where
        U::UserContext: Default,
    {
        self.query_with_test_timeout(query_str, timeout_ms, None)
    }

    /// Execute a query string with optional timeout and test timeout
    pub fn query_with_test_timeout(
        &mut self,
        query_str: &str,
        timeout_ms: Option<u64>,
        test_timeout_info: Option<(std::time::Instant, u64)>,
    ) -> Result<Vec<QueryResult<U, E>>, InterpreterError>
    where
        U::UserContext: Default,
    {
        let mut query_goal = query::parse_query(query_str)?;

        // Apply semantic analysis to the query goal
        semantic_analysis::analyze_goal(&mut query_goal, &self.environment)?;

        // Create query config with appropriate timeout
        let timeout = test_timeout_info
            .map(|(_, timeout_ms)| timeout_ms)
            .or(timeout_ms);
        let config = query::QueryConfig {
            timeout,
            ..Default::default()
        };

        query::execute_query(self.environment.clone(), query_goal, config)
    }

    /// Execute a query string with tracing enabled
    pub fn query_with_trace(
        &mut self,
        query_str: &str,
        trace_config: &mut trace::TraceConfig,
    ) -> Result<Vec<QueryResult<U, E>>, InterpreterError>
    where
        U::UserContext: Default,
    {
        self.query_with_trace_and_timeout(query_str, trace_config, None)
    }

    /// Execute a query string with tracing and optional timeout
    pub fn query_with_trace_and_timeout(
        &mut self,
        query_str: &str,
        trace_config: &mut trace::TraceConfig,
        timeout_ms: Option<u64>,
    ) -> Result<Vec<QueryResult<U, E>>, InterpreterError>
    where
        U::UserContext: Default,
    {
        let mut query_goal = query::parse_query(query_str)?;
        
        // Apply semantic analysis to the query goal
        semantic_analysis::analyze_goal(&mut query_goal, &self.environment)?;
        
        // TODO: Integrate tracing with the new unified QueryConfig system
        let config = query::QueryConfig {
            timeout: timeout_ms,
            ..Default::default()
        };

        query::execute_query(self.environment.clone(), query_goal, config)
    }

    /// Execute a query string with unified configuration
    pub fn execute_query(
        &mut self,
        query_str: &str,
        config: query::QueryConfig,
    ) -> Result<Vec<QueryResult<U, E>>, InterpreterError>
    where
        U::UserContext: Default,
    {
        let mut query_goal = query::parse_query(query_str)?;
        
        // Apply semantic analysis to the query goal
        semantic_analysis::analyze_goal(&mut query_goal, &self.environment)?;
        
        query::execute_query(self.environment.clone(), query_goal, config)
    }
}

impl<U, E> Default for Interpreter<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
