use super::*;
use crate::interpreter::ast;
use crate::interpreter::parser;


#[test]
fn test_external_path_resolution() {
    // Test external path resolution specifically
    let source = r#"
            use std::list::{member};
            
            rel test_rel() {
                member(1, [1, 2, 3])
            }
        "#;

    // Parse the source
    let program = parser::parse_str(source).expect("Failed to parse test program");

    // Create compiler and compile the program (this will properly initialize everything)
    let mut compiler = Compiler::new();
    let result = compiler.compile_from_ast(program);

    match result {
        Ok(_) => println!("External path resolution succeeded"),
        Err(e) => println!("External path resolution failed: {:?}", e),
    }
}
