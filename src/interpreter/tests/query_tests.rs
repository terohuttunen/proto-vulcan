//! Query parsing and execution tests
//!
//! Tests query parsing functionality and basic query execution patterns.

use super::super::*;



type TestInterpreter = Interpreter;

#[test]
fn test_end_to_end_query_parsing() {
    // Test the query parsing functionality
    use super::super::query;

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