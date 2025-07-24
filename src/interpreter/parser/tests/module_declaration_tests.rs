//! Module declaration parsing tests
//!
//! Tests for parsing module declarations (mod statements).

use super::super::*;
use crate::interpreter::parser::ast::{Item, ModuleDeclaration};
use super::super::builder::AstBuilder;
use crate::interpreter::symbol_table::SymbolTable;
use std::path::PathBuf;

#[test]
fn test_simple_mod_declaration() {
    let input = "mod my_module;";
    let result = VulcanParser::parse(Rule::mod_declaration, input);
    assert!(result.is_ok());

    let pair = result.unwrap().next().unwrap();
    let mut symbol_table = SymbolTable::new();
    let path = PathBuf::from("test.pv");
    let mut builder = AstBuilder::new(&mut symbol_table, &path);
    let mod_decl = builder.build_mod_declaration(pair).unwrap();

    assert_eq!(mod_decl.name.as_ref(), "my_module");
    assert!(mod_decl.visibility == ast::Visibility::Private);
}

#[test]
fn test_pub_mod_declaration() {
    let input = "pub mod utils;";
    let result = VulcanParser::parse(Rule::mod_declaration, input);
    assert!(result.is_ok());

    let pair = result.unwrap().next().unwrap();
    let mut symbol_table = SymbolTable::new();
    let path = PathBuf::from("test.pv");
    let mut builder = AstBuilder::new(&mut symbol_table, &path);
    let mod_decl = builder.build_mod_declaration(pair).unwrap();

    assert_eq!(mod_decl.name.as_ref(), "utils");
    assert!(mod_decl.visibility == ast::Visibility::Public);
}

#[test]
fn test_mod_declaration_in_program() {
    let input = r#"
        mod utils;
        pub mod solver;
        
        rel test() {
            succeed()
        }
    "#;

    let result = VulcanParser::parse(Rule::program, input);
    assert!(result.is_ok());

    let pair = result.unwrap().next().unwrap();
    let mut symbol_table = SymbolTable::new();
    let path = PathBuf::from("test.pv");
    let mut builder = AstBuilder::new(&mut symbol_table, &path);
    let program = builder.build_program(pair).unwrap();

    assert_eq!(program.items.len(), 3);

    // Check first module declaration
    if let Item::ModuleDeclaration(mod_decl) = &program.items[0] {
        assert_eq!(mod_decl.name.as_ref(), "utils");
        assert!(mod_decl.visibility == ast::Visibility::Private);
    } else {
        panic!("Expected ModuleDeclaration");
    }

    // Check second module declaration
    if let Item::ModuleDeclaration(mod_decl) = &program.items[1] {
        assert_eq!(mod_decl.name.as_ref(), "solver");
        assert!(mod_decl.visibility == ast::Visibility::Public);
    } else {
        panic!("Expected ModuleDeclaration");
    }

    // Check predicate
    assert!(matches!(program.items[2], Item::Predicate(_)));
}

#[test]
fn test_mod_declaration_in_module() {
    let input = r#"
        mod parent {
            mod child;
            pub mod utils;
            
            rel helper() {
                succeed()
            }
        }
    "#;

    let result = VulcanParser::parse(Rule::program, input);
    assert!(result.is_ok());

    let pair = result.unwrap().next().unwrap();
    let mut symbol_table = SymbolTable::new();
    let path = PathBuf::from("test.pv");
    let mut builder = AstBuilder::new(&mut symbol_table, &path);
    let program = builder.build_program(pair).unwrap();

    assert_eq!(program.items.len(), 1);

    if let Item::Module(module) = &program.items[0] {
        assert_eq!(module.name.as_ref(), "parent");
        assert_eq!(module.items.len(), 3);

        // Check child module declaration
        if let Item::ModuleDeclaration(mod_decl) = &module.items[0] {
            assert_eq!(mod_decl.name.as_ref(), "child");
            assert!(mod_decl.visibility == ast::Visibility::Private);
        } else {
            panic!("Expected ModuleDeclaration");
        }

        // Check utils module declaration
        if let Item::ModuleDeclaration(mod_decl) = &module.items[1] {
            assert_eq!(mod_decl.name.as_ref(), "utils");
            assert!(mod_decl.visibility == ast::Visibility::Public);
        } else {
            panic!("Expected ModuleDeclaration");
        }

        // Check predicate
        assert!(matches!(module.items[2], Item::Predicate(_)));
    } else {
        panic!("Expected Module");
    }
}

#[test]
fn test_display_mod_declaration() {
    let mod_decl = ModuleDeclaration {
        visibility: ast::Visibility::Private,
        name: "test_module".to_string().into(),
        span: Location::dummy(),
    };
    assert_eq!(format!("{}", mod_decl), "mod test_module;");

    let pub_mod_decl = ModuleDeclaration {
        visibility: ast::Visibility::Public,
        name: "public_module".to_string().into(),
        span: Location::dummy(),
    };
    assert_eq!(format!("{}", pub_mod_decl), "pub mod public_module;");
}