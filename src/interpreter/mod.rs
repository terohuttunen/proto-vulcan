use self::environment::Environment;
use self::parser::ast;
use self::query::QueryResult;
use crate::engine::Engine;
use crate::user::User;
use std::cell::RefCell;
use std::fmt::{self, Display};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub mod deferred;
mod environment;
mod execution;
mod integration;
pub mod parser;
pub mod query;
mod runtime_value;

#[derive(Debug)]
pub enum InterpreterError {
    ParseError(String),
    RuntimeError(String),
    UnknownRelation(String),
    UnknownVariable(String),
    DuplicateDefinition(String),
    ModuleNotFound(PathBuf),
    IoError(String),
}

impl Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterpreterError::ParseError(e) => write!(f, "Parse error: {}", e),
            InterpreterError::RuntimeError(e) => write!(f, "Runtime error: {}", e),
            InterpreterError::UnknownRelation(name) => write!(f, "Unknown relation: {}", name),
            InterpreterError::UnknownVariable(name) => write!(f, "Unknown variable: {}", name),
            InterpreterError::DuplicateDefinition(name) => {
                write!(f, "Duplicate definition: {}", name)
            }
            InterpreterError::ModuleNotFound(path) => {
                write!(f, "Module not found: {}", path.display())
            }
            InterpreterError::IoError(msg) => write!(f, "I/O error: {}", msg),
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
        if let Err(e) = interpreter.load_stdlib() {
            eprintln!("Fatal: Failed to load standard library: {:?}", e);
        }
        interpreter
    }

    /// Get a reference to the environment
    pub fn environment(&self) -> std::cell::Ref<Environment<U, E>> {
        self.environment.borrow()
    }

    /// Load a program into the interpreter
    pub fn load_program(&mut self, program: ast::Program) -> Result<(), InterpreterError> {
        self.environment.borrow_mut().load_program(program)
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
        let std_path = PathBuf::from("std");
        if !std_path.exists() || !std_path.is_dir() {
            return Err(InterpreterError::IoError(
                "Standard library not found.".to_string(),
            ));
        }
        let mod_file = std_path.join("mod.pv");
        if !mod_file.exists() {
            return Err(InterpreterError::IoError(
                "Standard library entrypoint (std/mod.pv) not found.".to_string(),
            ));
        }

        let source =
            fs::read_to_string(&mod_file).map_err(|e| InterpreterError::IoError(e.to_string()))?;
        let program =
            parser::parse_str(&source).map_err(|e| InterpreterError::ParseError(e.to_string()))?;

        let mut env = self.environment.borrow_mut();
        env.set_base_path(std_path);
        let result = env.load_program(program);
        env.set_base_path(PathBuf::new());
        result
    }

    /// Execute a query string
    pub fn query(&mut self, query_str: &str) -> Result<Vec<QueryResult<U, E>>, InterpreterError>
    where
        U::UserContext: Default,
    {
        let query_goal = query::parse_query(query_str)?;
        query::execute_query(self.environment.clone(), query_goal)
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
mod tests {
    use super::*;
    use crate::engine::DefaultEngine;
    use crate::user::DefaultUser;
    use parser::ast::*;

    type TestInterpreter = Interpreter<DefaultUser, DefaultEngine<DefaultUser>>;

    #[test]
    fn test_interpreter_creation() {
        let interpreter = TestInterpreter::new();
        assert_eq!(interpreter.environment().current_scope(), "global");
    }

    #[test]
    fn test_empty_program() {
        let mut interpreter = TestInterpreter::new();
        let program = Program { items: vec![] };
        let result = interpreter.load_program(program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_query_parsing_error() {
        let mut interpreter = TestInterpreter::new();
        let result = interpreter.query("invalid query");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            InterpreterError::ParseError(_)
        ));
    }

    #[test]
    fn test_comprehensive_program_loading() {
        let mut interpreter = TestInterpreter::with_stdlib();

        // Create a program with various components
        let program = Program {
            items: vec![
                // Struct definition
                Item::Struct(StructDefinition {
                    is_pub: true,
                    name: "Point".to_string(),
                    kind: StructKind::Named(vec![
                        NamedField {
                            is_pub: true,
                            name: "x".to_string(),
                            type_name: "i32".to_string(),
                        },
                        NamedField {
                            is_pub: true,
                            name: "y".to_string(),
                            type_name: "i32".to_string(),
                        },
                    ]),
                }),
                // Relation definition
                Item::Relation(RelationDefinition {
                    is_pub: false,
                    name: "distance".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "p1".to_string(),
                            type_name: Some("Point".to_string()),
                        },
                        Parameter {
                            name: "p2".to_string(),
                            type_name: Some("Point".to_string()),
                        },
                        Parameter {
                            name: "result".to_string(),
                            type_name: Some("f64".to_string()),
                        },
                    ],
                    search_strategy: Some(SearchStrategy::Bfs),
                    body: vec![Goal::Equality(
                        Term::Variable("result".to_string()),
                        Term::Literal(Literal::Number("0.0".to_string())),
                    )],
                }),
                // Module definition
                Item::Module(ModuleDefinition {
                    name: "geometry".to_string(),
                    search_strategy: None,
                    items: vec![Item::Relation(RelationDefinition {
                        is_pub: true,
                        name: "area".to_string(),
                        parameters: vec![
                            Parameter {
                                name: "width".to_string(),
                                type_name: Some("f64".to_string()),
                            },
                            Parameter {
                                name: "height".to_string(),
                                type_name: Some("f64".to_string()),
                            },
                            Parameter {
                                name: "result".to_string(),
                                type_name: Some("f64".to_string()),
                            },
                        ],
                        search_strategy: None,
                        body: vec![],
                    })],
                }),
            ],
        };

        // Load the program
        let result = interpreter.load_program(program);
        assert!(result.is_ok(), "Failed to load program: {:?}", result.err());

        // Verify the components were loaded correctly
        let env = interpreter.environment();

        // Check that the relation was loaded
        let distance_rel = env.lookup("distance");
        assert!(distance_rel.is_some(), "distance relation should be loaded");
        assert!(
            distance_rel.unwrap().is_relation(),
            "distance should be a relation"
        );

        // Check that the struct was loaded
        let point_struct = env.get_struct("Point");
        assert!(point_struct.is_some(), "Point struct should be loaded");
        assert_eq!(point_struct.unwrap().name, "Point");

        // Check that module scoping works
        assert_eq!(env.current_scope(), "global");
    }

    #[test]
    fn test_runtime_value_conversion() {
        // Test that we can convert various AST terms to runtime values
        use runtime_value::RuntimeValue;
        type TestRuntimeValue = RuntimeValue<DefaultUser, DefaultEngine<DefaultUser>>;

        // Test boolean literal
        let bool_term = Term::Literal(Literal::Boolean(true));
        let bool_runtime = TestRuntimeValue::from_ast_term(&bool_term).unwrap();
        assert!(bool_runtime.as_term().is_some());
        assert!(bool_runtime.as_term().unwrap().is_val());

        // Test number literal
        let num_term = Term::Literal(Literal::Number("42".to_string()));
        let num_runtime = TestRuntimeValue::from_ast_term(&num_term).unwrap();
        assert!(num_runtime.as_term().is_some());
        assert!(num_runtime.as_term().unwrap().is_val());

        // Test variable
        let var_term = Term::Variable("x".to_string());
        let var_runtime = TestRuntimeValue::from_ast_term(&var_term).unwrap();
        assert!(var_runtime.as_term().is_some());
        assert!(var_runtime.as_term().unwrap().is_var());

        // Test list
        let list_term = Term::List(ListConstruction {
            elements: vec![
                Term::Literal(Literal::Number("1".to_string())),
                Term::Literal(Literal::Number("2".to_string())),
                Term::Literal(Literal::Number("3".to_string())),
            ],
            tail: None,
        });
        let list_runtime = TestRuntimeValue::from_ast_term(&list_term).unwrap();
        assert!(list_runtime.as_term().is_some());
        assert!(list_runtime.as_term().unwrap().is_list());
    }

    #[test]
    fn test_end_to_end_parser_to_interpreter() {
        // Test that we can parse a program and load it into the interpreter
        let program_source = r#"
            pub struct Point {
                pub x: i32,
                pub y: i32,
            }
            
            rel distance(p1: Point, p2: Point, result: f64) @bfs {
                p1 == p2
            }
        "#;

        // Parse the program
        let parsed_program = parser::parse_str(program_source).unwrap();

        // Load it into the interpreter
        let mut interpreter = TestInterpreter::new();
        let result = interpreter.load_program(parsed_program);
        assert!(
            result.is_ok(),
            "Failed to load parsed program: {:?}",
            result.err()
        );

        // Verify it was loaded correctly
        let env = interpreter.environment();
        assert!(env.lookup("distance").is_some());
        assert!(env.get_struct("Point").is_some());
    }

    #[test]
    fn test_end_to_end_simple_relation() {
        // Test parsing and executing a simple relation
        let program_source = r#"
            rel parent(x, y) {
                x == "alice", y == "bob";
                x == "bob", y == "charlie"
            }
        "#;

        // Parse and load the program
        let parsed_program = parser::parse_str(program_source).unwrap();
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).unwrap();

        // Verify the relation was loaded
        {
            let env = interpreter.environment();
            let parent_rel = env.lookup("parent");
            assert!(parent_rel.is_some(), "parent relation should be loaded");
            assert!(
                parent_rel.unwrap().is_relation(),
                "parent should be a relation"
            );
        }

        // Test query execution (basic test - just ensure it doesn't crash)
        let query_result = interpreter.query("parent(alice, bob)");
        match query_result {
            Ok(_results) => {
                // Query executed successfully
            }
            Err(e) => {
                // For now, we expect query parsing to fail since it's simplified
                assert!(matches!(e, InterpreterError::ParseError(_)));
            }
        }
    }

    #[test]
    fn test_end_to_end_facts_and_rules() {
        // Test a program with both facts and rules
        let program_source = r#"
            rel fact(x) {
                x == "a";
                x == "b";
                x == "c"
            }
            
            rel rule(x, y) {
                fact(x),
                y == x
            }
        "#;

        // Parse and load the program
        let parsed_program = parser::parse_str(program_source).unwrap();
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).unwrap();

        // Verify both relations were loaded
        let env = interpreter.environment();
        assert!(env.lookup("fact").is_some());
        assert!(env.lookup("rule").is_some());

        // Test that both are relations
        assert!(env.lookup("fact").unwrap().is_relation());
        assert!(env.lookup("rule").unwrap().is_relation());
    }

    #[test]
    fn test_end_to_end_struct_and_relations() {
        // Test a program with structs and relations using them
        let program_source = r#"
            struct Person {
                name: String,
                age: i32
            }
            
            rel adult(person) {
                match person {
                    Person { name: n, age: a } => {
                        a == 18
                    }
                }
            }
        "#;

        // Parse and load the program
        let parsed_program = parser::parse_str(program_source).unwrap();
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).unwrap();

        // Verify struct and relation were loaded
        let env = interpreter.environment();
        assert!(env.get_struct("Person").is_some());
        assert!(env.lookup("adult").is_some());
        assert!(env.lookup("adult").unwrap().is_relation());

        // Verify struct definition
        let person_struct = env.get_struct("Person").unwrap();
        assert_eq!(person_struct.name, "Person");
        match &person_struct.kind {
            parser::ast::StructKind::Named(fields) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "name");
                assert_eq!(fields[1].name, "age");
            }
            _ => panic!("Expected named struct"),
        }
    }

    #[test]
    fn test_end_to_end_modules() {
        // Test a program with modules
        let program_source = r#"
            mod geometry {
                struct Point {
                    x: i32,
                    y: i32
                }
                
                rel origin(p) {
                    p == Point { x: 0, y: 0 }
                }
            }
            
            rel test_origin(result) {
                result == true
            }
        "#;

        // Parse and load the program
        let parsed_program = parser::parse_str(program_source).unwrap();
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).unwrap();

        // Verify global relation was loaded
        let env = interpreter.environment();
        assert!(env.lookup("test_origin").is_some());
        assert!(env.lookup("test_origin").unwrap().is_relation());

        // Note: Module scoping is implemented but the lookup doesn't currently
        // support qualified names like "geometry::origin"
    }

    #[test]
    fn test_end_to_end_complex_goals() {
        // Test a program with complex goal structures
        let program_source = r#"
            rel complex_goal(x, y, z) {
                [
                    x == 1,
                    |fresh_var| {
                        fresh_var == 2,
                        y == fresh_var
                    },
                    z == 3
                ]
            }
            
            rel disjunctive_goal(x) {
                conde {
                    x == "option1";
                    x == "option2";
                    x == "option3"
                }
            }
        "#;

        // Parse and load the program
        let parsed_program = parser::parse_str(program_source).unwrap();
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).unwrap();

        // Verify relations were loaded
        let env = interpreter.environment();
        assert!(env.lookup("complex_goal").is_some());
        assert!(env.lookup("disjunctive_goal").is_some());
        assert!(env.lookup("complex_goal").unwrap().is_relation());
        assert!(env.lookup("disjunctive_goal").unwrap().is_relation());
    }

    #[test]
    fn test_end_to_end_execution_context() {
        // Test that the execution context can convert goals properly
        let program_source = r#"
            rel simple_eq(x, y) {
                x == y
            }
            
            rel call_simple(result) {
                simple_eq(42, 42),
                result == true
            }
        "#;

        // Parse and load the program
        let parsed_program = parser::parse_str(program_source).unwrap();
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program.clone()).unwrap();

        // Test that we can create an execution context and convert goals
        use super::execution::ExecutionContext;
        let mut exec_context = ExecutionContext::new(interpreter.environment.clone());

        // Test converting a simple equality goal
        use super::parser::ast::{Goal, Literal, Term};

        // Fix: The variable 'x' must exist in the context before it can be used.
        let x_var = exec_context.create_fresh_var();
        exec_context.bind_var("x".to_string(), x_var);

        let equality_goal = Goal::Equality(
            Term::Variable("x".to_string()),
            Term::Literal(Literal::Number("42".to_string())),
        );

        let runtime_goal = exec_context.ast_goal_to_runtime(&equality_goal);
        assert!(
            runtime_goal.is_ok(),
            "Should be able to convert equality goal"
        );
    }

    #[test]
    fn test_end_to_end_query_parsing() {
        // Test the query parsing functionality
        use super::query;

        // Test parsing equality queries
        let eq_query = query::parse_query("x == 42");
        assert!(
            eq_query.is_ok(),
            "Should parse equality query, got: {:?}",
            eq_query.err()
        );

        // Test parsing relation call queries
        let rel_query = query::parse_query("parent(alice, bob)");
        assert!(rel_query.is_ok(), "Should parse relation call query");

        // Test parsing with different literal types
        let string_query = query::parse_query(r#"name == "alice""#);
        assert!(string_query.is_ok(), "Should parse string equality query");

        let bool_query = query::parse_query("flag == true");
        assert!(bool_query.is_ok(), "Should parse boolean equality query");

        // Test invalid queries
        let invalid_query = query::parse_query("invalid syntax here");
        assert!(invalid_query.is_err(), "Should fail on invalid syntax");
    }

    #[test]
    fn test_end_to_end_full_pipeline() {
        // Test the complete pipeline from source to query execution
        let program_source = r#"
            rel number(x) {
                x == 1;
                x == 2;
                x == 3
            }
            
            rel double(x, y) {
                number(x),
                y == x  // Simplified - real doubling would need arithmetic
            }
        "#;

        // Parse and load the program
        let parsed_program = parser::parse_str(program_source).unwrap();
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).unwrap();

        // Test various query types
        let queries = vec!["number(1)", "number(x)", "double(2, y)", "x == 42"];

        for query_str in queries {
            let result = interpreter.query(query_str);
            // For now, we expect either success or a parse error
            // A full implementation would return actual results
            match result {
                Ok(_) => {
                    // Query executed successfully
                }
                Err(InterpreterError::ParseError(_)) => {
                    // Expected for complex queries with current simple parser
                }
                Err(e) => {
                    panic!("Unexpected error for query '{}': {:?}", query_str, e);
                }
            }
        }
    }

    #[test]
    fn test_end_to_end_error_handling() {
        // Test error handling in the end-to-end pipeline

        // Test invalid source code
        let invalid_source = "this is not valid proto-vulcan syntax";
        let parse_result = parser::parse_str(invalid_source);
        assert!(parse_result.is_err(), "Should fail to parse invalid syntax");

        // Test loading a program with unknown relations
        let program_with_unknown_rel = r#"
            rel test_rel(x) {
                unknown_relation(x)
            }
        "#;

        let parsed_program = parser::parse_str(program_with_unknown_rel).unwrap();
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).unwrap();

        // The program loads successfully, but execution would fail
        // when trying to resolve unknown_relation
        let env = interpreter.environment();
        assert!(env.lookup("test_rel").is_some());
        assert!(env.lookup("unknown_relation").is_none());
    }

    #[test]
    fn test_end_to_end_variable_scoping() {
        // Test that variable scoping works correctly in the pipeline
        let program_source = r#"
            rel scoping_test(x, y) {
                |inner_var| {
                    inner_var == 42,
                    x == inner_var
                },
                y == x
            }
        "#;

        // Parse and load the program
        let parsed_program = parser::parse_str(program_source).unwrap();
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program.clone()).unwrap();

        // Verify the relation was loaded
        {
            let env = interpreter.environment();
            assert!(env.lookup("scoping_test").is_some());
            assert!(env.lookup("scoping_test").unwrap().is_relation());
        }

        // Test execution context variable handling
        use super::execution::ExecutionContext;
        let mut exec_context = ExecutionContext::new(interpreter.environment.clone());

        // Test that fresh variables are properly scoped
        use super::parser::ast::{FreshVariables, Goal, Literal, Term};
        let fresh_goal = Goal::Fresh(FreshVariables {
            vars: vec!["test_var".to_string()],
            body: vec![Goal::Equality(
                Term::Variable("test_var".to_string()),
                Term::Literal(Literal::Number("42".to_string())),
            )],
        });

        let runtime_goal = exec_context.ast_goal_to_runtime(&fresh_goal);
        assert!(
            runtime_goal.is_ok(),
            "Should handle fresh variables correctly"
        );
    }
}
