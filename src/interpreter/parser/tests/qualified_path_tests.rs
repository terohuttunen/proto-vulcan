//! Qualified path parsing tests
//!
//! Tests for parsing qualified paths in relation calls and other contexts.

use super::super::*;
use crate::interpreter::parser::ast::{QualifiedName, QualifiedPath, RelationName};

#[test]
fn test_simple_relation_call() {
    let input = "rel test() { member(x, list) }";
    let result = parse_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_std_qualified_path() {
    let input = "rel test() { std::list::member(x, list) }";
    let result = parse_str(input);
    assert!(result.is_ok());

    // Extract the relation call and verify it parsed as std path
    if let Ok(program) = result {
        if let Some(Item::Predicate(pred)) = program.items.first() {
            if let Some(Goal::RelationCall(call, _)) = pred.body.first() {
                if let RelationName::Qualified(qualified) = &call.name {
                    assert!(
                        matches!(qualified.path, QualifiedPath::External(ref crate_name, _) if crate_name == "std")
                    );
                    assert_eq!(qualified.name, "member");
                    assert_eq!(qualified.path.segments(), &["list"]);
                } else {
                    panic!("Expected qualified relation name");
                }
            } else {
                panic!("Expected relation call in body");
            }
        } else {
            panic!("Expected predicate item");
        }
    }
}

#[test]
fn test_crate_absolute_path() {
    let input = "rel test() { crate::solver::solve(constraint, result) }";
    let result = parse_str(input);
    assert!(result.is_ok());

    if let Ok(program) = result {
        if let Some(Item::Predicate(pred)) = program.items.first() {
            if let Some(Goal::RelationCall(call, _)) = pred.body.first() {
                if let RelationName::Qualified(qualified) = &call.name {
                    assert!(matches!(qualified.path, QualifiedPath::Absolute(_)));
                    assert_eq!(qualified.name, "solve");
                    assert_eq!(qualified.path.segments(), &["solver"]);
                } else {
                    panic!("Expected qualified relation name");
                }
            }
        }
    }
}

#[test]
fn test_global_namespace_path() {
    let input = "rel test() { ::global::item(x) }";
    let result = parse_str(input);
    assert!(result.is_ok());

    if let Ok(program) = result {
        if let Some(Item::Predicate(pred)) = program.items.first() {
            if let Some(Goal::RelationCall(call, _)) = pred.body.first() {
                if let RelationName::Qualified(qualified) = &call.name {
                    assert!(matches!(qualified.path, QualifiedPath::Global(_)));
                    assert_eq!(qualified.name, "item");
                    assert_eq!(qualified.path.segments(), &["global"]);
                } else {
                    panic!("Expected qualified relation name");
                }
            }
        }
    }
}

#[test]
fn test_super_path() {
    let input = "rel test() { super::parent::function(x) }";
    let result = parse_str(input);
    assert!(result.is_ok());

    if let Ok(program) = result {
        if let Some(Item::Predicate(pred)) = program.items.first() {
            if let Some(Goal::RelationCall(call, _)) = pred.body.first() {
                if let RelationName::Qualified(qualified) = &call.name {
                    assert!(matches!(qualified.path, QualifiedPath::Super(0, _)));
                    assert_eq!(qualified.name, "function");
                    assert_eq!(qualified.path.segments(), &["parent"]);
                } else {
                    panic!("Expected qualified relation name");
                }
            }
        }
    }
}

#[test]
fn test_self_path() {
    let input = "rel test() { self::local::helper(x) }";
    let result = parse_str(input);
    assert!(result.is_ok());

    if let Ok(program) = result {
        if let Some(Item::Predicate(pred)) = program.items.first() {
            if let Some(Goal::RelationCall(call, _)) = pred.body.first() {
                if let RelationName::Qualified(qualified) = &call.name {
                    assert!(matches!(qualified.path, QualifiedPath::Self_(_)));
                    assert_eq!(qualified.name, "helper");
                    assert_eq!(qualified.path.segments(), &["local"]);
                } else {
                    panic!("Expected qualified relation name");
                }
            }
        }
    }
}