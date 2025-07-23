//! Symbol and type resolution during IR compilation
//! 
//! This module contains utilities for resolving paths in the new ItemId-based IR.
//! The new IR uses path-based stable identifiers, so most resolution happens
//! during the two-phase compilation process in the compiler module.

use super::*;
use crate::interpreter::parser::ast;
use crate::interpreter::symbol_table::InternedSymbol;
use std::collections::HashMap;

/// Helper for resolving AST qualified paths to ItemId paths
pub struct PathResolver {
    /// Current module context stack (as path strings)
    module_stack: Vec<String>,
}

impl PathResolver {
    pub fn new() -> Self {
        Self {
            module_stack: vec!["::".to_string()], // Start with global module
        }
    }

    /// Resolve an AST qualified path to an absolute path string
    pub fn resolve_path(&self, ast_path: &ast::QualifiedPath) -> Result<String, super::compiler::CompileError> {
        match ast_path {
            ast::QualifiedPath::Global(segments) => {
                // Global paths start from the root
                let path = format!("::{}", segments.iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join("::"));
                Ok(path)
            }
            ast::QualifiedPath::Absolute(segments) => {
                // Absolute paths start from the crate root
                let path = format!("::{}", segments.iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join("::"));
                Ok(path)
            }
            ast::QualifiedPath::Relative(segments) => {
                // Relative paths are relative to current module
                let current_module = self.module_stack.last().unwrap();
                let relative_path = segments.iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                
                if current_module == "::" {
                    Ok(format!("::{}", relative_path))
                } else {
                    Ok(format!("{}::{}", current_module, relative_path))
                }
            }
            ast::QualifiedPath::Super(levels, segments) => {
                // Go up the module hierarchy
                let mut module_path = self.module_stack.clone();
                for _ in 0..*levels {
                    if module_path.len() > 1 {
                        module_path.pop();
                    }
                }
                
                let base_path = module_path.last().unwrap();
                let relative_path = segments.iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                
                if base_path == "::" {
                    Ok(format!("::{}", relative_path))
                } else {
                    Ok(format!("{}::{}", base_path, relative_path))
                }
            }
            ast::QualifiedPath::Self_(segments) => {
                // Self refers to current module
                let current_module = self.module_stack.last().unwrap();
                let relative_path = segments.iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                
                if current_module == "::" {
                    Ok(format!("::{}", relative_path))
                } else {
                    Ok(format!("{}::{}", current_module, relative_path))
                }
            }
            ast::QualifiedPath::External(crate_name, segments) => {
                // External crate reference
                let path = format!("{}::{}", 
                    crate_name.to_string(),
                    segments.iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join("::"));
                Ok(path)
            }
        }
    }

    /// Push a module onto the context stack
    pub fn push_module(&mut self, module_path: String) {
        self.module_stack.push(module_path);
    }

    /// Pop a module from the context stack
    pub fn pop_module(&mut self) -> Option<String> {
        if self.module_stack.len() > 1 {
            self.module_stack.pop()
        } else {
            None
        }
    }

    /// Get the current module path
    pub fn current_module(&self) -> &str {
        self.module_stack.last().unwrap()
    }
}

impl Default for PathResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_path_resolution() {
        let resolver = PathResolver::new();
        let global_path = ast::QualifiedPath::Global(vec![
            InternedSymbol::from_text("std"),
            InternedSymbol::from_text("collections"),
        ]);
        
        let resolved = resolver.resolve_path(&global_path).unwrap();
        assert_eq!(resolved, "::std::collections");
    }

    #[test]
    fn test_relative_path_resolution() {
        let mut resolver = PathResolver::new();
        resolver.push_module("::my_module".to_string());
        
        let relative_path = ast::QualifiedPath::Relative(vec![
            InternedSymbol::from_text("local"),
            InternedSymbol::from_text("item"),
        ]);
        
        let resolved = resolver.resolve_path(&relative_path).unwrap();
        assert_eq!(resolved, "::my_module::local::item");
    }

    #[test]
    fn test_super_path_resolution() {
        let mut resolver = PathResolver::new();
        resolver.push_module("::parent".to_string());
        resolver.push_module("::parent::child".to_string());
        
        let super_path = ast::QualifiedPath::Super(1, vec![
            InternedSymbol::from_text("sibling"),
        ]);
        
        let resolved = resolver.resolve_path(&super_path).unwrap();
        assert_eq!(resolved, "::parent::sibling");
    }

    #[test]
    fn test_external_path_resolution() {
        let resolver = PathResolver::new();
        let external_path = ast::QualifiedPath::External(
            InternedSymbol::from_text("std"),
            vec![InternedSymbol::from_text("collections"), InternedSymbol::from_text("HashMap")]
        );
        
        let resolved = resolver.resolve_path(&external_path).unwrap();
        assert_eq!(resolved, "std::collections::HashMap");
    }
}