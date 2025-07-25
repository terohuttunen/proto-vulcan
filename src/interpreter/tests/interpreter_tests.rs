//! Basic interpreter functionality tests
//! 
//! Tests core interpreter creation, program loading, and basic functionality.

use super::super::*;
use super::super::ExecutionConfig;


use crate::interpreter::symbol_table::InternedSymbol;
use parser::ast::*;

type TestInterpreter = Interpreter;

#[test]
fn test_interpreter_creation() {
    let interpreter = TestInterpreter::with_stdlib();
    assert_eq!(interpreter.environment().current_scope(), "global");
}

#[test]
fn test_empty_program() {
    let mut interpreter = TestInterpreter::with_stdlib();
    let program = Program {
        items: vec![],
        span: Default::default(),
    };
    let result = interpreter.load_program_ast(program);
    assert!(result.is_ok());
}

#[test]
fn test_query_parsing_error() {
    let mut interpreter = TestInterpreter::with_stdlib();
    
    // Load a minimal program first since the new system requires a base program
    let minimal_program = parser::parse_str("rel dummy() { true == true }").unwrap();
    interpreter.load_program_ast(minimal_program).unwrap();
    
    // Now test the invalid query
    let result = interpreter.query("invalid query", ExecutionConfig::default()).map(|iter| iter.collect_limited(100).unwrap_or_default());
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, InterpreterError::ParseError(_)));
}

#[test]
fn test_comprehensive_program_loading() {
    let mut interpreter = TestInterpreter::with_stdlib();

    // Create a program with various components
    let program = Program {
        items: vec![
            // Struct definition
            Item::Struct(StructDefinition {
                span: Default::default(),
                visibility: ast::Visibility::Public,
                name: "Point".to_string().into(),
                kind: StructKind::Named(vec![
                    NamedField {
                        span: Default::default(),
                        visibility: ast::Visibility::Public,
                        name: "x".to_string().into(),
                        type_name: QualifiedPath::Relative(vec![InternedSymbol::from_text("Number")]),
                    },
                    NamedField {
                        span: Default::default(),
                        visibility: ast::Visibility::Public,
                        name: "y".to_string().into(),
                        type_name: QualifiedPath::Relative(vec![InternedSymbol::from_text("Number")]),
                    },
                ]),
            }),
            // Relation definition
            Item::Predicate(PredicateDefinition {
                span: Default::default(),
                visibility: ast::Visibility::Private,
                predicate_kind: PredicateKind::Relation,
                attributes: vec![],
                name: "distance".to_string().into(),
                parameters: vec![
                    Parameter {
                        name: "p1".to_string().into(),
                        type_annotation: None,
                    },
                    Parameter {
                        name: "p2".to_string().into(),
                        type_annotation: None,
                    },
                    Parameter {
                        name: "result".to_string().into(),
                        type_annotation: None,
                    },
                ],
                search_strategy: Some(SearchStrategy::Bfs),
                body: vec![Goal::Equality(
                    Term::Variable(InternedSymbol::from_text("result")),
                    Term::Literal(Literal::Number("0.0".to_string()), Default::default()),
                    Default::default(),
                )],
            }),
            // Module definition
            Item::Module(ModuleDefinition {
                visibility: ast::Visibility::Public,
                span: Default::default(),
                name: "geometry".to_string().into(),
                search_strategy: None,
                items: vec![Item::Predicate(PredicateDefinition {
                    span: Default::default(),
                    visibility: ast::Visibility::Public,
                    predicate_kind: PredicateKind::Relation,
                    attributes: vec![],
                    name: "area".to_string().into(),
                    parameters: vec![
                        Parameter {
                            name: "width".to_string().into(),
                            type_annotation: None,
                        },
                        Parameter {
                            name: "height".to_string().into(),
                            type_annotation: None,
                        },
                        Parameter {
                            name: "result".to_string().into(),
                            type_annotation: None,
                        },
                    ],
                    search_strategy: None,
                    body: vec![],
                })],
            }),
        ],
        span: Default::default(),
    };

    // Load the program
    let result = interpreter.load_program_ast(program);
    assert!(result.is_ok(), "Failed to load program: {:?}", result.err());

    // Verify the components were loaded correctly
    
    // Check that the relation was loaded
    assert!(interpreter.has_predicate("distance"), "distance relation should be loaded");

    // Check that the struct was loaded
    assert!(interpreter.has_type("Point"), "Point struct should be loaded");

    // Check that module scoping works
    let env = interpreter.environment();
    assert_eq!(env.current_scope(), "global");
}

#[test]
fn test_runtime_value_conversion() {
    // Test that we can convert various AST terms to runtime values
    use super::super::runtime_value::RuntimeValue;
    type TestRuntimeValue = RuntimeValue;

    // Test boolean literal
    let bool_term = Term::Literal(Literal::Boolean(true), Default::default());
    let bool_runtime = TestRuntimeValue::from_ast_term(&bool_term).unwrap();
    assert!(bool_runtime.as_term().is_some());
    assert!(bool_runtime.as_term().unwrap().is_val());

    // Test number literal
    let num_term = Term::Literal(Literal::Number("42".to_string()), Default::default());
    let num_runtime = TestRuntimeValue::from_ast_term(&num_term).unwrap();
    assert!(num_runtime.as_term().is_some());
    assert!(num_runtime.as_term().unwrap().is_val());

    // Test variable
    let var_term = Term::Variable(InternedSymbol::from_text("x"));
    let var_runtime = TestRuntimeValue::from_ast_term(&var_term).unwrap();
    assert!(var_runtime.as_term().is_some());
    assert!(var_runtime.as_term().unwrap().is_var());

    // Test list
    let list_term = Term::List(
        ListConstruction {
            elements: vec![
                Term::Literal(Literal::Number("1".to_string()), Default::default()),
                Term::Literal(Literal::Number("2".to_string()), Default::default()),
                Term::Literal(Literal::Number("3".to_string()), Default::default()),
            ],
            tail: None,
        },
        Default::default(),
    );
    let list_runtime = TestRuntimeValue::from_ast_term(&list_term).unwrap();
    assert!(list_runtime.as_term().is_some());
    assert!(list_runtime.as_term().unwrap().is_list());
}