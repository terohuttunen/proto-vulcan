//! Visibility checking system for context-aware symbol access control

use super::super::environment::ModuleInfo;
use super::super::parser::ast::StructDefinition;
use super::super::parser::ast::{QualifiedPath, Visibility};
use super::super::runtime_value::RuntimeValue;
use super::types::*;
use crate::engine::Engine;
use crate::user::User;
use std::collections::HashMap;

/// Core visibility checker that determines symbol accessibility based on context
pub struct VisibilityChecker {
    /// Tracks module hierarchy for ancestor/descendant relationships
    module_hierarchy: ModuleHierarchy,
}

impl VisibilityChecker {
    pub fn new() -> Self {
        Self {
            module_hierarchy: ModuleHierarchy::new(),
        }
    }

    /// Register a module in the hierarchy for visibility checking
    pub fn register_module(&mut self, module_path: ModulePath, parent: Option<ModulePath>) {
        self.module_hierarchy.register_module(module_path, parent);
    }

    /// Check if a symbol with given visibility is accessible from the import context
    pub fn is_symbol_accessible(
        &self,
        symbol_visibility: &Visibility,
        symbol_module: &ModulePath,
        context: &ImportContext,
    ) -> VisibilityResult {
        match symbol_visibility {
            Visibility::Private => {
                // Private symbols are only accessible within the same module
                if context.is_same_module() {
                    VisibilityResult::Accessible
                } else {
                    VisibilityResult::NotAccessible(VisibilityReason::Private)
                }
            }
            Visibility::Public => {
                // Public symbols are always accessible
                VisibilityResult::Accessible
            }
            Visibility::Crate => {
                // Crate-visible symbols are accessible within the same crate
                if context.is_same_crate() {
                    VisibilityResult::Accessible
                } else {
                    VisibilityResult::NotAccessible(VisibilityReason::WrongCrate)
                }
            }
            Visibility::Super => {
                // Super-visible symbols are accessible to parent modules
                if context.is_importing_parent_of_target() || context.is_same_module() {
                    VisibilityResult::Accessible
                } else {
                    VisibilityResult::NotAccessible(VisibilityReason::WrongModule)
                }
            }
            Visibility::SelfModule => {
                // Self-module symbols are only accessible within the same module (like Private)
                if context.is_same_module() {
                    VisibilityResult::Accessible
                } else {
                    VisibilityResult::NotAccessible(VisibilityReason::Private)
                }
            }
            Visibility::Restricted(path) => {
                // Restricted visibility checks if importing module matches the restricted path
                if context.matches_restricted_path(path) {
                    VisibilityResult::Accessible
                } else {
                    VisibilityResult::NotAccessible(VisibilityReason::RestrictedPath)
                }
            }
        }
    }

    /// Get all accessible symbols from a module based on import context
    pub fn get_accessible_symbols<U: User, E: Engine<U>>(
        &self,
        module_info: &ModuleInfo<U, E>,
        context: &ImportContext,
    ) -> AccessibleSymbols<U, E> {
        let mut accessible = AccessibleSymbols::new();

        // Check public symbols - these follow standard visibility rules
        for (name, value) in &module_info.public_symbols {
            if let VisibilityResult::Accessible =
                self.is_symbol_accessible(&Visibility::Public, &context.target_module, context)
            {
                accessible.values.insert(name.clone(), value.clone());
            }
        }

        // Check private symbols - only accessible if we're in the same module
        if context.is_same_module() {
            for (name, value) in &module_info.private_symbols {
                accessible.values.insert(name.clone(), value.clone());
            }
        }

        // Check public types
        for (name, type_def) in &module_info.public_types {
            if let VisibilityResult::Accessible =
                self.is_symbol_accessible(&type_def.visibility, &context.target_module, context)
            {
                accessible.types.insert(name.clone(), type_def.clone());
            }
        }

        // Check private types - only accessible if we're in the same module
        if context.is_same_module() {
            for (name, type_def) in &module_info.private_types {
                accessible.types.insert(name.clone(), type_def.clone());
            }
        }

        accessible
    }

    /// Check accessibility of a specific symbol by name
    pub fn check_symbol_accessibility<U: User, E: Engine<U>>(
        &self,
        symbol_name: &str,
        module_info: &ModuleInfo<U, E>,
        context: &ImportContext,
    ) -> Option<SymbolAccessibility<U, E>> {
        // Check in public symbols first
        if let Some(value) = module_info.public_symbols.get(symbol_name) {
            if let VisibilityResult::Accessible =
                self.is_symbol_accessible(&Visibility::Public, &context.target_module, context)
            {
                return Some(SymbolAccessibility::Value(value.clone()));
            }
        }

        // Check in private symbols if same module
        if context.is_same_module() {
            if let Some(value) = module_info.private_symbols.get(symbol_name) {
                return Some(SymbolAccessibility::Value(value.clone()));
            }
        }

        // Check in public types
        if let Some(type_def) = module_info.public_types.get(symbol_name) {
            if let VisibilityResult::Accessible =
                self.is_symbol_accessible(&type_def.visibility, &context.target_module, context)
            {
                return Some(SymbolAccessibility::Type(type_def.clone()));
            }
        }

        // Check in private types if same module
        if context.is_same_module() {
            if let Some(type_def) = module_info.private_types.get(symbol_name) {
                return Some(SymbolAccessibility::Type(type_def.clone()));
            }
        }

        None
    }

    /// Validate a qualified path for visibility restrictions
    pub fn validate_qualified_path(
        &self,
        path: &QualifiedPath,
        context: &ImportContext,
    ) -> Result<ModulePath, ImportError> {
        match path {
            QualifiedPath::Relative(segments) => {
                // Relative paths are resolved relative to the importing module
                let mut resolved_segments = context.importing_module.segments.clone();
                resolved_segments.extend(segments.clone());
                Ok(ModulePath::new(resolved_segments))
            }
            QualifiedPath::Absolute(segments) => {
                // Absolute paths start from crate root
                Ok(ModulePath::new(segments.clone()))
            }
            QualifiedPath::Global(segments) => {
                // Global paths start from global namespace
                Ok(ModulePath::new(segments.clone()))
            }
            QualifiedPath::Super(levels, segments) => {
                // Super paths go up the module hierarchy
                let mut current_module = context.importing_module.clone();
                for _ in 0..*levels {
                    current_module =
                        current_module
                            .parent()
                            .ok_or_else(|| ImportError::InvalidGlobTarget {
                                target_path: path.clone(),
                                reason: "Cannot go beyond crate root with super::".to_string(),
                            })?;
                }
                current_module.segments.extend(segments.clone());
                Ok(current_module)
            }
            QualifiedPath::Self_(segments) => {
                // Self paths start from current module
                let mut resolved_segments = context.importing_module.segments.clone();
                resolved_segments.extend(segments.clone());
                Ok(ModulePath::new(resolved_segments))
            }

            QualifiedPath::External(crate_name, segments) => {
                // External crate paths (not yet supported)
                Err(ImportError::InvalidGlobTarget {
                    target_path: path.clone(),
                    reason: format!("External crate '{}' not supported", crate_name),
                })
            }
        }
    }
}

/// Type of accessible symbol found
#[derive(Debug, Clone)]
pub enum SymbolAccessibility<U: User, E: Engine<U>> {
    Value(RuntimeValue<U, E>),
    Type(StructDefinition),
}

/// Module hierarchy tracker for visibility checking
#[derive(Debug, Clone)]
struct ModuleHierarchy {
    /// Map from module to its parent module
    parent_map: HashMap<ModulePath, ModulePath>,
    /// Map from module to its children modules
    children_map: HashMap<ModulePath, Vec<ModulePath>>,
}

impl ModuleHierarchy {
    fn new() -> Self {
        Self {
            parent_map: HashMap::new(),
            children_map: HashMap::new(),
        }
    }

    fn register_module(&mut self, module: ModulePath, parent: Option<ModulePath>) {
        if let Some(parent_module) = parent {
            self.parent_map
                .insert(module.clone(), parent_module.clone());
            self.children_map
                .entry(parent_module)
                .or_insert_with(Vec::new)
                .push(module);
        }
    }

    fn get_parent(&self, module: &ModulePath) -> Option<&ModulePath> {
        self.parent_map.get(module)
    }

    fn get_children(&self, module: &ModulePath) -> Option<&Vec<ModulePath>> {
        self.children_map.get(module)
    }

    fn is_ancestor(&self, potential_ancestor: &ModulePath, module: &ModulePath) -> bool {
        let mut current = module;
        while let Some(parent) = self.get_parent(current) {
            if parent == potential_ancestor {
                return true;
            }
            current = parent;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_visibility() {
        let checker = VisibilityChecker::new();
        let context = ImportContext::new(
            ModulePath::from_string("app::ui"),
            ModulePath::from_string("app::utils"),
        );

        let result = checker.is_symbol_accessible(
            &Visibility::Public,
            &ModulePath::from_string("app::utils"),
            &context,
        );

        assert_eq!(result, VisibilityResult::Accessible);
    }

    #[test]
    fn test_private_visibility() {
        let checker = VisibilityChecker::new();
        let context = ImportContext::new(
            ModulePath::from_string("app::ui"),
            ModulePath::from_string("app::utils"),
        );

        let result = checker.is_symbol_accessible(
            &Visibility::Private,
            &ModulePath::from_string("app::utils"),
            &context,
        );

        assert_eq!(
            result,
            VisibilityResult::NotAccessible(VisibilityReason::Private)
        );
    }

    #[test]
    fn test_super_visibility() {
        let checker = VisibilityChecker::new();
        let context = ImportContext::new(
            ModulePath::from_string("app"),
            ModulePath::from_string("app::utils"),
        );

        let result = checker.is_symbol_accessible(
            &Visibility::Super,
            &ModulePath::from_string("app::utils"),
            &context,
        );
        assert_eq!(result, VisibilityResult::Accessible);
    }

    #[test]
    fn test_module_path_operations() {
        let path1 = ModulePath::from_string("app::ui::components");
        let path2 = ModulePath::from_string("app::ui");

        assert!(path2.is_ancestor_of(&path1));
        assert!(!path1.is_ancestor_of(&path2));

        assert_eq!(path1.parent().unwrap(), path2);
        assert_eq!(path1.to_string(), "app::ui::components");
    }

    #[test]
    fn test_import_context_relationships() {
        let context = ImportContext::new(
            ModulePath::from_string("app::ui"),
            ModulePath::from_string("app"),
        );

        assert!(context.is_parent_module());
        assert!(!context.is_same_module());
        assert!(context.is_same_crate());
    }
}
