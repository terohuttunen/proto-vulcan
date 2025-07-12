use super::parser::ast::{
    Item, Program, RelationDefinition, StructDefinition, UsePath, UseStatement,
};
use super::runtime_value::RuntimeValue;
use super::InterpreterError;
use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::User;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Environment manages symbol tables, scopes, and program state
pub struct Environment<U: User, E: Engine<U>> {
    /// Global symbol table
    globals: HashMap<String, RuntimeValue<U, E>>,
    /// Module scopes
    modules: HashMap<String, HashMap<String, RuntimeValue<U, E>>>,
    /// Current scope stack
    scope_stack: Vec<String>,
    /// Type definitions
    types: HashMap<String, StructDefinition>,
    /// The base path for resolving modules.
    base_path: PathBuf,
}

impl<U: User, E: Engine<U>> Environment<U, E> {
    /// Create a new environment
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            modules: HashMap::new(),
            scope_stack: vec!["global".to_string()],
            types: HashMap::new(),
            base_path: PathBuf::new(),
        }
    }

    /// Sets the base path for module resolution.
    pub fn set_base_path(&mut self, path: PathBuf) {
        self.base_path = path;
    }

    /// Adds a native Rust function as a relation to the global scope.
    pub fn add_native_relation(
        &mut self,
        name: String,
        func: Rc<dyn Fn(Vec<LTerm<U, E>>) -> Goal<U, E>>,
        arity: usize,
    ) {
        let value = RuntimeValue::NativeRelation { func, arity };
        self.globals.insert(name, value);
    }

    /// Load a program into the environment
    pub fn load_program(&mut self, program: Program) -> Result<(), InterpreterError> {
        for item in program.items {
            self.load_item(item)?;
        }
        Ok(())
    }

    /// Load a single program item
    fn load_item(&mut self, item: Item) -> Result<(), InterpreterError> {
        match item {
            Item::Relation(rel) => self.load_relation(rel),
            Item::Struct(struct_def) => self.load_struct(struct_def),
            Item::Module(module) => self.load_module(module),
            Item::Use(use_stmt) => self.load_use_statement(use_stmt),
            Item::Impl(_) => Ok(()), // TODO: Handle impl blocks
        }
    }

    /// Load a relation definition
    fn load_relation(&mut self, relation: RelationDefinition) -> Result<(), InterpreterError> {
        let name = relation.name.clone();
        let value = RuntimeValue::Relation(relation);

        if self.scope_stack.last() == Some(&"global".to_string()) {
            self.globals.insert(name, value);
        } else {
            let current_scope = self.scope_stack.last().unwrap().clone();
            self.modules
                .entry(current_scope)
                .or_insert_with(HashMap::new)
                .insert(name, value);
        }

        Ok(())
    }

    /// Load a struct definition
    fn load_struct(&mut self, struct_def: StructDefinition) -> Result<(), InterpreterError> {
        let name = struct_def.name.clone();
        self.types.insert(name, struct_def);
        Ok(())
    }

    /// Load a module
    fn load_module(
        &mut self,
        module: super::parser::ast::ModuleDefinition,
    ) -> Result<(), InterpreterError> {
        let module_name = module.name.clone();
        self.scope_stack.push(module_name.clone());

        for item in module.items {
            self.load_item(item)?;
        }

        self.scope_stack.pop();
        Ok(())
    }

    /// Load a use statement
    fn load_use_statement(&mut self, use_stmt: UseStatement) -> Result<(), InterpreterError> {
        match use_stmt.path {
            UsePath::Simple(path_segments) => {
                // Handle simple imports like "use std" or "use std::list"
                if path_segments.len() == 1 && path_segments[0] == "std" {
                    self.load_std_library()?;
                } else if path_segments.len() == 2 && path_segments[0] == "std" {
                    self.load_std_module(&path_segments[1])?;
                } else {
                    // For now, ignore other imports
                    return Ok(());
                }
            }
            UsePath::Glob(path_segments) => {
                // Handle glob imports like "use std::*"
                if path_segments.len() == 1 && path_segments[0] == "std" {
                    self.load_std_library()?;
                } else {
                    return Ok(());
                }
            }
            UsePath::List(_, _) => {
                // Handle list imports like "use std::{member, append}"
                // TODO: Implement selective imports
                return Ok(());
            }
        }
        Ok(())
    }

    /// Load the entire standard library
    pub fn load_std_library(&mut self) -> Result<(), InterpreterError> {
        self.load_std_module("list")?;
        self.load_std_module("logic")?;
        self.load_std_module("util")?;
        Ok(())
    }

    /// Load a specific standard library module
    fn load_std_module(&mut self, module_name: &str) -> Result<(), InterpreterError> {
        let std_path = format!("std/{}.pv", module_name);

        if Path::new(&std_path).exists() {
            let source = fs::read_to_string(&std_path).map_err(|e| {
                InterpreterError::RuntimeError(format!(
                    "Failed to read std module {}: {}",
                    module_name, e
                ))
            })?;

            let program = super::parser::parse_str(&source).map_err(|e| {
                InterpreterError::ParseError(format!("Parse error in std::{}: {}", module_name, e))
            })?;

            // Load the module contents into the global scope
            for item in program.items {
                self.load_item(item)?;
            }
        } else {
            return Err(InterpreterError::RuntimeError(format!(
                "Standard library module '{}' not found",
                module_name
            )));
        }

        Ok(())
    }

    /// Look up a symbol in the current scope
    pub fn lookup(&self, name: &str) -> Option<&RuntimeValue<U, E>> {
        // First check current module scope
        if let Some(current_scope) = self.scope_stack.last() {
            if current_scope != "global" {
                if let Some(module_symbols) = self.modules.get(current_scope) {
                    if let Some(value) = module_symbols.get(name) {
                        return Some(value);
                    }
                }
            }
        }

        // Then check globals
        self.globals.get(name)
    }

    /// Get a struct definition
    pub fn get_struct(&self, name: &str) -> Option<&StructDefinition> {
        self.types.get(name)
    }

    /// Create a fresh variable
    pub fn fresh_var(&self, name: &'static str) -> LTerm<U, E> {
        LTerm::var(name)
    }

    /// Get current scope name
    pub fn current_scope(&self) -> &str {
        self.scope_stack.last().unwrap()
    }

    /// Get all relations in the current environment
    pub fn relations(&self) -> HashMap<String, &RuntimeValue<U, E>> {
        let mut relations = HashMap::new();

        // Add global relations
        for (name, value) in &self.globals {
            if matches!(value, RuntimeValue::Relation(_)) {
                relations.insert(name.clone(), value);
            }
        }

        // Add module relations
        for module_symbols in self.modules.values() {
            for (name, value) in module_symbols {
                if matches!(value, RuntimeValue::Relation(_)) {
                    relations.insert(name.clone(), value);
                }
            }
        }

        relations
    }

    /// Get all structs in the current environment
    pub fn structs(&self) -> &HashMap<String, StructDefinition> {
        &self.types
    }

    /// Get all variables in the current scope (placeholder implementation)
    pub fn variables(&self) -> HashMap<String, &RuntimeValue<U, E>> {
        let mut variables = HashMap::new();

        // Add global variables (non-relations)
        for (name, value) in &self.globals {
            if !matches!(value, RuntimeValue::Relation(_)) {
                variables.insert(name.clone(), value);
            }
        }

        variables
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::DefaultEngine;
    use crate::interpreter::parser::ast::*;
    use crate::user::DefaultUser;

    type TestEnv = Environment<DefaultUser, DefaultEngine<DefaultUser>>;

    #[test]
    fn test_new_environment() {
        let env = TestEnv::new();
        assert_eq!(env.current_scope(), "global");
        assert!(env.lookup("nonexistent").is_none());
    }

    #[test]
    fn test_load_relation() {
        let mut env = TestEnv::new();

        let relation = RelationDefinition {
            is_pub: false,
            attributes: vec![],
            name: "test_rel".to_string(),
            parameters: vec![],
            search_strategy: None,
            body: vec![],
        };

        env.load_relation(relation).unwrap();

        assert!(env.lookup("test_rel").is_some());
        match env.lookup("test_rel").unwrap() {
            RuntimeValue::Relation(rel) => assert_eq!(rel.name, "test_rel"),
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_load_struct() {
        let mut env = TestEnv::new();

        let struct_def = StructDefinition {
            is_pub: false,
            name: "Point".to_string(),
            kind: StructKind::Named(vec![
                NamedField {
                    is_pub: false,
                    name: "x".to_string(),
                    type_name: "i32".to_string(),
                },
                NamedField {
                    is_pub: false,
                    name: "y".to_string(),
                    type_name: "i32".to_string(),
                },
            ]),
        };

        env.load_struct(struct_def).unwrap();

        let struct_def = env.get_struct("Point").unwrap();
        assert_eq!(struct_def.name, "Point");
    }

    #[test]
    fn test_module_scoping() {
        let mut env = TestEnv::new();

        let module = ModuleDefinition {
            name: "test_module".to_string(),
            search_strategy: None,
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                attributes: vec![],
                name: "module_rel".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![],
            })],
        };

        env.load_module(module).unwrap();

        // Should not find module relation in global scope
        assert!(env.lookup("module_rel").is_none());

        // But should find it if we had proper module path resolution
        assert!(env
            .modules
            .get("test_module")
            .unwrap()
            .contains_key("module_rel"));
    }

    #[test]
    fn test_fresh_var() {
        let env = TestEnv::new();
        let var: LTerm<DefaultUser, DefaultEngine<DefaultUser>> = env.fresh_var("x");
        assert!(var.is_var());
    }
}
