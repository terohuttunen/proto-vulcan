//! Unified item registry for the IR
//! 
//! This registry stores all items (modules, types, predicates) using stable
//! ItemId keys, supporting dynamic addition and removal while maintaining
//! immutability through Rc::make_mut.

use super::*;
use im_rc::HashMap;
use crate::interpreter::symbol_table::InternedSymbol;

/// Unified registry for all IR items
/// Uses ItemId for stable, path-based identification with im-rc HashMap for efficient structural sharing
#[derive(Debug, Clone)]
pub struct ItemRegistry {
    /// All items stored by their ItemId with structural sharing via im-rc HashMap
    items: HashMap<ItemId, Item>,
}

impl ItemRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }
    
    /// Add an item to the registry
    /// Returns true if the item was added, false if it already existed
    pub fn add_item(&mut self, item: Item) -> bool {
        let id = item.id().clone();
        let was_new = self.items.insert(id.clone(), item).is_none();
        
        // Update parent module's item list if this is a new item
        if was_new {
            self.update_parent_module_items(&id);
        }
        
        was_new
    }
    
    /// Remove an item from the registry
    /// Returns the removed item if it existed
    pub fn remove_item<T: AsRef<ItemId>>(&mut self, id: T) -> Option<Item> {
        let id_ref = id.as_ref();
        let removed = self.items.remove(id_ref);
        
        // Update parent module's item list if item was removed
        if removed.is_some() {
            self.remove_from_parent_module_items(id_ref);
        }
        
        removed
    }
    
    /// Replace an existing item in the registry
    /// Returns the old item if it existed
    pub fn replace_item(&mut self, item: Item) -> Option<Item> {
        let id = item.id().clone();
        self.items.insert(id, item)
    }
    
    /// Get an item by its ItemId
    pub fn get_item<T: AsRef<ItemId>>(&self, id: T) -> Option<&Item> {
        self.items.get(id.as_ref())
    }
    
    /// Get a cloned item for mutation
    pub fn get_item_for_mutation<T: AsRef<ItemId>>(&self, id: T) -> Option<Item> {
        self.items.get(id.as_ref()).cloned()
    }
    
    /// Check if an item exists
    pub fn contains_item<T: AsRef<ItemId>>(&self, id: T) -> bool {
        self.items.contains_key(id.as_ref())
    }
    
    /// Get all items
    pub fn all_items(&self) -> impl Iterator<Item = &Item> {
        self.items.values()
    }
    
    /// Get all items of a specific kind
    pub fn items_of_kind(&self, kind: ItemKind) -> impl Iterator<Item = &Item> {
        self.items.values().filter(move |item| item.id().kind == kind)
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
        match self.get_item(module_ref)? {
            Item::Module(module) => Some(module),
            _ => None,
        }
    }
    
    /// Get a type by TypeId or any reference that can be converted to ItemId
    pub fn get_type<T: AsRef<ItemId>>(&self, type_ref: T) -> Option<&TypeDefinition> {
        match self.get_item(type_ref)? {
            Item::Type(type_def) => Some(type_def),
            _ => None,
        }
    }
    
    /// Get a predicate by PredicateId or any reference that can be converted to ItemId
    pub fn get_predicate<T: AsRef<ItemId>>(&self, predicate_ref: T) -> Option<&Predicate> {
        match self.get_item(predicate_ref)? {
            Item::Predicate(predicate) => Some(predicate),
            _ => None,
        }
    }
    
    /// Get all modules
    pub fn modules(&self) -> impl Iterator<Item = &Module> {
        self.items_of_kind(ItemKind::Module).filter_map(|item| match item {
            Item::Module(module) => Some(module),
            _ => None,
        })
    }
    
    /// Get all types
    pub fn types(&self) -> impl Iterator<Item = &TypeDefinition> {
        self.items_of_kind(ItemKind::Type).filter_map(|item| match item {
            Item::Type(type_def) => Some(type_def),
            _ => None,
        })
    }
    
    /// Get all predicates
    pub fn predicates(&self) -> impl Iterator<Item = &Predicate> {
        self.items_of_kind(ItemKind::Predicate).filter_map(|item| match item {
            Item::Predicate(predicate) => Some(predicate),
            _ => None,
        })
    }
    
    /// Get the arity (number of parameters) of a predicate by PredicateId or any reference that can be converted to ItemId
    pub fn get_predicate_arity<T: AsRef<ItemId>>(&self, predicate_ref: T) -> Option<usize> {
        self.get_predicate(predicate_ref).map(|p| p.parameters.len())
    }
    
    /// Find items by path prefix (useful for module traversal)
    pub fn find_by_path_prefix<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = &'a Item> + 'a {
        self.items.values().filter(move |item| {
            item.id().path.starts_with(prefix)
        })
    }
    
    /// Find modules that are children of a given module
    /// Uses the Module.items field for O(k) performance instead of O(n) registry scanning
    pub fn child_modules(&self, parent_path: &str) -> impl Iterator<Item = &Module> {
        let parent_id = ItemId::new(parent_path, ItemKind::Module);
        
        // Get the parent module and iterate through its direct children
        self.get_item(&parent_id)
            .and_then(|item| match item {
                Item::Module(parent_module) => Some(parent_module),
                _ => None,
            })
            .into_iter()
            .flat_map(move |parent_module| {
                parent_module.items.iter().filter_map(move |item_id| {
                    // Only return modules (filter out types and predicates)
                    if item_id.kind == ItemKind::Module {
                        match self.get_item(item_id)? {
                            Item::Module(module) => Some(module),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
            })
    }
    
    /// Add a module to the registry
    /// Automatically handles parent reference consistency
    pub fn add_module(&mut self, mut module: Module) -> bool {
        // Ensure parent reference is consistent with module path
        if module.parent.is_none() {
            // Try to infer parent from path if not explicitly set
            let module_path = &module.id.id.path;
            if let Some(last_colon_pos) = module_path.rfind("::") {
                if last_colon_pos > 2 { // More than just "::" at the start
                    let parent_path = &module_path[..last_colon_pos];
                    module.parent = Some(ModuleId::new(parent_path));
                }
            }
        }
        
        self.add_item(Item::Module(module))
    }
    
    /// Add a type to the registry
    pub fn add_type(&mut self, type_def: TypeDefinition) -> bool {
        self.add_item(Item::Type(type_def))
    }
    
    /// Add a predicate to the registry
    pub fn add_predicate(&mut self, predicate: Predicate) -> bool {
        self.add_item(Item::Predicate(predicate))
    }
    
    // Immutable update methods leveraging im-rc's persistent data structures
    
    /// Create a new registry with an item added (immutable operation)
    /// Returns the new registry and whether the item was actually added
    pub fn with_item(&self, item: Item) -> (Self, bool) {
        let id = item.id().clone();
        let mut new_items = self.items.clone();
        let was_new = new_items.insert(id.clone(), item).is_none();
        
        let mut new_registry = Self {
            items: new_items,
        };
        
        // Update parent module's item list if this is a new item
        if was_new {
            new_registry.update_parent_module_items(&id);
        }
        
        (new_registry, was_new)
    }
    
    /// Create a new registry with an item removed (immutable operation)
    /// Returns the new registry and the removed item if it existed
    pub fn without_item<T: AsRef<ItemId>>(&self, id: T) -> (Self, Option<Item>) {
        let id_ref = id.as_ref();
        let mut new_items = self.items.clone();
        let removed = new_items.remove(id_ref);
        
        let mut new_registry = Self {
            items: new_items,
        };
        
        // Update parent module's item list if item was removed
        if removed.is_some() {
            new_registry.remove_from_parent_module_items(id_ref);
        }
        
        (new_registry, removed)
    }
    
    /// Create a new registry with a module added (immutable operation)
    /// Automatically handles parent reference consistency
    pub fn with_module(&self, mut module: Module) -> (Self, bool) {
        // Ensure parent reference is consistent with module path
        if module.parent.is_none() {
            // Try to infer parent from path if not explicitly set
            let module_path = &module.id.id.path;
            if let Some(last_colon_pos) = module_path.rfind("::") {
                if last_colon_pos > 2 { // More than just "::" at the start
                    let parent_path = &module_path[..last_colon_pos];
                    module.parent = Some(ModuleId::new(parent_path));
                }
            }
        }
        
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
    
    /// Remove a module and all its children using recursive hierarchy traversal
    pub fn remove_module_tree(&mut self, module_id: &ModuleId) -> Vec<Item> {
        let mut removed = Vec::new();
        self.remove_item_and_children_recursive(module_id, &mut removed);
        removed
    }
    
    /// Recursively remove an item and all its children from the hierarchy
    /// Used internally by remove_module_tree to handle both modules and their child items
    fn remove_item_and_children_recursive<T: AsRef<ItemId>>(&mut self, item_id: T, removed: &mut Vec<Item>) {
        let item_id = item_id.as_ref();
        // Get the item first (before removing it) to check if it's a module with children
        if let Some(item) = self.get_item(item_id) {
            // If it's a module, recursively remove its children first
            if let Item::Module(module) = item {
                let child_ids: Vec<_> = module.items.clone(); // Clone to avoid borrow issues
                for child_id in child_ids {
                    self.remove_item_and_children_recursive(&child_id, removed);
                }
            }
        }
        
        // Remove the item itself
        if let Some(item) = self.remove_item(item_id) {
            removed.push(item);
        }
    }
    
    /// Get the parent module ID for an item, using Module.parent field when available
    /// Falls back to path parsing for non-module items or during transition
    fn get_parent_module_id<T: AsRef<ItemId>>(&self, item_id: T) -> Option<ModuleId> {
        let item_id = item_id.as_ref();
        // If this is a module, check if it has a parent field
        if item_id.kind == ItemKind::Module {
            if let Some(Item::Module(module)) = self.get_item(item_id) {
                return module.parent.clone();
            }
        }
        
        // Fallback: parse path to find parent (for non-modules or during transition)
        let item_path = &item_id.path;
        if let Some(last_colon_pos) = item_path.rfind("::") {
            if last_colon_pos > 2 { // More than just "::" at the start
                let parent_path = &item_path[..last_colon_pos];
                return Some(ModuleId::new(parent_path));
            }
        }
        
        None
    }
    
    /// Update parent module's item list when a new item is added
    fn update_parent_module_items<T: AsRef<ItemId>>(&mut self, item_id: T) {
        let item_id = item_id.as_ref();
        // Use the unified parent resolution helper
        if let Some(parent_module_id) = self.get_parent_module_id(item_id) {
            // Check if parent module exists and update its items list
            if let Some(item) = self.items.get(parent_module_id.as_ref()) {
                if let Item::Module(module) = item {
                    if !module.items.contains(item_id) {
                        // Clone the module, update it, and replace it in the registry
                        let mut updated_module = module.clone();
                        updated_module.items.push(item_id.clone());
                        self.items.insert(parent_module_id.as_ref().clone(), Item::Module(updated_module));
                    }
                }
            }
        }
    }
    
    /// Remove item from parent module's item list when an item is removed
    fn remove_from_parent_module_items<T: AsRef<ItemId>>(&mut self, item_id: T) {
        let item_id = item_id.as_ref();
        // Use the unified parent resolution helper
        if let Some(parent_module_id) = self.get_parent_module_id(item_id) {
            // Check if parent module exists and remove from its items list
            if let Some(item) = self.items.get(parent_module_id.as_ref()) {
                if let Item::Module(module) = item {
                    if module.items.contains(item_id) {
                        // Clone the module, update it, and replace it in the registry
                        let mut updated_module = module.clone();
                        updated_module.items.retain(|id| id != item_id);
                        self.items.insert(parent_module_id.as_ref().clone(), Item::Module(updated_module));
                    }
                }
            }
        }
    }
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
            id: ModuleId::new("::test_module"),
            parent: None,
            items: vec![],
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
        assert_eq!(retrieved.id.id.path.as_ref(), "::test_module");
        
        // Remove the module
        let removed = registry.remove_item(&module.id.id).unwrap();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
        
        match removed {
            Item::Module(m) => assert_eq!(m.id.id.path.as_ref(), "::test_module"),
            _ => panic!("Expected module"),
        }
    }
    
    #[test]
    fn test_registry_different_item_types() {
        let mut registry = ItemRegistry::new();
        
        // Add a module
        let module = Module {
            id: ModuleId::new("::test"),
            parent: None,
            items: vec![],
            visibility: Visibility::Public,
        };
        registry.add_module(module);
        
        // Add a type with the same name (different namespace)
        let type_def = TypeDefinition {
            id: TypeId::new("::test"),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("test"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        };
        registry.add_type(type_def);
        
        // Add a predicate with the same name (different namespace)
        let predicate = Predicate {
            id: PredicateId::new("::test"),
            parameters: vec![],
            body: StructuralGoal::empty_container(),
            kind: PredicateKind::Relation,
            visibility: Visibility::Public,
        };
        registry.add_predicate(predicate);
        
        // All three should coexist
        assert_eq!(registry.len(), 3);
        
        // Check we can retrieve each by their specific type
        assert!(registry.get_module(&ModuleId::new("::test")).is_some());
        assert!(registry.get_type(&TypeId::new("::test")).is_some());
        assert!(registry.get_predicate(&PredicateId::new("::test")).is_some());
    }
    
    #[test]
    fn test_registry_module_hierarchy() {
        let mut registry = ItemRegistry::new();
        
        // Create parent module
        let parent = Module {
            id: ModuleId::new("::parent"),
            parent: None,
            items: vec![],
            visibility: Visibility::Public,
        };
        registry.add_module(parent);
        
        // Create child modules
        let child1 = Module {
            id: ModuleId::new("::parent::child1"),
            parent: Some(ModuleId::new("::parent")),
            items: vec![],
            visibility: Visibility::Public,
        };
        registry.add_module(child1);
        
        let child2 = Module {
            id: ModuleId::new("::parent::child2"),
            parent: Some(ModuleId::new("::parent")),
            items: vec![],
            visibility: Visibility::Public,
        };
        registry.add_module(child2);
        
        // Create grandchild
        let grandchild = Module {
            id: ModuleId::new("::parent::child1::grandchild"),
            parent: Some(ModuleId::new("::parent::child1")),
            items: vec![],
            visibility: Visibility::Public,
        };
        registry.add_module(grandchild);
        
        assert_eq!(registry.len(), 4);
        
        // Test child module lookup
        let children: Vec<_> = registry.child_modules("::parent").collect();
        assert_eq!(children.len(), 2);
        
        let child_paths: Vec<&str> = children.iter()
            .map(|m| m.id.id.path.as_ref())
            .collect();
        assert!(child_paths.contains(&"::parent::child1"));
        assert!(child_paths.contains(&"::parent::child2"));
        
        // Grandchild should not be included in direct children
        assert!(!child_paths.contains(&"::parent::child1::grandchild"));
    }
    
    #[test]
    fn test_registry_remove_module_tree() {
        let mut registry = ItemRegistry::new();
        
        // Build a module hierarchy with types and predicates
        registry.add_module(Module {
            id: ModuleId::new("::parent"),
            parent: None,
            items: vec![],
            visibility: Visibility::Public,
        });
        
        registry.add_module(Module {
            id: ModuleId::new("::parent::child"),
            parent: Some(ModuleId::new("::parent")),
            items: vec![],
            visibility: Visibility::Public,
        });
        
        registry.add_type(TypeDefinition {
            id: TypeId::new("::parent::child::MyType"),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("MyType"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        });
        
        registry.add_predicate(Predicate {
            id: PredicateId::new("::parent::child::my_pred"),
            parameters: vec![],
            body: StructuralGoal::empty_container(),
            kind: PredicateKind::Relation,
            visibility: Visibility::Public,
        });
        
        assert_eq!(registry.len(), 4);
        
        // Remove the parent module tree
        let parent_module_id = ModuleId::new("::parent");
        let removed = registry.remove_module_tree(&parent_module_id);
        
        // Should have removed parent, child, type, and predicate
        assert_eq!(removed.len(), 4);
        assert_eq!(registry.len(), 0);
    }
    
    #[test]
    fn test_items_by_kind() {
        let mut registry = ItemRegistry::new();
        
        // Add items of different kinds
        registry.add_module(Module {
            id: ModuleId::new("::mod1"),
            parent: None,
            items: vec![],
            visibility: Visibility::Public,
        });
        
        registry.add_module(Module {
            id: ModuleId::new("::mod2"),
            parent: None,
            items: vec![],
            visibility: Visibility::Public,
        });
        
        registry.add_type(TypeDefinition {
            id: TypeId::new("::Type1"),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("Type1"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        });
        
        registry.add_predicate(Predicate {
            id: PredicateId::new("::pred1"),
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
        let type_ref = TypeId::new("::TestType");
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
        let module_ref = ModuleId::new("::TestModule");
        let module = Module {
            id: module_ref.clone(),
            parent: None,
            items: vec![],
            visibility: Visibility::Public,
        };
        
        registry.add_module(module);
        
        assert!(registry.contains_item(&module_ref));
        assert!(registry.get_module(&module_ref).is_some());
        
        // Test with PredicateId
        let predicate_ref = PredicateId::new("::test_pred");
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
        assert_eq!(item_id.path.as_ref(), "::TestType");
        assert_eq!(item_id.kind, ItemKind::Type);
    }
    
    #[test]
    fn test_incremental_module_item_updates() {
        let mut registry = ItemRegistry::new();
        
        // Create a parent module
        let parent_module = Module {
            id: ModuleId::new("::parent"),
            parent: None,
            items: vec![],
            visibility: Visibility::Public,
        };
        registry.add_module(parent_module);
        
        // Add a child type to the parent module
        let child_type = TypeDefinition {
            id: TypeId::new("::parent::ChildType"),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("ChildType"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        };
        
        let child_type_id = child_type.id.id.clone();
        registry.add_type(child_type);
        
        // Check that parent module's items list was updated
        let _parent_id = ItemId::new("::parent", ItemKind::Module);
        let parent = registry.get_module(&ModuleId::new("::parent")).unwrap();
        assert!(parent.items.contains(&child_type_id));
        
        // Remove the child type
        registry.remove_item(&child_type_id);
        
        // Check that parent module's items list was updated
        let parent = registry.get_module(&ModuleId::new("::parent")).unwrap();
        assert!(!parent.items.contains(&child_type_id));
    }
    
    #[test]
    fn test_immutable_update_methods() {
        let registry = ItemRegistry::new();
        
        // Test immutable add with with_item
        let module = Module {
            id: ModuleId::new("::test_module"),
            parent: None,
            items: vec![],
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
            id: ModuleId::new("::parent"),
            parent: None,
            items: vec![],
            visibility: Visibility::Public,
        };
        registry.add_module(parent);
        
        // Add child module without explicit parent reference
        let child = Module {
            id: ModuleId::new("::parent::child"),
            parent: None, // Will be inferred
            items: vec![],
            visibility: Visibility::Public,
        };
        registry.add_module(child);
        
        // Verify parent reference was inferred
        let retrieved_child = registry.get_module(&ModuleId::new("::parent::child")).unwrap();
        assert_eq!(retrieved_child.parent, Some(ModuleId::new("::parent")));
        
        // Test with deeper nesting
        let grandchild = Module {
            id: ModuleId::new("::parent::child::grandchild"),
            parent: None, // Will be inferred
            items: vec![],
            visibility: Visibility::Public,
        };
        registry.add_module(grandchild);
        
        let retrieved_grandchild = registry.get_module(&ModuleId::new("::parent::child::grandchild")).unwrap();
        assert_eq!(retrieved_grandchild.parent, Some(ModuleId::new("::parent::child")));
        
        // Test root module (no parent)
        let root = Module {
            id: ModuleId::new("::root"),
            parent: None,
            items: vec![],
            visibility: Visibility::Public,
        };
        registry.add_module(root);
        
        let retrieved_root = registry.get_module(&ModuleId::new("::root")).unwrap();
        assert_eq!(retrieved_root.parent, None);
    }
    
    #[test]
    fn test_parent_reference_consistency() {
        let mut registry = ItemRegistry::new();
        
        // Create hierarchy with explicit parent references
        let parent = Module {
            id: ModuleId::new("::parent"),
            parent: None,
            items: vec![],
            visibility: Visibility::Public,
        };
        registry.add_module(parent);
        
        // Add child with explicit parent reference
        let child = Module {
            id: ModuleId::new("::parent::child"),
            parent: Some(ModuleId::new("::parent")),
            items: vec![],
            visibility: Visibility::Public,
        };
        registry.add_module(child);
        
        // Add child type - should be automatically added to parent's items
        let child_type = TypeDefinition {
            id: TypeId::new("::parent::child::ChildType"),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("ChildType"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        };
        registry.add_type(child_type.clone());
        
        // Verify the child module's items list was updated
        let retrieved_child = registry.get_module(&ModuleId::new("::parent::child")).unwrap();
        assert!(retrieved_child.items.contains(&child_type.id.id));
        
        // Verify the parent module's items list includes the child module
        let retrieved_parent = registry.get_module(&ModuleId::new("::parent")).unwrap();
        let child_module_id = ItemId::new("::parent::child", ItemKind::Module);
        assert!(retrieved_parent.items.contains(&child_module_id));
    }

    #[test]
    fn test_generic_remove_methods() {
        let mut registry = ItemRegistry::new();
        
        // Add items
        let module = Module {
            id: ModuleId::new("::test_module"),
            parent: None,
            items: vec![],
            visibility: Visibility::Public,
        };
        let module_id = module.id.clone();
        registry.add_module(module);
        
        let type_def = TypeDefinition {
            id: TypeId::new("::TestType"),
            kind: TypeKind::Struct(StructDefinition {
                name: InternedSymbol::from_text("TestType"),
                fields: StructFields::Tuple(vec![]),
            }),
            visibility: Visibility::Public,
        };
        let type_id = type_def.id.clone();
        registry.add_type(type_def);
        
        let predicate = Predicate {
            id: PredicateId::new("::test_pred"),
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
            id: ModuleId::new("::another_module"),
            parent: None,
            items: vec![],
            visibility: Visibility::Public,
        });
        
        let module_id = ModuleId::new("::another_module");
        let (final_registry, removed) = registry_with_module.without_item(&module_id);
        assert!(removed.is_some());
        assert_eq!(final_registry.len(), 0);
    }

    #[test]
    fn test_all_methods_generic_asref() {
        let mut registry = ItemRegistry::new();
        
        // Create items with different ID types
        let module_id = ModuleId::new("::test_module");
        let type_id = TypeId::new("::TestType");
        let predicate_id = PredicateId::new("::test_pred");
        
        let module = Module {
            id: module_id.clone(),
            parent: None,
            items: vec![],
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
            parameters: vec![
                Parameter {
                    name: InternedSymbol::from_text("x"),
                    type_annotation: None,
                },
            ],
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
            id: PredicateId::new("::test_pred"),
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
        let non_existent = PredicateId::new("::non_existent");
        assert_eq!(registry.get_predicate_arity(&non_existent), None);
    }
}