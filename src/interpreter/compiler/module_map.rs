//! ModuleMap for efficient module-to-item mapping during compilation
//!
//! This module provides a specialized data structure for maintaining mappings
//! from each module to its contained items. This is used during compilation
//! to provide efficient lookups and updates as the program is built.

use super::ir::{ItemId, ItemName, ModuleId, Program};
use std::collections::HashMap;

/// Maps each module to its contained items for efficient compilation lookups
///
/// The ModuleMap provides O(1) average case lookup for items within modules,
/// which is essential during compilation phases like symbol resolution and
/// import processing. It works alongside the ItemRegistry to provide
/// module-centric access patterns.
#[derive(Debug, Clone)]
pub struct ModuleMap {
    /// Maps module ID to a map of item names to item IDs
    module_items: HashMap<ModuleId, HashMap<ItemName, ItemId>>,
}

impl ModuleMap {
    /// Create an empty ModuleMap
    pub fn new() -> Self {
        Self {
            module_items: HashMap::new(),
        }
    }

    /// Create a new ModuleMap from a Program by iterating through all items
    ///
    /// This constructor builds the module map by examining every item in the
    /// program's registry and organizing them by their containing module.
    pub fn from_program(program: &Program) -> Self {
        let mut module_map = Self::new();
        let root_module = program.get_root_module();
        let root_module_id = root_module.id.clone();

        // Iterate through all items in the registry
        for item in program.registry().all_items() {
            let item_id = item.id();

            // Extract the module path and item name from the item ID
            let (module_path_opt, item_name) = item_id.split();

            if let Some(module_path) = module_path_opt {
                // Convert module path to module ID
                let module_id = module_path.to_module_id();

                // Add the item to the appropriate module
                module_map.add_item(module_id, item_name, item_id.clone());
            } else {
                // Item is at root level - use root module from program
                module_map.add_item(root_module_id.clone(), item_name, item_id.clone());
            }
        }

        module_map
    }

    /// Add an item to a module's item list
    ///
    /// If the module doesn't exist in the map, it will be created automatically.
    /// This method is used during compilation when new items are added to the program.
    pub fn add_item(&mut self, module_id: ModuleId, item_name: ItemName, item_id: ItemId) {
        self.module_items
            .entry(module_id)
            .or_insert_with(HashMap::new)
            .insert(item_name, item_id);
    }

    /// Remove an item from a module's item list
    ///
    /// Returns the removed ItemId if the item existed, None otherwise.
    /// The module entry will remain even if it becomes empty.
    pub fn remove_item(&mut self, module_id: &ModuleId, item_name: &ItemName) -> Option<ItemId> {
        self.module_items
            .get_mut(module_id)
            .and_then(|items| items.remove(item_name))
    }

    /// Add a new module (creates empty item map)
    ///
    /// This ensures the module exists in the map even if it has no items yet.
    /// Useful when creating new modules during compilation.
    pub fn add_module(&mut self, module_id: ModuleId) {
        self.module_items
            .entry(module_id)
            .or_insert_with(HashMap::new);
    }

    /// Remove a module and all its items
    ///
    /// Returns the map of items that were in the module if it existed,
    /// None if the module was not in the map.
    pub fn remove_module(&mut self, module_id: &ModuleId) -> Option<HashMap<ItemName, ItemId>> {
        self.module_items.remove(module_id)
    }

    /// Get all items in a module
    ///
    /// Returns a reference to the HashMap of item names to item IDs for
    /// the specified module, or None if the module doesn't exist.
    pub fn get_module_items(&self, module_id: &ModuleId) -> Option<&HashMap<ItemName, ItemId>> {
        self.module_items.get(module_id)
    }

    /// Get a specific item from a module
    ///
    /// Returns a reference to the ItemId if the item exists in the module,
    /// None if either the module or the item doesn't exist.
    pub fn get_item(&self, module_id: &ModuleId, item_name: &ItemName) -> Option<&ItemId> {
        self.module_items
            .get(module_id)
            .and_then(|items| items.get(item_name))
    }

    /// Check if a module contains a specific item
    ///
    /// Returns true if the module exists and contains the specified item,
    /// false otherwise.
    pub fn contains_item(&self, module_id: &ModuleId, item_name: &ItemName) -> bool {
        self.module_items
            .get(module_id)
            .map(|items| items.contains_key(item_name))
            .unwrap_or(false)
    }

    /// Get all module IDs
    ///
    /// Returns an iterator over all module IDs that have entries in the map.
    /// This includes modules that may have empty item lists.
    pub fn modules(&self) -> impl Iterator<Item = &ModuleId> {
        self.module_items.keys()
    }

    /// Get the number of modules in the map
    pub fn module_count(&self) -> usize {
        self.module_items.len()
    }

    /// Get the total number of items across all modules
    pub fn total_item_count(&self) -> usize {
        self.module_items.values().map(|items| items.len()).sum()
    }

    /// Check if the map is empty (no modules)
    pub fn is_empty(&self) -> bool {
        self.module_items.is_empty()
    }

    /// Get all items of a specific kind across all modules
    ///
    /// This is useful for finding all types, predicates, or modules
    /// during compilation phases that need to process items by category.
    pub fn items_of_kind(&self, kind: super::ir::ItemKind) -> Vec<&ItemId> {
        self.module_items
            .values()
            .flat_map(|items| items.iter())
            .filter(|(item_name, _)| item_name.kind == kind)
            .map(|(_, item_id)| item_id)
            .collect()
    }

    /// Get all items in a specific module of a specific kind
    ///
    /// Returns item IDs for items of the specified kind within the given module.
    pub fn module_items_of_kind(
        &self,
        module_id: &ModuleId,
        kind: super::ir::ItemKind,
    ) -> Vec<&ItemId> {
        self.module_items
            .get(module_id)
            .map(|items| {
                items
                    .iter()
                    .filter(|(item_name, _)| item_name.kind == kind)
                    .map(|(_, item_id)| item_id)
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for ModuleMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::compiler::ir::*;
    use crate::interpreter::symbol_table::InternedSymbol;

    fn create_test_program() -> Program {
        let mut program = Program::new();

        // Add a test module
        let test_module = Module {
            id: ModuleId::with_parent(program.get_root_module_path(), "test"),
            visibility: Visibility::Public,
        };
        program.registry_mut().add_module(test_module);

        // Add a type to the test module
        let test_type = TypeDefinition {
            id: TypeId::with_parent(
                ModuleId::with_parent(program.get_root_module_path(), "test").full_path(),
                "TestType",
            ),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("TestType"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        };
        program.registry_mut().add_type(test_type);

        // Add a predicate to the test module
        let test_predicate = Predicate {
            id: PredicateId::with_parent(
                ModuleId::with_parent(program.get_root_module_path(), "test").full_path(),
                "test_pred",
            ),
            parameters: vec![],
            body: StructuralGoal::empty_container(),
            kind: PredicateKind::Relation,
            visibility: Visibility::Public,
        };
        program.registry_mut().add_predicate(test_predicate);

        program
    }

    #[test]
    fn test_new_module_map() {
        let module_map = ModuleMap::new();
        assert!(module_map.is_empty());
        assert_eq!(module_map.module_count(), 0);
        assert_eq!(module_map.total_item_count(), 0);
    }

    #[test]
    fn test_from_program() {
        let program = create_test_program();
        let module_map = ModuleMap::from_program(&program);

        // Should have root module and test module
        assert!(!module_map.is_empty());
        assert_eq!(module_map.module_count(), 2);

        // Check that test module exists and has items
        let test_module_id = ModuleId::with_parent(program.get_root_module_path(), "test");
        let test_items = module_map.get_module_items(&test_module_id);
        assert!(test_items.is_some());

        let items = test_items.unwrap();
        assert_eq!(items.len(), 2); // TestType and test_pred
    }

    #[test]
    fn test_add_item() {
        let mut module_map = ModuleMap::new();
        let program = Program::new();
        let root_module_id = program.get_root_module().id.clone();
        let item_name = ItemName::new("test_item", ItemKind::Type).unwrap();
        let item_id = ItemId::global("test_item", ItemKind::Type);

        module_map.add_item(root_module_id.clone(), item_name.clone(), item_id.clone());

        assert!(!module_map.is_empty());
        assert_eq!(module_map.module_count(), 1);
        assert_eq!(module_map.total_item_count(), 1);
        assert!(module_map.contains_item(&root_module_id, &item_name));
    }

    #[test]
    fn test_remove_item() {
        let mut module_map = ModuleMap::new();
        let program = Program::new();
        let root_module_id = program.get_root_module().id.clone();
        let item_name = ItemName::new("test_item", ItemKind::Type).unwrap();
        let item_id = ItemId::global("test_item", ItemKind::Type);

        module_map.add_item(root_module_id.clone(), item_name.clone(), item_id.clone());

        let removed = module_map.remove_item(&root_module_id, &item_name);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap(), item_id);
        assert!(!module_map.contains_item(&root_module_id, &item_name));
    }

    #[test]
    fn test_add_remove_module() {
        let mut module_map = ModuleMap::new();
        let program = Program::new();
        let root_module_id = program.get_root_module().id.clone();

        module_map.add_module(root_module_id.clone());
        assert_eq!(module_map.module_count(), 1);

        let removed = module_map.remove_module(&root_module_id);
        assert!(removed.is_some());
        assert_eq!(module_map.module_count(), 0);
    }

    #[test]
    fn test_get_item() {
        let mut module_map = ModuleMap::new();
        let program = Program::new();
        let root_module_id = program.get_root_module().id.clone();
        let item_name = ItemName::new("test_item", ItemKind::Type).unwrap();
        let item_id = ItemId::global("test_item", ItemKind::Type);

        module_map.add_item(root_module_id.clone(), item_name.clone(), item_id.clone());

        let retrieved = module_map.get_item(&root_module_id, &item_name);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), &item_id);
    }

    #[test]
    fn test_items_of_kind() {
        let program = create_test_program();
        let module_map = ModuleMap::from_program(&program);

        let types = module_map.items_of_kind(ItemKind::Type);
        let predicates = module_map.items_of_kind(ItemKind::Predicate);
        let modules = module_map.items_of_kind(ItemKind::Module);

        assert_eq!(types.len(), 1); // TestType
        assert_eq!(predicates.len(), 1); // test_pred
        assert_eq!(modules.len(), 2); // root module and test module
    }

    #[test]
    fn test_module_items_of_kind() {
        let program = create_test_program();
        let module_map = ModuleMap::from_program(&program);
        let test_module_id = ModuleId::with_parent(program.get_root_module_path(), "test");

        let types = module_map.module_items_of_kind(&test_module_id, ItemKind::Type);
        let predicates = module_map.module_items_of_kind(&test_module_id, ItemKind::Predicate);

        assert_eq!(types.len(), 1);
        assert_eq!(predicates.len(), 1);
    }
}
