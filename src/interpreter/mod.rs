pub mod environment;
pub mod parser;
pub mod runtime_value;
// pub mod execution;
pub mod integration;
pub mod query;

use crate::engine::Engine;
use crate::user::User;
use environment::Environment;
use parser::ast::Program;
use query::QueryResult;

/// Main interpreter for proto-vulcan programs
pub struct Interpreter<U: User, E: Engine<U>> {
    environment: Environment<U, E>,
}

impl<U: User, E: Engine<U>> Interpreter<U, E> {
    /// Create a new interpreter instance
    pub fn new() -> Self {
        Self {
            environment: Environment::new(),
        }
    }

    /// Load and execute a program
    pub fn load_program(&mut self, program: Program) -> Result<(), InterpreterError> {
        self.environment.load_program(program)
    }

    /// Execute a query string and return results
    pub fn query(&mut self, query_str: &str) -> Result<Vec<QueryResult<U, E>>, InterpreterError> {
        let query_ast = query::parse_query(query_str)?;
        query::execute_query(&mut self.environment, query_ast)
    }

    /// Get the current environment (for debugging/inspection)
    pub fn environment(&self) -> &Environment<U, E> {
        &self.environment
    }
}

/// Errors that can occur during interpretation
#[derive(Debug, Clone)]
pub enum InterpreterError {
    ParseError(String),
    RuntimeError(String),
    UnknownRelation(String),
    UnknownVariable(String),
    TypeMismatch(String),
    ScopeError(String),
}

impl std::fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpreterError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            InterpreterError::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
            InterpreterError::UnknownRelation(name) => write!(f, "Unknown relation: {}", name),
            InterpreterError::UnknownVariable(name) => write!(f, "Unknown variable: {}", name),
            InterpreterError::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            InterpreterError::ScopeError(msg) => write!(f, "Scope error: {}", msg),
        }
    }
}

impl std::error::Error for InterpreterError {}

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
        let mut interpreter = TestInterpreter::new();

        // Create a program with various components
        let program = Program {
            items: vec![
                // Use statement
                Item::Use(UseStatement {
                    path: UsePath::Simple(vec!["std".to_string(), "collections".to_string()]),
                }),
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
        let list_term = Term::List(vec![
            Term::Literal(Literal::Number("1".to_string())),
            Term::Literal(Literal::Number("2".to_string())),
            Term::Literal(Literal::Number("3".to_string())),
        ]);
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
}
