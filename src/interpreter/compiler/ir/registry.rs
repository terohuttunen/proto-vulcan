//! Unified item registry for the IR
//!
//! This registry stores all items (modules, types, predicates) using stable
//! ItemId keys, supporting dynamic addition and removal while maintaining
//! immutability through Rc::make_mut.

use super::*;
use im_rc::HashMap;
use std::rc::Rc;

/// Unified registry for all IR items
/// Uses ItemId for stable, path-based identification with im-rc HashMap for efficient structural sharing
#[derive(Debug, Clone)]
pub struct ItemRegistry {
    /// All items stored by their ItemId with structural sharing via im-rc HashMap
    /// Items are wrapped in Rc for efficient clone-on-write semantics
    items: HashMap<ItemId, Rc<Item>>,

    /// Map to track which modules re-export other modules
    /// Key: ModuleId of the exporting module
    /// Value: Vec of ModuleIds that are re-exported by this module
    re_exports: HashMap<ModuleId, Vec<ModuleId>>,
}

impl ItemRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            re_exports: HashMap::new(),
        }
    }

    /// Insert an item into the registry
    /// Returns the previous item if one existed with the same ID, None otherwise
    pub fn insert_item(&mut self, item: Item) -> Option<Rc<Item>> {
        let id = item.id().clone();
        self.items.insert(id.clone(), Rc::new(item))
    }

    /// Add an item to the registry (convenience method for backward compatibility)
    /// Returns true if the item was added, false if it already existed
    pub fn add_item(&mut self, item: Item) -> bool {
        self.insert_item(item).is_none()
    }

    /// Remove an item from the registry
    /// Returns the removed item if it existed
    pub fn remove_item<T: AsRef<ItemId>>(&mut self, id: T) -> Option<Rc<Item>> {
        let id_ref = id.as_ref();
        let removed = self.items.remove(id_ref);

        /*
        // If item was removed, also remove it from its parent module's items collection
        if removed.is_some() {
            self.remove_item_from_parent_module(id_ref);
        }
        */

        removed
    }

    /// Replace an existing item in the registry
    /// Returns the old item if it existed
    pub fn replace_item(&mut self, item: Item) -> Option<Rc<Item>> {
        let id = item.id().clone();
        self.items.insert(id, Rc::new(item))
    }

    /// Get an item by its ItemId
    pub fn get_item<T: AsRef<ItemId>>(&self, id: T) -> Option<&Rc<Item>> {
        self.items.get(id.as_ref())
    }

    /// Get a cloned Rc<Item> for mutation
    pub fn get_item_for_mutation<T: AsRef<ItemId>>(&self, id: T) -> Option<Rc<Item>> {
        self.items.get(id.as_ref()).cloned()
    }

    /// Check if an item exists
    pub fn contains_item<T: AsRef<ItemId>>(&self, id: T) -> bool {
        self.items.contains_key(id.as_ref())
    }

    //pub fn children(&self, module_id: ModuleId) -> impl Iterator<Item = &Rc<Item>>

    /// Get all items
    pub fn all_items(&self) -> impl Iterator<Item = &Rc<Item>> {
        self.items.values()
    }

    /// Get all items of a specific kind
    pub fn items_of_kind(&self, kind: ItemKind) -> impl Iterator<Item = &Rc<Item>> {
        self.items
            .values()
            .filter(move |item| item.id().kind() == kind)
    }

    /// Get the number of items in the registry
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    // Specialized getters using the Borrow trait for efficient lookups

    /// Get a module by ModuleId or any reference that can be converted to ItemId
    pub fn get_module<T: AsRef<ItemId>>(&self, module_ref: T) -> Option<&Module> {
        match self.get_item(module_ref)?.as_ref() {
            Item::Module(module) => Some(module),
            _ => None,
        }
    }

    /// Get a type by TypeId or any reference that can be converted to ItemId
    pub fn get_type<T: AsRef<ItemId>>(&self, type_ref: T) -> Option<&TypeDefinition> {
        match self.get_item(type_ref)?.as_ref() {
            Item::Type(type_def) => Some(type_def),
            _ => None,
        }
    }

    /// Get a predicate by PredicateId or any reference that can be converted to ItemId
    pub fn get_predicate<T: AsRef<ItemId>>(&self, predicate_ref: T) -> Option<&Predicate> {
        match self.get_item(predicate_ref)?.as_ref() {
            Item::Predicate(predicate) => Some(predicate),
            _ => None,
        }
    }

    /// Get all modules
    pub fn modules(&self) -> impl Iterator<Item = &Module> {
        self.items_of_kind(ItemKind::Module)
            .filter_map(|item| match item.as_ref() {
                Item::Module(module) => Some(module),
                _ => None,
            })
    }

    /// Get all types
    pub fn types(&self) -> impl Iterator<Item = &TypeDefinition> {
        self.items_of_kind(ItemKind::Type)
            .filter_map(|item| match item.as_ref() {
                Item::Type(type_def) => Some(type_def),
                _ => None,
            })
    }

    /// Get all predicates
    pub fn predicates(&self) -> impl Iterator<Item = &Predicate> {
        self.items_of_kind(ItemKind::Predicate)
            .filter_map(|item| match item.as_ref() {
                Item::Predicate(predicate) => Some(predicate),
                _ => None,
            })
    }

    /// Get the arity (number of parameters) of a predicate by PredicateId or any reference that can be converted to ItemId
    pub fn get_predicate_arity<T: AsRef<ItemId>>(&self, predicate_ref: T) -> Option<usize> {
        self.get_predicate(predicate_ref)
            .map(|p| p.parameters.len())
    }

    /*
    /// Find modules that are children of a given module
    /// Uses the Module.items field for O(k) performance instead of O(n) registry scanning
    pub fn child_modules(&self, parent_path: &str) -> Box<dyn Iterator<Item = &Module> + '_> {
        // Parse the parent path and construct ModuleId
        let parent_module_id = if parent_path == "::" {
            ModuleId::root()
        } else {
            // Parse the path by splitting on :: and building nested ModulePath
            let mut parts: Vec<&str> = parent_path.split("::").collect();
            if parts.first() == Some(&"") {
                parts.remove(0); // Remove empty first element from leading ::
            }

            if parts.is_empty() {
                ModuleId::root()
            } else {
                // Build nested path: start from root and extend with each part
                let mut current_path = Rc::new(ModulePath::root());
                for part in &parts[..parts.len() - 1] {
                    current_path = current_path.extend(*part).into();
                }

                // Create ModuleId for the final module name
                let module_name = parts.last().unwrap_or(&"");
                ModuleId::new(current_path, *module_name)
            }
        };
        let parent_id = parent_module_id.to_item_id();

        // Get the parent module and iterate through its direct children
        let iter = self
            .get_item(&parent_id)
            .and_then(|item| match item.as_ref() {
                Item::Module(parent_module) => Some(parent_module),
                _ => None,
            })
            .into_iter()
            .flat_map(move |parent_module| {
                parent_module
                    .items
                    .iter()
                    .filter_map(move |(item_name, item_id)| {
                        // Only return modules (filter out types and predicates)
                        if item_name.kind == ItemKind::Module {
                            match self.get_item(item_id)?.as_ref() {
                                Item::Module(module) => Some(module),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    })
            });

        Box::new(iter)
    }
    */

    /// Add a module to the registry
    /// Add a module to the registry
    pub fn add_module(&mut self, module: Module) -> bool {
        let module_id = module.id.clone();
        let result = self.add_item(Item::Module(module));
        /*
        if result {
            // Add to parent module's items map for ergonomic access
            self.add_module_to_parent_module(module_id);
        }
        */
        result
    }

    /*/
    /// Add an item to a specific module's items map (ergonomic helper)
    /// This maintains the parent-child relationship by adding the item to the parent module's map
    /// Uses efficient Rc::make_mut for copy-on-write semantics
    pub fn add_item_to_module(
        &mut self,
        parent_module_id: &ModuleId,
        item_name: super::ItemName,
        item_id: ItemId,
    ) -> bool {
        // First check if the parent module exists
        if let Some(RegistryEntry::Item(module_rc)) = self.items.get_mut(parent_module_id.as_ref())
        {
            // Get mutable access to the module (CoW only if multiple references exist)
            if let Item::Module(module) = Rc::make_mut(module_rc) {
                module.items.insert(item_name, item_id);
                return true;
            }
        }
        false
    }

    /// Add an item to a module using ModulePath directly
    pub fn add_item_to_module_path(
        &mut self,
        module_path: Rc<ModulePath>,
        item_id: ItemId,
    ) -> bool {
        let module_id = module_path.to_module_id();

        // Add item directly to the module
        if let Some(RegistryEntry::Item(module_rc)) = self.items.get_mut(module_id.as_ref()) {
            if let Item::Module(module) = Rc::make_mut(module_rc) {
                module.items.insert(item_id.name.clone(), item_id);
                return true;
            }
        }
        false
    }

    /// Add an alias to a specific module's items map (ergonomic helper)
    /// This maintains the parent-child relationship by adding the item to the parent module's map
    /// Uses efficient Rc::make_mut for copy-on-write semantics
    pub fn add_alias_to_module(
        &mut self,
        parent_module_id: &ModuleId,
        alias: super::ItemName,
        item_id: ItemId,
    ) -> bool {
        // First check if the parent module exists
        if let Some(RegistryEntry::Item(module_rc)) = self.items.get_mut(parent_module_id.as_ref())
        {
            // Get mutable access to the module (CoW only if multiple references exist)
            if let Item::Module(module) = Rc::make_mut(module_rc) {
                module.imports.insert(alias, item_id);
                return true;
            }
        }
        false
    }

    /// Add an item to a module using ModulePath directly
    pub fn add_import_to_module_path(
        &mut self,
        module_path: Rc<ModulePath>,
        item_id: ItemId,
    ) -> bool {
        let module_id = module_path.to_module_id();

        // Add item directly to the module
        if let Some(RegistryEntry::Item(module_rc)) = self.items.get_mut(module_id.as_ref()) {
            if let Item::Module(module) = Rc::make_mut(module_rc) {
                module.imports.insert(item_id.name.clone(), item_id);
                return true;
            }
        }
        false
    }

    /// Add a module to its parent module using typed ID
    pub fn add_module_to_parent_module(&mut self, module_id: super::ModuleId) -> bool {
        let (parent_module_path, _module_name) = module_id.id.split();
        if let Some(parent_module_path) = parent_module_path {
            self.add_item_to_module_path(parent_module_path, module_id.id)
        } else {
            false
        }
    }
    /// Add a type to its parent module using typed ID
    pub fn add_type_to_parent_module(&mut self, type_id: super::TypeId) -> bool {
        let (parent_module_path, _type_name) = type_id.split();
        if let Some(parent_module_path) = parent_module_path {
            self.add_item_to_module_path(parent_module_path, type_id.id)
        } else {
            false
        }
    }

    /// Add a predicate to its parent module using typed ID
    pub fn add_predicate_to_parent_module(&mut self, predicate_id: super::PredicateId) -> bool {
        let (parent_module_path, _predicate_name) = predicate_id.split();
        if let Some(parent_module_path) = parent_module_path {
            self.add_item_to_module_path(parent_module_path, predicate_id.id)
        } else {
            false
        }
    }

    /// Get all items in a module (ergonomic helper)
    pub fn get_module_items(
        &self,
        module_id: &ModuleId,
    ) -> Option<&im_rc::HashMap<super::ItemName, ItemId>> {
        if let Some(item) = self.items.get(module_id.as_ref()) {
            if let Item::Module(module) = item.as_ref() {
                Some(&module.items)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Check if a module contains a specific item (ergonomic helper)
    pub fn module_contains_item(&self, module_id: &ModuleId, item_name: &super::ItemName) -> bool {
        if let Some(items) = self.get_module_items(module_id) {
            items.contains_key(item_name)
        } else {
            false
        }
    }
    */

    /// Add a type to the registry
    /// Automatically maintains parent-child relationships
    pub fn add_type(&mut self, type_def: TypeDefinition) -> bool {
        let type_id = type_def.id.clone();
        let result = self.add_item(Item::Type(type_def));
        /*
        if result {
            // Add to parent module's items map for ergonomic access
            self.add_type_to_parent_module(type_id);
        }
        */
        result
    }

    /// Add a predicate to the registry
    /// Automatically maintains parent-child relationships  
    pub fn add_predicate(&mut self, predicate: Predicate) -> bool {
        let predicate_id = predicate.id.clone();
        let result = self.add_item(Item::Predicate(predicate));
        /*
        if result {
            // Add to parent module's items map for ergonomic access
            self.add_predicate_to_parent_module(predicate_id);
        }
        */
        result
    }

    pub fn add_alias(&mut self, alias: Alias) -> bool {
        self.add_item(Item::Alias(alias))
    }

    // Immutable update methods leveraging im-rc's persistent data structures

    /// Create a new registry with an item inserted (immutable operation)
    /// Returns the new registry and the previous item if one existed with the same ID
    pub fn with_item_inserted(&self, item: Item) -> (Self, Option<Rc<Item>>) {
        let id = item.id().clone();
        let mut new_items = self.items.clone();
        let previous = new_items.insert(id.clone(), Rc::new(item));

        let mut new_registry = Self {
            items: new_items,
            re_exports: self.re_exports.clone(),
        };

        (new_registry, previous)
    }

    /// Create a new registry with an item added (immutable operation, backward compatibility)
    /// Returns the new registry and whether the item was actually added
    pub fn with_item(&self, item: Item) -> (Self, bool) {
        let (new_registry, previous) = self.with_item_inserted(item);
        (new_registry, previous.is_none())
    }

    /// Create a new registry with an item removed (immutable operation)
    /// Returns the new registry and the removed item if it existed
    pub fn without_item<T: AsRef<ItemId>>(&self, id: T) -> (Self, Option<Item>) {
        let id_ref = id.as_ref();
        let mut new_items = self.items.clone();
        let removed = new_items.remove(id_ref);

        let mut new_registry = Self {
            items: new_items,
            re_exports: self.re_exports.clone(),
        };

        (
            new_registry,
            removed.map(|rc| Rc::try_unwrap(rc).unwrap_or_else(|rc| (*rc).clone())),
        )
    }

    /// Create a new registry with a module added (immutable operation)
    /// Automatically handles parent reference consistency
    pub fn with_module(&self, mut module: Module) -> (Self, bool) {
        self.with_item(Item::Module(module))
    }

    /// Create a new registry with a type added (immutable operation)
    pub fn with_type(&self, type_def: TypeDefinition) -> (Self, bool) {
        self.with_item(Item::Type(type_def))
    }

    /// Create a new registry with a predicate added (immutable operation)
    pub fn with_predicate(&self, predicate: Predicate) -> (Self, bool) {
        self.with_item(Item::Predicate(predicate))
    }

    /// Create a new registry with a re-export added (immutable operation)
    pub fn with_re_export(&self, exporting_module: ModuleId, re_exported_module: ModuleId) -> Self {
        let mut new_re_exports = self.re_exports.clone();
        new_re_exports
            .entry(exporting_module)
            .or_insert_with(Vec::new)
            .push(re_exported_module);
        
        Self {
            items: self.items.clone(),
            re_exports: new_re_exports,
        }
    }

    /// Create a new registry with a re-export removed (immutable operation)
    pub fn without_re_export(&self, exporting_module: &ModuleId, re_exported_module: &ModuleId) -> (Self, bool) {
        let mut new_re_exports = self.re_exports.clone();
        let was_removed = if let Some(re_exported_modules) = new_re_exports.get_mut(exporting_module) {
            if let Some(pos) = re_exported_modules.iter().position(|m| m == re_exported_module) {
                re_exported_modules.remove(pos);
                // Clean up empty entries
                if re_exported_modules.is_empty() {
                    new_re_exports.remove(exporting_module);
                }
                true
            } else {
                false
            }
        } else {
            false
        };
        
        let new_registry = Self {
            items: self.items.clone(),
            re_exports: new_re_exports,
        };
        
        (new_registry, was_removed)
    }

    /*
    /// Remove a module and all its children using recursive hierarchy traversal
    pub fn remove_module_tree(&mut self, module_id: &ModuleId) -> Vec<Item> {
        let mut removed = Vec::new();
        self.remove_item_and_children_recursive(module_id, &mut removed);
        removed
    }

    /// Recursively remove an item and all its children from the hierarchy
    /// Used internally by remove_module_tree to handle both modules and their child items
    /// Uses efficient Rc::make_mut pattern for optimal copy-on-write performance
    fn remove_item_and_children_recursive<T: AsRef<ItemId>>(
        &mut self,
        item_id: T,
        removed: &mut Vec<Item>,
    ) {
        let item_id = item_id.as_ref();

        // If this is a module, recursively remove all its children first
        if let Some(module_rc) = self.items.get_mut(item_id) {
            if let Item::Module(module) = Rc::make_mut(module_rc) {
                // Collect child IDs to avoid borrow checker issues during recursive removal
                let child_ids: Vec<_> = module.items.values().cloned().collect();
                // Recursively remove children
                for child_id in child_ids {
                    self.remove_item_and_children_recursive(&child_id, removed);
                }
            }
        }

        // Remove the item itself using the existing remove_item method
        if let Some(item_rc) = self.remove_item(item_id) {
            // Optimize the unwrapping - try to avoid cloning if possible
            match Rc::try_unwrap(item_rc) {
                Ok(item) => removed.push(item),
                Err(rc) => removed.push((*rc).clone()), // Clone only if multiple references exist
            }
        }
    }

    /// Remove an item from a specific module's items map and from the registry
    /// Uses efficient Rc::make_mut for copy-on-write semantics
    pub fn remove_item_from_module(
        &mut self,
        parent_module_id: &ModuleId,
        item_name: &super::ItemName,
    ) -> Option<Rc<Item>> {
        // Check if the parent module exists and get the item ID first
        let item_id = if let Some(module_rc) = self.items.get_mut(parent_module_id.as_ref()) {
            // Get mutable access to the module (CoW only if multiple references exist)
            if let Item::Module(module) = Rc::make_mut(module_rc) {
                // Use im-rc HashMap's efficient remove method to get the item ID
                module.items.remove(item_name)
            } else {
                None
            }
        } else {
            None
        };

        // If we found and removed the item from the module, also remove it from the registry
        if let Some(item_id) = item_id {
            self.remove_item(&item_id)
        } else {
            None
        }
    }

    /// Remove an item from its parent module's items collection
    /// This is the counterpart to add_item_to_module_path
    fn remove_item_from_parent_module(&mut self, item_id: &ItemId) {
        // Determine the parent module path by extracting the module path from the item path
        let (parent_module_path, item_name) = item_id.split();

        if let Some(parent_module_path) = parent_module_path {
            let parent_module_id = parent_module_path.to_module_id();

            // Remove item from the parent module's items collection
            if let Some(module_rc) = self.items.get_mut(parent_module_id.as_ref()) {
                if let Item::Module(module) = Rc::make_mut(module_rc) {
                    module.items.remove(&item_name);
                }
            }
        }
    }
    */

    // ============================================================================
    // Re-exports Management API
    // ============================================================================

    /// Add a re-export relationship: exporting_module re-exports re_exported_module
    /// 
    /// This is used when a module has `pub use other_module::*` or similar re-export statements.
    /// It records that items from `re_exported_module` are accessible through `exporting_module`.
    pub fn add_re_export(&mut self, exporting_module: ModuleId, re_exported_module: ModuleId) {
        self.re_exports
            .entry(exporting_module)
            .or_insert_with(Vec::new)
            .push(re_exported_module);
    }

    /// Remove a specific re-export relationship
    /// 
    /// Returns true if the re-export was found and removed, false otherwise.
    pub fn remove_re_export(&mut self, exporting_module: &ModuleId, re_exported_module: &ModuleId) -> bool {
        if let Some(re_exported_modules) = self.re_exports.get_mut(exporting_module) {
            if let Some(pos) = re_exported_modules.iter().position(|m| m == re_exported_module) {
                re_exported_modules.remove(pos);
                // Clean up empty entries
                if re_exported_modules.is_empty() {
                    self.re_exports.remove(exporting_module);
                }
                return true;
            }
        }
        false
    }

    /// Clear all re-exports for a given module
    /// 
    /// This removes all `pub use other::*` relationships where the given module is the exporter.
    pub fn clear_re_exports(&mut self, exporting_module: &ModuleId) {
        self.re_exports.remove(exporting_module);
    }

    /// Get all modules re-exported by the given module
    /// 
    /// Returns None if the module has no re-exports.
    pub fn get_re_exports(&self, module_id: &ModuleId) -> Option<&Vec<ModuleId>> {
        self.re_exports.get(module_id)
    }

    /// Check if a specific re-export relationship exists
    pub fn has_re_export(&self, exporting_module: &ModuleId, re_exported_module: &ModuleId) -> bool {
        self.re_exports
            .get(exporting_module)
            .map(|modules| modules.contains(re_exported_module))
            .unwrap_or(false)
    }

    /// Find all modules that re-export the given target module
    /// 
    /// This is useful for reverse lookups: "which modules make target_module's items available?"
    pub fn modules_that_re_export(&self, target_module: &ModuleId) -> Vec<&ModuleId> {
        self.re_exports
            .iter()
            .filter_map(|(exporting_module, re_exported_modules)| {
                if re_exported_modules.contains(target_module) {
                    Some(exporting_module)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the total number of re-export relationships
    pub fn re_export_count(&self) -> usize {
        self.re_exports.values().map(|v| v.len()).sum()
    }

    /// Check if any re-exports are defined
    pub fn has_any_re_exports(&self) -> bool {
        !self.re_exports.is_empty()
    }

    /// Check if adding a re-export would create a cycle
    /// 
    /// This prevents infinite loops in re-export chains by detecting if 
    /// `target_module` already re-exports `exporting_module` (directly or indirectly).
    pub fn would_create_cycle(&self, exporting_module: &ModuleId, target_module: &ModuleId) -> bool {
        use std::collections::HashSet;
        
        let mut visited = HashSet::new();
        self.has_re_export_path(target_module, exporting_module, &mut visited)
    }

    /// Check if there's a re-export path from `from_module` to `to_module`
    /// 
    /// Used internally for cycle detection.
    fn has_re_export_path(&self, from_module: &ModuleId, to_module: &ModuleId, visited: &mut std::collections::HashSet<ModuleId>) -> bool {
        // Avoid infinite recursion
        if visited.contains(from_module) {
            return false;
        }
        visited.insert(from_module.clone());

        // Direct re-export
        if let Some(re_exported_modules) = self.re_exports.get(from_module) {
            if re_exported_modules.contains(to_module) {
                return true;
            }
            
            // Indirect re-export through other modules
            for intermediate_module in re_exported_modules {
                if self.has_re_export_path(intermediate_module, to_module, visited) {
                    return true;
                }
            }
        }
        
        false
    }

    /// Resolve the complete re-export chain for a module with cycle detection
    /// 
    /// Returns all modules that are transitively re-exported by the given module.
    /// The `visited` set is used for cycle detection and should be empty on the initial call.
    pub fn resolve_re_export_chain(&self, module_id: &ModuleId, visited: &mut std::collections::HashSet<ModuleId>) -> Vec<ModuleId> {
        use std::collections::HashSet;
        
        let mut result = Vec::new();
        
        // Avoid infinite recursion
        if visited.contains(module_id) {
            return result;
        }
        visited.insert(module_id.clone());

        if let Some(re_exported_modules) = self.re_exports.get(module_id) {
            for re_exported in re_exported_modules {
                // Add the directly re-exported module
                result.push(re_exported.clone());
                
                // Recursively add transitively re-exported modules
                let mut transitive = self.resolve_re_export_chain(re_exported, visited);
                result.append(&mut transitive);
            }
        }
        
        result
    }

    /// Get all items visible through a module, including re-exported items
    /// 
    /// This includes:
    /// 1. Items directly defined in the module
    /// 2. Items from modules that this module re-exports (transitively)
    /// 
    /// Note: This returns a HashMap where later entries may shadow earlier ones.
    /// The caller should handle name conflicts according to their resolution rules.
    pub fn get_all_visible_items(&self, module_id: &ModuleId) -> std::collections::HashMap<ItemName, ItemId> {
        use std::collections::{HashMap, HashSet};
        
        let mut result = HashMap::new();
        let mut visited = HashSet::new();
        
        self.collect_visible_items_recursive(module_id, &mut result, &mut visited);
        
        result
    }

    /// Recursively collect items from a module and its re-exports
    fn collect_visible_items_recursive(
        &self, 
        module_id: &ModuleId, 
        result: &mut std::collections::HashMap<ItemName, ItemId>,
        visited: &mut std::collections::HashSet<ModuleId>
    ) {
        // Avoid infinite recursion
        if visited.contains(module_id) {
            return;
        }
        visited.insert(module_id.clone());

        // Collect items directly defined in this module
        for item in self.items.values() {
            if let Some(item_module_id) = item.id().parent_module_id() {
                if item_module_id == *module_id {
                    let (_, item_name) = item.id().split();
                    result.insert(item_name, item.id().clone());
                }
            }
        }

        // Recursively collect items from re-exported modules
        if let Some(re_exported_modules) = self.re_exports.get(module_id) {
            for re_exported in re_exported_modules {
                self.collect_visible_items_recursive(re_exported, result, visited);
            }
        }
    }

    // ============================================================================
    // Runtime Symbol Resolution with Incremental Support
    // ============================================================================

    /// Resolve a symbol at runtime with proper precedence rules
    /// 
    /// This supports incremental compilation by resolving symbols dynamically,
    /// allowing changes to be picked up without recompilation.
    pub fn resolve_symbol_incremental(
        &self,
        module_id: &ModuleId,
        symbol_name: &str,
        symbol_kind: ItemKind,
    ) -> Result<ItemId, ResolutionError> {
        let mut candidates = Vec::new();

        // 1. Direct items (highest precedence)
        candidates.extend(
            self.find_direct_items(module_id, symbol_name, symbol_kind)
                .into_iter()
                .map(|id| (id, ImportPrecedence::DirectItem))
        );

        // 2. Explicit imports (medium precedence)
        candidates.extend(
            self.find_explicit_imports(module_id, symbol_name, symbol_kind)
                .into_iter()
                .map(|id| (id, ImportPrecedence::ExplicitImport))
        );

        // 3. Glob imports (lowest precedence) - RUNTIME RESOLUTION
        candidates.extend(
            self.find_glob_imports_runtime(module_id, symbol_name, symbol_kind)
                .into_iter()
                .map(|id| (id, ImportPrecedence::GlobImport))
        );

        self.resolve_with_precedence(candidates)
    }

    /// Find items directly defined in a module
    pub fn find_direct_items(
        &self,
        module_id: &ModuleId,
        symbol_name: &str,
        symbol_kind: ItemKind,
    ) -> Vec<ItemId> {
        let mut results = Vec::new();
        
        for item in self.items.values() {
            if let Some(item_module_id) = item.id().parent_module_id() {
                if item_module_id == *module_id {
                    let (_, item_name) = item.id().split();
                    if item_name.name.as_ref() == symbol_name && item_name.kind == symbol_kind {
                        results.push(item.id().clone());
                    }
                }
            }
        }
        
        results
    }

    /// Find items imported with explicit imports (use module::item)
    pub fn find_explicit_imports(
        &self,
        module_id: &ModuleId,
        symbol_name: &str,
        symbol_kind: ItemKind,
    ) -> Vec<ItemId> {
        let mut results = Vec::new();
        
        // Look for Alias items in this module that match the symbol
        for item in self.items.values() {
            if let Item::Alias(alias) = item.as_ref() {
                if let Some(alias_module_id) = alias.id.parent_module_id() {
                    if alias_module_id == *module_id {
                        let (_, alias_name) = alias.id.split();
                        if alias_name.name.as_ref() == symbol_name && alias_name.kind == symbol_kind {
                            results.push(alias.target.clone());
                        }
                    }
                }
            }
        }
        
        results
    }

    /// Find items through glob imports (use module::*) - runtime resolution
    fn find_glob_imports_runtime(
        &self,
        module_id: &ModuleId,
        symbol_name: &str,
        symbol_kind: ItemKind,
    ) -> Vec<ItemId> {
        let mut results = Vec::new();
        let mut visited = std::collections::HashSet::new();

        // Follow re-export chains to find symbols
        if let Some(re_exported_modules) = self.get_re_exports(module_id) {
            for target_module in re_exported_modules {
                self.search_module_for_symbol(
                    target_module,
                    symbol_name,
                    symbol_kind,
                    &mut results,
                    &mut visited,
                );
            }
        }

        results
    }

    /// Recursively search a module for a specific symbol
    fn search_module_for_symbol(
        &self,
        module_id: &ModuleId,
        symbol_name: &str,
        symbol_kind: ItemKind,
        results: &mut Vec<ItemId>,
        visited: &mut std::collections::HashSet<ModuleId>,
    ) {
        if visited.contains(module_id) {
            return; // Cycle detection
        }
        visited.insert(module_id.clone());

        // Search direct items in this module
        results.extend(self.find_direct_items(module_id, symbol_name, symbol_kind));

        // Recursively search re-exported modules
        if let Some(re_exported_modules) = self.get_re_exports(module_id) {
            for re_exported in re_exported_modules {
                self.search_module_for_symbol(
                    re_exported,
                    symbol_name,
                    symbol_kind,
                    results,
                    visited,
                );
            }
        }
    }

    /// Resolve conflicts using precedence rules (like Rust)
    fn resolve_with_precedence(
        &self,
        candidates: Vec<(ItemId, ImportPrecedence)>,
    ) -> Result<ItemId, ResolutionError> {
        if candidates.is_empty() {
            return Err(ResolutionError::NotFound);
        }

        // Group by precedence level
        let min_precedence = candidates.iter().map(|(_, p)| p).min().unwrap().clone();
        let best_candidates: Vec<_> = candidates
            .into_iter()
            .filter(|(_, p)| p == &min_precedence)
            .map(|(id, _)| id)
            .collect();

        match best_candidates.len() {
            1 => Ok(best_candidates[0].clone()),
            _ => Err(ResolutionError::Ambiguous {
                symbol_name: "unknown".to_string(), // TODO: pass symbol name
                candidates: best_candidates,
            }),
        }
    }
}

/// Symbol import precedence (following Rust's rules)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportPrecedence {
    DirectItem = 0,     // Highest priority
    ExplicitImport = 1, // Medium priority  
    GlobImport = 2,     // Lowest priority
}

/// Symbol resolution errors
#[derive(Debug, Clone)]
pub enum ResolutionError {
    NotFound,
    Ambiguous {
        symbol_name: String,
        candidates: Vec<ItemId>,
    },
    CyclicImport {
        cycle: Vec<ModuleId>,
    },
}

impl Default for ItemRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::symbol_table::InternedSymbol;

    #[test]
    fn test_registry_basic_operations() {
        let mut registry = ItemRegistry::new();
        assert!(registry.is_empty());

        // Create a test module
        let module = Module {
            id: ModuleId::from_path("::test_module"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };

        // Add the module
        assert!(registry.add_module(module.clone()));
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        // Try to add the same module again
        assert!(!registry.add_module(module.clone()));
        assert_eq!(registry.len(), 1);

        // Retrieve the module
        let retrieved = registry.get_module(&module.id).unwrap();
        assert_eq!(retrieved.id.id.to_string().as_str(), "::test_module");

        // Remove the module
        let removed = registry.remove_item(&module.id.id).unwrap();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());

        match removed.as_ref() {
            Item::Module(m) => assert_eq!(m.id.id.to_string().as_str(), "::test_module"),
            _ => panic!("Expected module"),
        }
    }

    #[test]
    fn test_registry_different_item_types() {
        let mut registry = ItemRegistry::new();

        // Add a module
        let module = Module {
            id: ModuleId::from_path("::test"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };
        registry.add_module(module);

        // Add a type with the same name (different namespace)
        let type_def = TypeDefinition {
            id: TypeId::from_path("::test"),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("test"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        };
        registry.add_type(type_def);

        // Add a predicate with the same name (different namespace)
        let predicate = Predicate {
            id: PredicateId::from_path("::test"),
            parameters: vec![],
            body: StructuralGoal::empty_container(),
            kind: PredicateKind::Relation,
            visibility: Visibility::Public,
        };
        registry.add_predicate(predicate);

        // All three should coexist
        assert_eq!(registry.len(), 3);

        // Check we can retrieve each by their specific type
        assert!(registry
            .get_module(&ModuleId::from_path("::test"))
            .is_some());
        assert!(registry.get_type(&TypeId::from_path("::test")).is_some());
        assert!(registry
            .get_predicate(&PredicateId::from_path("::test"))
            .is_some());
    }

    /*
    #[test]
    fn test_registry_module_hierarchy() {
        let mut registry = ItemRegistry::new();

        // Create parent module
        let parent = Module {
            id: ModuleId::from_path("::parent"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };
        registry.add_module(parent);

        // Create child modules
        let child1 = Module {
            id: ModuleId::from_path("::parent::child1"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };
        registry.add_module(child1);

        let child2 = Module {
            id: ModuleId::from_path("::parent::child2"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };
        registry.add_module(child2);

        // Create grandchild
        let grandchild = Module {
            id: ModuleId::from_path("::parent::child1::grandchild"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };
        registry.add_module(grandchild);

        assert_eq!(registry.len(), 4);

        // Test child module lookup
        let children: Vec<_> = registry.child_modules("::parent").collect();
        assert_eq!(children.len(), 2);

        let child_paths: Vec<String> = children.iter().map(|m| m.id.id.to_string()).collect();
        assert!(child_paths.contains(&"::parent::child1".to_string()));
        assert!(child_paths.contains(&"::parent::child2".to_string()));

        // Grandchild should not be included in direct children
        assert!(!child_paths.contains(&"::parent::child1::grandchild".to_string()));
    }

    #[test]
    fn test_registry_remove_module_tree() {
        let mut registry = ItemRegistry::new();

        // Build a module hierarchy with types and predicates
        registry.add_module(Module {
            id: ModuleId::from_path("::parent"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        });

        registry.add_module(Module {
            id: ModuleId::from_path("::parent::child"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        });

        registry.add_type(TypeDefinition {
            id: TypeId::from_path("::parent::child::MyType"),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("MyType"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        });

        registry.add_predicate(Predicate {
            id: PredicateId::from_path("::parent::child::my_pred"),
            parameters: vec![],
            body: StructuralGoal::empty_container(),
            kind: PredicateKind::Relation,
            visibility: Visibility::Public,
        });

        assert_eq!(registry.len(), 4);

        // Remove the parent module tree
        let parent_module_id = ModuleId::from_path("::parent");
        let removed = registry.remove_module_tree(&parent_module_id);

        // Should have removed parent, child, type, and predicate
        assert_eq!(removed.len(), 4);
        assert_eq!(registry.len(), 0);
    }
    */

    #[test]
    fn test_items_by_kind() {
        let mut registry = ItemRegistry::new();

        // Add items of different kinds
        registry.add_module(Module {
            id: ModuleId::from_path("::mod1"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        });

        registry.add_module(Module {
            id: ModuleId::from_path("::mod2"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        });

        registry.add_type(TypeDefinition {
            id: TypeId::from_path("::Type1"),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("Type1"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        });

        registry.add_predicate(Predicate {
            id: PredicateId::from_path("::pred1"),
            parameters: vec![],
            body: StructuralGoal::empty_container(),
            kind: PredicateKind::Relation,
            visibility: Visibility::Public,
        });

        // Test filtering by kind
        assert_eq!(registry.modules().count(), 2);
        assert_eq!(registry.types().count(), 1);
        assert_eq!(registry.predicates().count(), 1);
        assert_eq!(registry.items_of_kind(ItemKind::Module).count(), 2);
        assert_eq!(registry.items_of_kind(ItemKind::Type).count(), 1);
        assert_eq!(registry.items_of_kind(ItemKind::Predicate).count(), 1);
    }

    #[test]
    fn test_as_ref_item_id() {
        let mut registry = ItemRegistry::new();

        // Create a type reference
        let type_ref = TypeId::from_path("::TestType");
        let type_def = TypeDefinition {
            id: type_ref.clone(),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("TestStruct"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        };

        registry.add_type(type_def);

        // Test that we can use TypeId directly with AsRef<ItemId> methods
        assert!(registry.contains_item(&type_ref));
        assert!(registry.get_item(&type_ref).is_some());
        assert!(registry.get_type(&type_ref).is_some());

        // Test with ModuleId
        let module_ref = ModuleId::from_path("::TestModule");
        let module = Module {
            id: module_ref.clone(),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };

        registry.add_module(module);

        assert!(registry.contains_item(&module_ref));
        assert!(registry.get_module(&module_ref).is_some());

        // Test with PredicateId
        let predicate_ref = PredicateId::from_path("::test_pred");
        let predicate = Predicate {
            id: predicate_ref.clone(),
            parameters: vec![],
            body: StructuralGoal::empty_container(),
            kind: PredicateKind::Relation,
            visibility: Visibility::Public,
        };

        registry.add_predicate(predicate);

        assert!(registry.contains_item(&predicate_ref));
        assert!(registry.get_predicate(&predicate_ref).is_some());

        // Test AsRef implementation directly
        let item_id: &ItemId = type_ref.as_ref();
        assert_eq!(item_id.to_string().as_str(), "::TestType");
        assert_eq!(item_id.kind(), ItemKind::Type);
    }

    /*
    #[test]
    fn test_incremental_module_item_updates() {
        let mut registry = ItemRegistry::new();

        // Create a parent module
        let parent_module = Module {
            id: ModuleId::from_path("::parent"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };
        registry.add_module(parent_module);

        // Add a child type to the parent module
        let child_type = TypeDefinition {
            id: TypeId::from_path("::parent::ChildType"),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("ChildType"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        };

        let child_type_name = child_type.id.id.name.clone();
        let child_type_module_path = child_type.id.id.module_path.clone();
        registry.add_type(child_type);

        // Check that parent module's items list was updated
        let parent = registry
            .get_module(&ModuleId::from_path("::parent"))
            .unwrap();
        assert!(parent.items.contains_key(&child_type_name));

        // Remove the child type
        let child_type_id = ItemId::new(child_type_module_path, child_type_name.clone());
        registry.remove_item(&child_type_id);

        // Check that parent module's items list was updated
        let parent = registry
            .get_module(&ModuleId::from_path("::parent"))
            .unwrap();
        assert!(!parent.items.contains_key(&child_type_name));
    }
    */

    #[test]
    fn test_immutable_update_methods() {
        let registry = ItemRegistry::new();

        // Test immutable add with with_item
        let module = Module {
            id: ModuleId::from_path("::test_module"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };

        let (new_registry, was_added) = registry.with_module(module.clone());
        assert!(was_added);
        assert_eq!(registry.len(), 0); // Original unchanged
        assert_eq!(new_registry.len(), 1); // New registry has the item
        assert!(new_registry.get_module(&module.id).is_some());

        // Test immutable remove with without_item
        let (final_registry, removed_item) = new_registry.without_item(&module.id.id);
        assert!(removed_item.is_some());
        assert_eq!(new_registry.len(), 1); // Previous unchanged
        assert_eq!(final_registry.len(), 0); // Final registry is empty

        // Verify structural sharing works (cheap clone)
        let registry_copy = registry.clone();
        assert_eq!(registry.len(), registry_copy.len());
    }

    #[test]
    fn test_parent_reference_inference() {
        let mut registry = ItemRegistry::new();

        // Add parent module first
        let parent = Module {
            id: ModuleId::from_path("::parent"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };
        registry.add_module(parent);

        // Add child module without explicit parent reference
        let child = Module {
            id: ModuleId::from_path("::parent::child"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };
        registry.add_module(child);

        // Verify parent reference can be derived from the module path
        let retrieved_child = registry
            .get_module(&ModuleId::from_path("::parent::child"))
            .unwrap();
        let expected_parent_path = ModuleId::from_path("::parent").full_path();
        assert_eq!(retrieved_child.id.parent_path(), Some(expected_parent_path));

        // Test with deeper nesting
        let grandchild = Module {
            id: ModuleId::from_path("::parent::child::grandchild"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };
        registry.add_module(grandchild);

        let retrieved_grandchild = registry
            .get_module(&ModuleId::from_path("::parent::child::grandchild"))
            .unwrap();
        let expected_grandparent_path = ModuleId::from_path("::parent::child").full_path();
        assert_eq!(
            retrieved_grandchild.id.parent_path(),
            Some(expected_grandparent_path)
        );

        // Test root module (no parent)
        let root = Module {
            id: ModuleId::from_path("::root"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };
        registry.add_module(root);

        let retrieved_root = registry.get_module(&ModuleId::from_path("::root")).unwrap();
        assert_eq!(retrieved_root.id.parent_path(), Some(Rc::new(ModulePath::root())));
    }

    /*
    #[test]
    fn test_parent_reference_consistency() {
        let mut registry = ItemRegistry::new();

        // Create hierarchy with explicit parent references
        let parent = Module {
            id: ModuleId::from_path("::parent"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };
        registry.add_module(parent);

        // Add child with explicit parent reference
        let child = Module {
            id: ModuleId::from_path("::parent::child"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };
        registry.add_module(child);

        // Add child type - should be automatically added to parent's items
        let child_type = TypeDefinition {
            id: TypeId::from_path("::parent::child::ChildType"),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("ChildType"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        };
        registry.add_type(child_type.clone());

        // Verify the child module's items list was updated
        let retrieved_child = registry
            .get_module(&ModuleId::from_path("::parent::child"))
            .unwrap();
        assert!(retrieved_child.items.contains_key(&child_type.id.id.name));

        // Verify the parent module's items list includes the child module
        let retrieved_parent = registry
            .get_module(&ModuleId::from_path("::parent"))
            .unwrap();
        let child_module_name = ItemName::new_unchecked("child", ItemKind::Module);
        assert!(retrieved_parent.items.contains_key(&child_module_name));
    }
    */

    #[test]
    fn test_generic_remove_methods() {
        let mut registry = ItemRegistry::new();

        // Add items
        let module = Module {
            id: ModuleId::from_path("::test_module"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };
        let module_id = module.id.clone();
        registry.add_module(module);

        let type_def = TypeDefinition {
            id: TypeId::from_path("::TestType"),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("TestType"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        };
        let type_id = type_def.id.clone();
        registry.add_type(type_def);

        let predicate = Predicate {
            id: PredicateId::from_path("::test_pred"),
            parameters: vec![],
            body: StructuralGoal::empty_container(),
            kind: PredicateKind::Relation,
            visibility: Visibility::Public,
        };
        let predicate_id = predicate.id.clone();
        registry.add_predicate(predicate);

        assert_eq!(registry.len(), 3);

        // Test remove_item with different AsRef<ItemId> types

        // Remove with ItemId directly
        let removed_module = registry.remove_item(module_id.id);
        assert!(removed_module.is_some());
        assert_eq!(registry.len(), 2);

        // Remove with TypeId (which implements AsRef<ItemId>)
        let removed_type = registry.remove_item(&type_id);
        assert!(removed_type.is_some());
        assert_eq!(registry.len(), 1);

        // Remove with PredicateId (which implements AsRef<ItemId>)
        let removed_predicate = registry.remove_item(&predicate_id);
        assert!(removed_predicate.is_some());
        assert_eq!(registry.len(), 0);

        // Test immutable without_item with different types
        let registry = ItemRegistry::new();
        let (registry_with_module, _) = registry.with_module(Module {
            id: ModuleId::from_path("::another_module"),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        });

        let module_id = ModuleId::from_path("::another_module");
        let (final_registry, removed) = registry_with_module.without_item(&module_id);
        assert!(removed.is_some());
        assert_eq!(final_registry.len(), 0);
    }

    #[test]
    fn test_all_methods_generic_asref() {
        let mut registry = ItemRegistry::new();

        // Create items with different ID types
        let module_id = ModuleId::from_path("::test_module");
        let type_id = TypeId::from_path("::TestType");
        let predicate_id = PredicateId::from_path("::test_pred");

        let module = Module {
            id: module_id.clone(),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };

        let type_def = TypeDefinition {
            id: type_id.clone(),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("TestStruct"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        };

        let predicate = Predicate {
            id: predicate_id.clone(),
            parameters: vec![Parameter {
                name: InternedSymbol::from_text("x"),
                type_annotation: None,
            }],
            body: StructuralGoal::empty_container(),
            kind: PredicateKind::Relation,
            visibility: Visibility::Public,
        };

        registry.add_module(module);
        registry.add_type(type_def);
        registry.add_predicate(predicate);

        // Test all generic methods work with typed IDs

        // get_item
        assert!(registry.get_item(&module_id).is_some());
        assert!(registry.get_item(&type_id).is_some());
        assert!(registry.get_item(&predicate_id).is_some());

        // contains_item
        assert!(registry.contains_item(&module_id));
        assert!(registry.contains_item(&type_id));
        assert!(registry.contains_item(&predicate_id));

        // get_item_for_mutation
        assert!(registry.get_item_for_mutation(&module_id).is_some());
        assert!(registry.get_item_for_mutation(&type_id).is_some());
        assert!(registry.get_item_for_mutation(&predicate_id).is_some());

        // Specialized getters
        assert!(registry.get_module(&module_id).is_some());
        assert!(registry.get_type(&type_id).is_some());
        assert!(registry.get_predicate(&predicate_id).is_some());

        // Arity getter
        assert_eq!(registry.get_predicate_arity(&predicate_id), Some(1));

        // Test with raw ItemId as well
        let raw_module_id = module_id.id.clone();
        let raw_type_id = type_id.id.clone();
        let raw_predicate_id = predicate_id.id.clone();

        assert!(registry.get_item(&raw_module_id).is_some());
        assert!(registry.contains_item(&raw_type_id));
        assert!(registry.get_item_for_mutation(&raw_predicate_id).is_some());

        // Test remove_item
        assert!(registry.remove_item(&module_id).is_some()); // Remove with ModuleId
        assert!(registry.remove_item(raw_type_id).is_some()); // Remove with ItemId
        assert!(registry.remove_item(&predicate_id).is_some()); // Remove with PredicateId

        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_predicate_arity_methods() {
        let mut registry = ItemRegistry::new();

        // Create a predicate with 3 parameters
        let predicate = Predicate {
            id: PredicateId::from_path("::test_pred"),
            parameters: vec![
                Parameter {
                    name: InternedSymbol::from_text("x"),
                    type_annotation: None,
                },
                Parameter {
                    name: InternedSymbol::from_text("y"),
                    type_annotation: None,
                },
                Parameter {
                    name: InternedSymbol::from_text("z"),
                    type_annotation: None,
                },
            ],
            body: StructuralGoal::empty_container(),
            kind: PredicateKind::Relation,
            visibility: Visibility::Public,
        };

        let predicate_ref = predicate.id.clone();
        registry.add_predicate(predicate);

        // Test arity methods
        assert_eq!(registry.get_predicate_arity(&predicate_ref), Some(3));

        // Test with non-existent predicate
        let non_existent = PredicateId::from_path("::non_existent");
        assert_eq!(registry.get_predicate_arity(&non_existent), None);
    }

    #[test]
    fn test_basic_re_exports() {
        let mut registry = ItemRegistry::new();
        
        let module_a = ModuleId::from_path("::a");
        let module_b = ModuleId::from_path("::b");
        let module_c = ModuleId::from_path("::c");

        // Test adding re-exports
        registry.add_re_export(module_a.clone(), module_b.clone());
        registry.add_re_export(module_a.clone(), module_c.clone());

        // Test querying re-exports
        assert!(registry.has_re_export(&module_a, &module_b));
        assert!(registry.has_re_export(&module_a, &module_c));
        assert!(!registry.has_re_export(&module_b, &module_a));

        let re_exports = registry.get_re_exports(&module_a).unwrap();
        assert_eq!(re_exports.len(), 2);
        assert!(re_exports.contains(&module_b));
        assert!(re_exports.contains(&module_c));

        // Test reverse lookup
        let modules_re_exporting_b = registry.modules_that_re_export(&module_b);
        assert_eq!(modules_re_exporting_b.len(), 1);
        assert_eq!(modules_re_exporting_b[0], &module_a);

        // Test counts
        assert_eq!(registry.re_export_count(), 2);
        assert!(registry.has_any_re_exports());
    }

    #[test]
    fn test_remove_re_exports() {
        let mut registry = ItemRegistry::new();
        
        let module_a = ModuleId::from_path("::a");
        let module_b = ModuleId::from_path("::b");
        let module_c = ModuleId::from_path("::c");

        // Add re-exports
        registry.add_re_export(module_a.clone(), module_b.clone());
        registry.add_re_export(module_a.clone(), module_c.clone());

        // Remove one re-export
        assert!(registry.remove_re_export(&module_a, &module_b));
        assert!(!registry.has_re_export(&module_a, &module_b));
        assert!(registry.has_re_export(&module_a, &module_c));

        // Try to remove non-existent re-export
        assert!(!registry.remove_re_export(&module_a, &module_b));

        // Remove last re-export (should clean up empty entry)
        assert!(registry.remove_re_export(&module_a, &module_c));
        assert!(registry.get_re_exports(&module_a).is_none());
        assert_eq!(registry.re_export_count(), 0);
        assert!(!registry.has_any_re_exports());
    }

    #[test]
    fn test_clear_re_exports() {
        let mut registry = ItemRegistry::new();
        
        let module_a = ModuleId::from_path("::a");
        let module_b = ModuleId::from_path("::b");
        let module_c = ModuleId::from_path("::c");

        // Add re-exports
        registry.add_re_export(module_a.clone(), module_b.clone());
        registry.add_re_export(module_a.clone(), module_c.clone());
        
        // Clear all re-exports for module_a
        registry.clear_re_exports(&module_a);
        
        assert!(registry.get_re_exports(&module_a).is_none());
        assert_eq!(registry.re_export_count(), 0);
    }

    #[test]
    fn test_cycle_detection() {
        let mut registry = ItemRegistry::new();
        
        let module_a = ModuleId::from_path("::a");
        let module_b = ModuleId::from_path("::b");
        let module_c = ModuleId::from_path("::c");

        // Create a chain: A -> B -> C
        registry.add_re_export(module_a.clone(), module_b.clone());
        registry.add_re_export(module_b.clone(), module_c.clone());

        // Direct cycle detection: C -> A would create a cycle
        assert!(registry.would_create_cycle(&module_c, &module_a));
        
        // Indirect cycle detection: C -> B would create a cycle (B already leads to C)
        assert!(registry.would_create_cycle(&module_c, &module_b));
        
        // No cycle: A -> C is fine (even though A -> B -> C exists)
        assert!(!registry.would_create_cycle(&module_a, &module_c));
    }

    #[test]
    fn test_re_export_chain_resolution() {
        let mut registry = ItemRegistry::new();
        
        let module_a = ModuleId::from_path("::a");
        let module_b = ModuleId::from_path("::b");
        let module_c = ModuleId::from_path("::c");
        let module_d = ModuleId::from_path("::d");

        // Create chain: A -> B, B -> C, B -> D
        registry.add_re_export(module_a.clone(), module_b.clone());
        registry.add_re_export(module_b.clone(), module_c.clone());
        registry.add_re_export(module_b.clone(), module_d.clone());

        let mut visited = std::collections::HashSet::new();
        let chain = registry.resolve_re_export_chain(&module_a, &mut visited);
        
        // Should contain B (direct), C and D (transitive through B)
        assert_eq!(chain.len(), 3);
        assert!(chain.contains(&module_b));
        assert!(chain.contains(&module_c));
        assert!(chain.contains(&module_d));
    }

    #[test]
    fn test_visible_items_with_re_exports() {
        let mut registry = ItemRegistry::new();

        // Create modules
        let module_a = Module {
            id: ModuleId::from_path("::a"),
            visibility: Visibility::Public,
        };
        let module_b = Module {
            id: ModuleId::from_path("::b"),
            visibility: Visibility::Public,
        };
        registry.add_module(module_a.clone());
        registry.add_module(module_b.clone());

        // Add a type to module B
        let type_in_b = TypeDefinition {
            id: TypeId::from_path("::b::TypeInB"),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("TypeInB"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        };
        registry.add_type(type_in_b);

        // Add a predicate to module A
        let pred_in_a = Predicate {
            id: PredicateId::from_path("::a::pred_in_a"),
            parameters: vec![],
            body: StructuralGoal::empty_container(),
            kind: PredicateKind::Relation,
            visibility: Visibility::Public,
        };
        registry.add_predicate(pred_in_a);

        // A re-exports B
        registry.add_re_export(module_a.id.clone(), module_b.id.clone());

        // Get all visible items through module A
        let visible_items = registry.get_all_visible_items(&module_a.id);
        
        // Should contain both the predicate from A and the type from B
        assert_eq!(visible_items.len(), 2);
        
        let pred_name = ItemName::new("pred_in_a", ItemKind::Predicate).unwrap();
        let type_name = ItemName::new("TypeInB", ItemKind::Type).unwrap();
        
        assert!(visible_items.contains_key(&pred_name));
        assert!(visible_items.contains_key(&type_name));
    }

    #[test]
    fn test_immutable_re_export_operations() {
        let registry = ItemRegistry::new();
        
        let module_a = ModuleId::from_path("::a");
        let module_b = ModuleId::from_path("::b");

        // Test immutable add
        let new_registry = registry.with_re_export(module_a.clone(), module_b.clone());
        
        // Original registry unchanged
        assert!(!registry.has_re_export(&module_a, &module_b));
        
        // New registry has the re-export
        assert!(new_registry.has_re_export(&module_a, &module_b));
        
        // Test immutable remove
        let (final_registry, was_removed) = new_registry.without_re_export(&module_a, &module_b);
        assert!(was_removed);
        assert!(!final_registry.has_re_export(&module_a, &module_b));
        
        // Test removing non-existent
        let (unchanged_registry, was_removed) = registry.without_re_export(&module_a, &module_b);
        assert!(!was_removed);
        assert_eq!(unchanged_registry.re_export_count(), 0);
    }
}
