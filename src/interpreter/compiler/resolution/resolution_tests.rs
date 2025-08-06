use super::*;
use crate::interpreter::ast;
use crate::interpreter::parser;

#[test]
fn test_glob_import_resolution() {
    // Create a simple test case for glob import resolution
    let source = r#"
            use tests::common::*;
            
            rel test_rel() {
                positive(5)
            }
        "#;

    // Parse the source
    let program = parser::parse_str(source).expect("Failed to parse test program");

    // Create compiler and compile the program (this will properly initialize everything)
    let mut compiler = Compiler::new();
    let result = compiler.compile_from_ast(program);

    match result {
        Ok(_) => println!("Import resolution succeeded"),
        Err(e) => {
            println!("Import resolution failed: {:?}", e);
            // Print remaining unresolved imports
            println!(
                "Remaining unresolved imports: {}",
                compiler.pending_imports.len()
            );

            for import in &compiler.pending_imports {
                println!(
                    "  importing_module: {}",
                    import.importing_module.full_path()
                );
            }
        }
    }
}

#[test]
fn test_external_path_resolution() {
    // Test external path resolution specifically
    let source = r#"
            use std::list::*;
            
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
