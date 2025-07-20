//! Tests for struct support in the interpreter
//! 
//! This module tests both tuple structs (Foo(x, y)) and named structs (Foo {x: a, y: b})

use super::*;
use crate::{
    engine::{DefaultEngine, Engine},
    user::{DefaultUser, User},
};

type TestInterpreter = Interpreter<DefaultUser, DefaultEngine<DefaultUser>>;

/// Test tuple struct construction and pattern matching
#[cfg(test)]
mod tuple_struct_tests {
    use super::*;

    #[test]
    fn test_tuple_struct_construction() {
        let program = r#"
            struct Point(i32, i32);
            
            rel test_point() {
                Point(1, 2) == Point(1, 2);
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("test_point()").expect("Failed to execute query");
        assert!(!results.is_empty(), "Should find at least one solution");
    }

    #[test]
    fn test_tuple_struct_pattern_matching() {
        let program = r#"
            struct Point(i32, i32);
            
            rel get_x(point, x) {
                Point(x, _) == point;
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("get_x(Point(42, 13), X)").expect("Failed to execute query");
        assert!(!results.is_empty(), "Should find solution where X = 42");
        
        // Check that X is bound to 42
        let result = &results[0];
        let x_value = result.bindings.get("X").expect("X should be bound");
        // Note: The exact format might vary, but should represent 42
        assert!(format!("{:?}", x_value).contains("42"));
    }

    #[test]
    fn test_nested_tuple_structs() {
        let program = r#"
            struct Point(i32, i32);
            struct Line(Point, Point);
            
            rel test_line() {
                Line(Point(0, 0), Point(1, 1)) == Line(Point(0, 0), Point(1, 1));
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("test_line()").expect("Failed to execute query");
        assert!(!results.is_empty(), "Should find solution for nested tuple structs");
    }

    #[test]
    fn test_tuple_struct_unification() {
        let program = r#"
            struct Pair(i32, i32);
            
            rel same_pair(p1, p2) {
                p1 == p2;
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("same_pair(Pair(X, Y), Pair(1, 2))").expect("Failed to execute query");
        assert!(!results.is_empty(), "Should unify and bind X=1, Y=2");
    }
}

/// Test named struct construction and pattern matching
#[cfg(test)]
mod named_struct_tests {
    use super::*;

    #[test]
    fn test_named_struct_construction() {
        let program = r#"
            struct PersonConstruct { name: String, age: i32 }
            
            rel test_person_construct_equality() {
                PersonConstruct {name: "Alice", age: 30} == PersonConstruct {name: "Alice", age: 30};
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("test_person_construct_equality()").expect("Failed to execute query");
        assert!(!results.is_empty(), "Named struct construction and equality should work");
    }

    #[test]
    fn test_named_struct_pattern_matching() {
        let program = r#"
            struct PersonPattern { name: String, age: i32 }
            
            rel get_pattern_name(person, name) {
                PersonPattern {name: name, age: _} == person;
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query(r#"get_pattern_name(PersonPattern {name: "Bob", age: 25}, Name)"#).expect("Failed to execute query");
        assert!(!results.is_empty(), "Should extract name from named struct");
        
        // Check that Name is bound to "Bob"
        let result = &results[0];
        let name_value = result.bindings.get("Name").expect("Name should be bound");
        assert!(format!("{:?}", name_value).contains("Bob"));
    }

    #[test]
    fn test_named_struct_field_order_independence() {
        let program = r#"
            struct PointOrder { x: i32, y: i32 }
            
            rel same_point_order(p1, p2) {
                p1 == p2;
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        // Test that field order doesn't matter
        let results = interpreter.query("same_point_order(PointOrder {x: 1, y: 2}, PointOrder {y: 2, x: 1})").expect("Failed to execute query");
        assert!(!results.is_empty(), "Named structs should match regardless of field order");
    }

    #[test]
    fn test_compound_struct_comprehensive() {
        let program = r#"
            struct AddressComp { street: String, city: String }
            
            rel test_address_equality() {
                AddressComp {street: "Main St", city: "Boston"} == AddressComp {street: "Main St", city: "Boston"};
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("test_address_equality()").expect("Failed to execute query");
        assert!(!results.is_empty(), "Compound struct equality should work");
    }

    #[test]
    fn test_nested_named_structs() {
        let program = r#"
            struct AddressNested { street: String, city: String }
            struct PersonNested { name: String, address: AddressNested }
            
            rel test_nested_structs() {
                PersonNested {
                    name: "Alice", 
                    address: AddressNested {street: "Main St", city: "Boston"}
                } == PersonNested {
                    name: "Alice", 
                    address: AddressNested {street: "Main St", city: "Boston"}
                };
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("test_nested_structs()").expect("Failed to execute query");
        assert!(!results.is_empty(), "Should handle nested named structs");
    }
}

/// Test mixed struct types and edge cases
#[cfg(test)]
mod mixed_struct_tests {
    use super::*;

    #[test]
    fn test_tuple_and_named_structs_together() {
        let program = r#"
            struct PointMixed(i32, i32);
            struct RectangleMixed { top_left: PointMixed, bottom_right: PointMixed }
            
            rel test_mixed_types() {
                RectangleMixed {
                    top_left: PointMixed(0, 0), 
                    bottom_right: PointMixed(10, 10)
                } == RectangleMixed {
                    top_left: PointMixed(0, 0), 
                    bottom_right: PointMixed(10, 10)
                };
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("test_mixed_types()").expect("Failed to execute query");
        assert!(!results.is_empty(), "Should handle mixing tuple and named structs");
    }

    #[test]
    fn test_struct_with_variables() {
        let program = r#"
            struct Pair(i32, i32);
            
            rel extract_both(pair, x, y) {
                Pair(x, y) == pair;
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("extract_both(Pair(42, 99), X, Y)").expect("Failed to execute query");
        assert!(!results.is_empty(), "Should extract both values from tuple struct");
        
        // Verify both variables are bound
        let result = &results[0];
        assert!(result.bindings.contains_key("X"), "X should be bound");
        assert!(result.bindings.contains_key("Y"), "Y should be bound");
    }

    #[test]
    fn test_struct_inequality() {
        let program = r#"
            struct Point(i32, i32);
            
            rel different_points(p1, p2) {
                p1 != p2;
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("different_points(Point(1, 2), Point(3, 4))").expect("Failed to execute query");
        assert!(!results.is_empty(), "Should recognize different structs as unequal");
    }
}

/// Test struct syntax parsing and interpreter integration
#[cfg(test)]
mod interpreter_syntax_tests {
    use super::*;

    #[test]
    fn test_tuple_struct_syntax_basic() {
        let program = r#"
            struct Point(i32, i32);
            struct Color(String);
            
            rel test_tuple_syntax() {
                Point(1, 2) == Point(1, 2);
                Color("red") == Color("red");
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("test_tuple_syntax()").expect("Failed to execute query");
        assert!(!results.is_empty(), "Tuple struct syntax should work correctly");
    }

    #[test]
    fn test_named_struct_syntax_basic() {
        let program = r#"
            struct Person { name: String, age: i32 }
            struct Book { title: String, author: String, pages: i32 }
            
            rel test_named_syntax() {
                Person {name: "Alice", age: 30} == Person {name: "Alice", age: 30};
                Book {title: "1984", author: "Orwell", pages: 328} == 
                Book {title: "1984", author: "Orwell", pages: 328};
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("test_named_syntax()").expect("Failed to execute query");
        assert!(!results.is_empty(), "Named struct syntax should work correctly");
    }

    #[test]
    fn test_mixed_struct_syntax() {
        let program = r#"
            struct Point(i32, i32);
            struct Circle { center: Point, radius: i32 }
            struct Rectangle { top_left: Point, bottom_right: Point }
            
            rel test_mixed_syntax() {
                Circle {center: Point(0, 0), radius: 5} == 
                Circle {center: Point(0, 0), radius: 5};
                
                Rectangle {top_left: Point(0, 10), bottom_right: Point(10, 0)} ==
                Rectangle {top_left: Point(0, 10), bottom_right: Point(10, 0)};
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("test_mixed_syntax()").expect("Failed to execute query");
        assert!(!results.is_empty(), "Mixed struct syntax should work correctly");
    }

    #[test]
    fn test_struct_pattern_syntax() {
        let program = r#"
            struct Person { name: String, age: i32 }
            struct Point(i32, i32);
            
            rel get_person_name(person, name) {
                Person {name: name, age: _} == person;
            }
            
            rel get_point_x(point, x) {
                Point(x, _) == point;
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        // Test named struct pattern matching
        let results1 = interpreter.query(r#"get_person_name(Person {name: "Bob", age: 25}, Name)"#)
            .expect("Failed to execute named pattern query");
        assert!(!results1.is_empty(), "Named struct pattern matching should work");
        
        let name_binding = results1[0].bindings.get("Name").expect("Name should be bound");
        assert!(format!("{:?}", name_binding).contains("Bob"), "Name should be extracted correctly");
        
        // Test tuple struct pattern matching
        let results2 = interpreter.query("get_point_x(Point(42, 13), X)")
            .expect("Failed to execute tuple pattern query");
        assert!(!results2.is_empty(), "Tuple struct pattern matching should work");
        
        let x_binding = results2[0].bindings.get("X").expect("X should be bound");
        assert!(format!("{:?}", x_binding).contains("42"), "X coordinate should be extracted correctly");
    }

    #[test]
    fn test_struct_field_order_syntax() {
        let program = r#"
            struct Config { host: String, port: i32, ssl: String }
            
            rel same_config(c1, c2) {
                c1 == c2
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        // Test that field order doesn't matter in syntax
        let results = interpreter.query(r#"same_config(Config {host: "localhost", port: 8080, ssl: "true"}, Config {ssl: "true", host: "localhost", port: 8080})"#)
            .expect("Failed to execute field order query");
        
        assert!(!results.is_empty(), "Field order should not matter in named struct syntax");
    }

    #[test]
    fn test_nested_struct_syntax() {
        let program = r#"
            struct Address { street: String, city: String, zip: String }
            struct Person { name: String, address: Address }
            struct Company { name: String, hq: Address, employees: i32 }
            
            rel test_nested_syntax() {
                Person {
                    name: "Alice", 
                    address: Address {street: "123 Main St", city: "Boston", zip: "02101"}
                } == Person {
                    name: "Alice", 
                    address: Address {street: "123 Main St", city: "Boston", zip: "02101"}
                };
                
                Company {
                    name: "TechCorp",
                    hq: Address {street: "456 Tech Ave", city: "SF", zip: "94105"},
                    employees: 100
                } == Company {
                    name: "TechCorp",
                    hq: Address {street: "456 Tech Ave", city: "SF", zip: "94105"},
                    employees: 100
                };
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        let results = interpreter.query("test_nested_syntax()").expect("Failed to execute query");
        assert!(!results.is_empty(), "Nested struct syntax should work correctly");
    }

    #[test]
    fn test_struct_in_relations() {
        let program = r#"
            struct Point(i32, i32);
            struct Vector { x: i32, y: i32 }
            
            rel distance_from_origin(point, dist) {
                |x, y| {
                    Point(x, y) == point,
                    // Simple distance calculation (just for syntax testing)
                    dist == x // Simplified for test
                }
            }
            
            rel vector_magnitude(vec, mag) {
                |x, y| {
                    Vector {x: x, y: y} == vec,
                    mag == x // Simplified for test  
                }
            }
            
            rel point_to_vector(point, vector) {
                |x, y| {
                    Point(x, y) == point,
                    Vector {x: x, y: y} == vector
                }
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        // Test tuple struct in relations
        let results1 = interpreter.query("distance_from_origin(Point(3, 4), Dist)")
            .expect("Failed to execute distance query");
        assert!(!results1.is_empty(), "Tuple struct should work in relations");
        
        // Test named struct in relations
        let results2 = interpreter.query("vector_magnitude(Vector {x: 5, y: 12}, Mag)")
            .expect("Failed to execute magnitude query");
        assert!(!results2.is_empty(), "Named struct should work in relations");
        
        // Test conversion between struct types
        let results3 = interpreter.query("point_to_vector(Point(1, 2), Vec)")
            .expect("Failed to execute conversion query");
        assert!(!results3.is_empty(), "Struct conversion should work in relations");
        
        let vec_binding = results3[0].bindings.get("Vec").expect("Vec should be bound");
        let vec_str = format!("{:?}", vec_binding);
        assert!(vec_str.contains("1") && vec_str.contains("2"), "Vector should have correct values");
    }

    #[test]
    fn test_complex_struct_queries() {
        let program = r#"
            struct User { id: i32, name: String }
            struct Post { id: i32, author: User, title: String }
            struct Comment { post_id: i32, author: User, text: String }
            
            rel author_posted(user, post) {
                Post {author: user, id: _, title: _} == post;
            }
            
            rel user_commented(user, comment) {
                Comment {author: user, post_id: _, text: _} == comment;
            }
            
            rel same_author(post, comment) {
                |author| {
                    Post {author: author, id: _, title: _} == post,
                    Comment {author: author, post_id: _, text: _} == comment
                }
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        // Test complex pattern matching with nested structs
        let alice = r#"User {id: 1, name: "Alice"}"#;
        let post = format!(r#"Post {{id: 100, author: {}, title: "Hello World"}}"#, alice);
        let comment = format!(r#"Comment {{post_id: 100, author: {}, text: "Great post!"}}"#, alice);
        
        let results1 = interpreter.query(&format!("author_posted({}, {})", alice, post))
            .expect("Failed to execute author_posted query");
        assert!(!results1.is_empty(), "Complex struct pattern matching should work");
        
        let results2 = interpreter.query(&format!("same_author({}, {})", post, comment))
            .expect("Failed to execute same_author query");
        assert!(!results2.is_empty(), "Complex struct unification should work");
    }

    #[test]
    fn test_struct_with_variables() {
        let program = r#"
            struct Pair(i32, i32);
            struct Named { first: i32, second: i32 }
            
            rel extract_tuple(pair, x, y) {
                Pair(x, y) == pair;
            }
            
            rel extract_named(named, x, y) {
                Named {first: x, second: y} == named;
            }
            
            rel convert_pair_to_named(pair, named) {
                |x, y| {
                    Pair(x, y) == pair,
                    Named {first: x, second: y} == named
                }
            }
        "#;

        let parsed_program = parser::parse_str(program).expect("Failed to parse program");
        let mut interpreter = TestInterpreter::new();
        interpreter.load_program(parsed_program).expect("Failed to load program");
        
        // Test variable extraction from tuple struct
        let results1 = interpreter.query("extract_tuple(Pair(10, 20), X, Y)")
            .expect("Failed to execute tuple extraction");
        assert!(!results1.is_empty(), "Variable extraction from tuple should work");
        
        let x_val = results1[0].bindings.get("X").expect("X should be bound");
        let y_val = results1[0].bindings.get("Y").expect("Y should be bound");
        assert!(format!("{:?}", x_val).contains("10"), "X should be 10");
        assert!(format!("{:?}", y_val).contains("20"), "Y should be 20");
        
        // Test variable extraction from named struct
        let results2 = interpreter.query("extract_named(Named {first: 30, second: 40}, A, B)")
            .expect("Failed to execute named extraction");
        assert!(!results2.is_empty(), "Variable extraction from named should work");
        
        // Test conversion between struct types with variables
        let results3 = interpreter.query("convert_pair_to_named(Pair(5, 15), Named)")
            .expect("Failed to execute conversion");
        assert!(!results3.is_empty(), "Struct type conversion should work");
        
        let named_val = results3[0].bindings.get("Named").expect("Named should be bound");
        let named_str = format!("{:?}", named_val);
        assert!(named_str.contains("5") && named_str.contains("15"), 
                "Named struct should contain converted values");
    }
}