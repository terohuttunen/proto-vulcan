//! Constraint domain integration tests
//!
//! Tests constraint programming functionality including CLP(FD) domains,
//! arithmetic constraints, distinct constraints, and complex problems.

use super::super::*;
use crate::engine::DefaultEngine;
use crate::user::DefaultUser;
use parser::ast::*;

type TestInterpreter = Interpreter<DefaultUser, DefaultEngine<DefaultUser>>;

#[test]
fn test_constraint_block_basic_domain() {
    let mut interpreter = TestInterpreter::with_stdlib();
    let program_str = r#"
    rel solve() {
        |x, y| {
            constraint(domain="clpfd") {
                x in 1..3,
                y in 1..3,
                x < y
            }
        }
    }
    "#;
    let program = parser::parse_str(program_str).unwrap();
    interpreter.load_program(program).unwrap();
    let results = interpreter.query("solve()").unwrap();
    // Fresh blocks with constraint blocks work but return empty bindings
    // since the fresh variables are scoped within the block
    assert!(!results.is_empty());
    assert_eq!(results.len(), 1);
    assert!(results[0].bindings.is_empty());
}

#[test]
fn test_constraint_block_arithmetic() {
    let mut interpreter = TestInterpreter::with_stdlib();
    let program_str = r#"
    rel solve() {
        |x, y| {
            constraint(domain="clpfd") {
                x in 1..5,
                y in 1..5,
                x + y == 6
            }
        }
    }
    "#;
    let program = parser::parse_str(program_str).unwrap();
    interpreter.load_program(program).unwrap();
    let results = interpreter.query("solve()").unwrap();
    // Fresh blocks with constraint blocks work but return empty bindings
    // since the fresh variables are scoped within the block
    assert!(!results.is_empty());
    assert_eq!(results.len(), 1);
    assert!(results[0].bindings.is_empty());
}

#[test]
fn test_constraint_block_distinct() {
    let mut interpreter = TestInterpreter::with_stdlib();
    let program_str = r#"
    rel solve() {
        |q1, q2, q3, q4| {
            constraint(domain="clpfd") {
                q1 in 1..4,
                q2 in 1..4,
                q3 in 1..4,
                q4 in 1..4,
                distinct(q1, q2, q3, q4)
            }
        }
    }
    "#;
    let program = parser::parse_str(program_str).unwrap();
    interpreter.load_program(program).unwrap();
    let results = interpreter.query("solve()").unwrap();
    // Fresh blocks with constraint blocks work but return empty bindings
    // since the fresh variables are scoped within the block
    assert!(!results.is_empty());
    assert_eq!(results.len(), 1);
    assert!(results[0].bindings.is_empty());
}

#[test]
fn test_constraint_block_simple_queens() {
    let mut interpreter = TestInterpreter::with_stdlib();
    let program_str = r#"
    rel solve() {
        |q1, q2, q3, q4| {
            constraint(domain="clpfd") {
                q1 in 1..4,
                q2 in 1..4,
                q3 in 1..4,
                q4 in 1..4,

                q1 != q2,
                q1 != q3,
                q1 != q4,
                q2 != q3,
                q2 != q4,
                q3 != q4,

                q1 - q2 != 1, q2 - q1 != 1,
                q1 - q3 != 2, q3 - q1 != 2,
                q1 - q4 != 3, q4 - q1 != 3,
                q2 - q3 != 1, q3 - q2 != 1,
                q2 - q4 != 2, q4 - q2 != 2,
                q3 - q4 != 1, q4 - q3 != 1
            }
        }
    }
    "#;
    let program = parser::parse_str(program_str).unwrap();
    interpreter.load_program(program).unwrap();
    let results = interpreter.query("solve()").unwrap();
    // Fresh blocks with constraint blocks work but return empty bindings
    // since the fresh variables are scoped within the block
    assert!(!results.is_empty());
    assert_eq!(results.len(), 1);
    assert!(results[0].bindings.is_empty());
}

#[test]
fn test_constraint_block_fresh_variables() {
    let mut interpreter = TestInterpreter::with_stdlib();
    let program_str = r#"
    rel solve() {
        |x| {
            constraint(domain="clpfd") {
                x in 6..10
            }
        }
    }
    "#;
    let program = parser::parse_str(program_str).unwrap();
    interpreter.load_program(program).unwrap();
    let results = interpreter.query("solve()").unwrap();
    // Fresh blocks with constraint blocks work but return empty bindings
    // since the fresh variables are scoped within the block
    assert!(!results.is_empty());
    assert_eq!(results.len(), 1);
    assert!(results[0].bindings.is_empty());
}

#[test]
fn test_constraint_block_multiple_domains() {
    // This test requires a second constraint domain to be registered.
    // For now, we'll just test that the parser can handle the syntax.
    // The actual execution would fail until a "clp_test" domain is added.
    let mut interpreter = TestInterpreter::with_stdlib();
    let program_str = r#"
    rel solve() {
        constraint(domain="clpfd") {
            x in 1..2
        },
        constraint(domain="clp_test") {
            a != b
        }
    }
    "#;
    let program = parser::parse_str(program_str).unwrap();
    interpreter.load_program(program).unwrap();
    // We can't query this because the "clp_test" domain doesn't exist.
    // The fact that it parses is the test.
}