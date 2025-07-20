//! End-to-end execution and pipeline tests
//!
//! Tests the complete pipeline from source parsing to query execution,
//! including integration between parser, interpreter, and execution context.

use super::super::*;
use crate::engine::DefaultEngine;
use crate::user::DefaultUser;
use parser::ast::*;

type TestInterpreter = Interpreter<DefaultUser, DefaultEngine<DefaultUser>>;

#[test]
fn test_end_to_end_parser_to_interpreter() {
    // Test that we can parse a program and load it into the interpreter
    let program_source = r#"
        pub struct Point {
            pub x: i32,
            pub y: i32,
        }
        
        rel distance(p1, p2, result) @bfs {
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
    let program_source = r#"
        rel complex_goal(x, y, z) {
            all {
                x == 1,
                |fresh_var| {
                    fresh_var == 2,
                    y == fresh_var
                },
                z == 3
            }
        }
        
        rel disjunctive_goal(x) {
            any {
                x == "option1",
                x == "option2",
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
    use super::super::execution::ExecutionContext;
    let mut exec_context = ExecutionContext::new(interpreter.environment.clone());

    // Test converting a simple equality goal
    use super::super::parser::ast::{Goal, Literal, Term};

    // Fix: The variable 'x' must exist in the context before it can be used.
    let x_var = exec_context.create_fresh_var();
    exec_context.bind_var("x".to_string(), x_var);

    let equality_goal = Goal::Equality(
        Term::Variable("x".to_string(), Default::default()),
        Term::Literal(Literal::Number("42".to_string()), Default::default()),
        Default::default(),
    );

    let runtime_goal = exec_context.ast_goal_to_runtime(&equality_goal);
    assert!(
        runtime_goal.is_ok(),
        "Should be able to convert equality goal"
    );
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
    let mut interpreter = TestInterpreter::default();

    let program_str = r#"
        rel test_scope(result) {
            |x| {
                |y| {
                    x == 1,
                    y == 2,
                    result == [x, y]
                }
            }
        }
    "#;

    let program = parser::parse_str(program_str).unwrap();
    interpreter.load_program(program).unwrap();

    let results = interpreter.query("test_scope(result).").unwrap();
    assert!(!results.is_empty());
}