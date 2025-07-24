//! Memory-efficient symbol registry with reference counting and deduplication

use super::super::parser::ast::StructDefinition;
use super::super::runtime_value::RuntimeValue;
use super::types::*;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique identifier for symbols in the registry
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct SymbolId(u64);

impl SymbolId {
    fn next() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        SymbolId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Memory-efficient symbol reference that can be owned, shared, or weak
#[derive(Debug, Clone)]
pub enum SymbolRef<T> {
    /// Fully owned symbol (for unique symbols)
    Owned(T),
    /// Shared reference (for frequently used symbols)
    Shared(Rc<T>),
    /// Weak reference (for caching without ownership)
    Weak(Weak<T>),
}

impl<T: Clone> SymbolRef<T> {
    /// Get the value, upgrading weak references if possible
    pub fn get(&self) -> Option<T> {
        match self {
            SymbolRef::Owned(value) => Some(value.clone()),
            SymbolRef::Shared(rc) => Some((**rc).clone()),
            SymbolRef::Weak(weak) => weak.upgrade().map(|rc| (*rc).clone()),
        }
    }

    /// Convert to owned value if possible
    pub fn into_owned(self) -> Option<T> {
        match self {
            SymbolRef::Owned(value) => Some(value),
            SymbolRef::Shared(rc) => Some((*rc).clone()),
            SymbolRef::Weak(weak) => weak.upgrade().map(|rc| (*rc).clone()),
        }
    }

    /// Check if the reference is still valid
    pub fn is_valid(&self) -> bool {
        match self {
            SymbolRef::Owned(_) | SymbolRef::Shared(_) => true,
            SymbolRef::Weak(weak) => weak.strong_count() > 0,
        }
    }

    /// Upgrade to a shared reference if possible
    pub fn upgrade(&self) -> Option<SymbolRef<T>> {
        match self {
            SymbolRef::Owned(value) => Some(SymbolRef::Shared(Rc::new(value.clone()))),
            SymbolRef::Shared(rc) => Some(SymbolRef::Shared(rc.clone())),
            SymbolRef::Weak(weak) => weak.upgrade().map(SymbolRef::Shared),
        }
    }
}

/// Metadata about a symbol in the registry
#[derive(Debug, Clone)]
pub struct SymbolMetadata {
    pub id: SymbolId,
    pub name: String,
    pub module_path: ModulePath,
    pub symbol_type: SymbolType,
    pub is_public: bool,
}

/// Unified symbol registry for memory-efficient symbol management
pub struct SymbolRegistry {
    /// Canonical storage for runtime values with reference counting
    value_symbols: HashMap<SymbolId, Rc<RuntimeValue>>,

    /// Canonical storage for type definitions with reference counting
    type_symbols: HashMap<SymbolId, Rc<StructDefinition>>,

    /// Symbol metadata for all registered symbols (also reference counted)
    symbol_metadata: HashMap<SymbolId, Rc<SymbolMetadata>>,

    /// Reverse lookup: (module, symbol_name) -> symbol_id
    name_lookup: HashMap<(ModulePath, String), SymbolId>,

    /// Index by symbol type for efficient filtering
    type_index: HashMap<SymbolType, Vec<SymbolId>>,

    /// Index by module for efficient module-based queries
    module_index: HashMap<ModulePath, Vec<SymbolId>>,

    /// Weak references cache for performance
    weak_cache: HashMap<SymbolId, Weak<RuntimeValue>>,
    weak_type_cache: HashMap<SymbolId, Weak<StructDefinition>>,
}

impl SymbolRegistry {
    pub fn new() -> Self {
        Self {
            value_symbols: HashMap::new(),
            type_symbols: HashMap::new(),
            symbol_metadata: HashMap::new(),
            name_lookup: HashMap::new(),
            type_index: HashMap::new(),
            module_index: HashMap::new(),
            weak_cache: HashMap::new(),
            weak_type_cache: HashMap::new(),
        }
    }

    /// Register a runtime value symbol
    pub fn register_value(
        &mut self,
        name: String,
        module_path: ModulePath,
        value: RuntimeValue,
        is_public: bool,
    ) -> SymbolId {
        let symbol_id = SymbolId::next();

        // Determine symbol type from runtime value
        let symbol_type = match &value {
            RuntimeValue::Relation(_) => SymbolType::Relation,
            RuntimeValue::PredicateHandle(_) => SymbolType::PredicateHandle,
            RuntimeValue::BuiltinRelation { .. } => SymbolType::BuiltinRelation,
            RuntimeValue::Struct(_) => SymbolType::Struct,
            RuntimeValue::Type(_) => SymbolType::Type, // Registry types are their own symbol type
            RuntimeValue::Term(_) => SymbolType::Term,
        };

        // Store the value with reference counting
        let rc_value = Rc::new(value);
        self.value_symbols.insert(symbol_id, rc_value.clone());

        // Store metadata
        let metadata = SymbolMetadata {
            id: symbol_id,
            name: name.clone(),
            module_path: module_path.clone(),
            symbol_type: symbol_type.clone(),
            is_public,
        };
        self.symbol_metadata.insert(symbol_id, Rc::new(metadata));

        // Update indices
        self.name_lookup
            .insert((module_path.clone(), name), symbol_id);
        self.type_index
            .entry(symbol_type)
            .or_insert_with(Vec::new)
            .push(symbol_id);
        self.module_index
            .entry(module_path)
            .or_insert_with(Vec::new)
            .push(symbol_id);

        // Store weak reference for caching
        self.weak_cache.insert(symbol_id, Rc::downgrade(&rc_value));

        symbol_id
    }

    /// Register a type definition symbol
    pub fn register_type(
        &mut self,
        name: String,
        module_path: ModulePath,
        type_def: StructDefinition,
        is_public: bool,
    ) -> SymbolId {
        let symbol_id = SymbolId::next();

        // Store the type with reference counting
        let rc_type = Rc::new(type_def);
        self.type_symbols.insert(symbol_id, rc_type.clone());

        // Store metadata
        let metadata = SymbolMetadata {
            id: symbol_id,
            name: name.clone(),
            module_path: module_path.clone(),
            symbol_type: SymbolType::Type,
            is_public,
        };
        self.symbol_metadata.insert(symbol_id, Rc::new(metadata));

        // Update indices
        self.name_lookup
            .insert((module_path.clone(), name), symbol_id);
        self.type_index
            .entry(SymbolType::Type)
            .or_insert_with(Vec::new)
            .push(symbol_id);
        self.module_index
            .entry(module_path)
            .or_insert_with(Vec::new)
            .push(symbol_id);

        // Store weak reference for caching
        self.weak_type_cache
            .insert(symbol_id, Rc::downgrade(&rc_type));

        symbol_id
    }

    /// Get a symbol by ID as a shared reference
    pub fn get_value(&self, id: SymbolId) -> Option<SymbolRef<RuntimeValue>> {
        self.value_symbols
            .get(&id)
            .map(|rc| SymbolRef::Shared(rc.clone()))
    }

    /// Get a type by ID as a shared reference
    pub fn get_type(&self, id: SymbolId) -> Option<SymbolRef<StructDefinition>> {
        self.type_symbols
            .get(&id)
            .map(|rc| SymbolRef::Shared(rc.clone()))
    }

    /// Look up a symbol by name and module
    pub fn lookup_symbol(&self, module_path: &ModulePath, name: &str) -> Option<SymbolId> {
        self.name_lookup
            .get(&(module_path.clone(), name.to_string()))
            .copied()
    }

    /// Get all symbols in a module
    pub fn get_module_symbols(&self, module_path: &ModulePath) -> Vec<SymbolId> {
        self.module_index
            .get(module_path)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all symbols of a specific type
    pub fn get_symbols_by_type(&self, symbol_type: &SymbolType) -> Vec<SymbolId> {
        self.type_index
            .get(symbol_type)
            .cloned()
            .unwrap_or_default()
    }

    /// Get symbol metadata
    pub fn get_metadata(&self, id: SymbolId) -> Option<&SymbolMetadata> {
        self.symbol_metadata.get(&id).map(|rc| &**rc)
    }

    /// Get the actual reference count for a symbol's metadata
    /// This uses Rc::strong_count() which is always accurate
    pub fn get_metadata_ref_count(&self, id: SymbolId) -> Option<usize> {
        self.symbol_metadata.get(&id).map(|rc| Rc::strong_count(rc))
    }

    /// Get a cloned Rc to the metadata for sharing
    pub fn get_metadata_rc(&self, id: SymbolId) -> Option<Rc<SymbolMetadata>> {
        self.symbol_metadata.get(&id).cloned()
    }

    /// Create a weak reference to a symbol
    pub fn create_weak_ref(&self, id: SymbolId) -> Option<SymbolRef<RuntimeValue>> {
        self.weak_cache
            .get(&id)
            .map(|weak| SymbolRef::Weak(weak.clone()))
    }

    /// Create a weak reference to a type
    pub fn create_weak_type_ref(&self, id: SymbolId) -> Option<SymbolRef<StructDefinition>> {
        self.weak_type_cache
            .get(&id)
            .map(|weak| SymbolRef::Weak(weak.clone()))
    }

    /// Perform garbage collection on weak references
    pub fn garbage_collect(&mut self) -> RegistryStats {
        let mut collected_count = 0;

        // Collect dead weak references in value cache
        let mut dead_value_refs = Vec::new();
        for (&id, weak_ref) in &self.weak_cache {
            if weak_ref.strong_count() == 0 {
                dead_value_refs.push(id);
            }
        }

        // Collect dead weak references in type cache
        let mut dead_type_refs = Vec::new();
        for (&id, weak_ref) in &self.weak_type_cache {
            if weak_ref.strong_count() == 0 {
                dead_type_refs.push(id);
            }
        }

        // Clean up dead references
        for id in dead_value_refs {
            self.weak_cache.remove(&id);
            collected_count += 1;
        }

        for id in dead_type_refs {
            self.weak_type_cache.remove(&id);
            collected_count += 1;
        }

        RegistryStats {
            total_symbols: self.symbol_metadata.len(),
            value_symbols: self.value_symbols.len(),
            type_symbols: self.type_symbols.len(),
            weak_value_refs: self.weak_cache.len(),
            weak_type_refs: self.weak_type_cache.len(),
            collected_refs: collected_count,
        }
    }

    /// Get registry statistics
    pub fn get_stats(&self) -> RegistryStats {
        RegistryStats {
            total_symbols: self.symbol_metadata.len(),
            value_symbols: self.value_symbols.len(),
            type_symbols: self.type_symbols.len(),
            weak_value_refs: self.weak_cache.len(),
            weak_type_refs: self.weak_type_cache.len(),
            collected_refs: 0,
        }
    }

    /// Create an accessible symbols collection from a list of symbol IDs
    pub fn create_accessible_symbols(&self, symbol_ids: &[SymbolId]) -> AccessibleSymbols {
        let mut accessible = AccessibleSymbols::new();

        for &id in symbol_ids {
            if let Some(metadata) = self.get_metadata(id) {
                match metadata.symbol_type {
                    SymbolType::Type => {
                        if let Some(type_ref) = self.get_type(id) {
                            if let Some(type_def) = type_ref.get() {
                                accessible.types.insert(metadata.name.clone(), type_def);
                            }
                        }
                    }
                    _ => {
                        if let Some(value_ref) = self.get_value(id) {
                            if let Some(value) = value_ref.get() {
                                accessible.values.insert(metadata.name.clone(), value);
                            }
                        }
                    }
                }
            }
        }

        accessible
    }

    /// Find symbols by pattern (for debugging and introspection)
    pub fn find_symbols_by_pattern(
        &self,
        module_pattern: Option<&str>,
        name_pattern: Option<&str>,
    ) -> Vec<SymbolId> {
        let mut results = Vec::new();

        for (&id, metadata) in &self.symbol_metadata {
            let module_matches = module_pattern.map_or(true, |pattern| {
                metadata.module_path.to_string().contains(pattern)
            });

            let name_matches = name_pattern.map_or(true, |pattern| metadata.name.contains(pattern));

            if module_matches && name_matches {
                results.push(id);
            }
        }

        results
    }
}

/// Statistics about the symbol registry
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_symbols: usize,
    pub value_symbols: usize,
    pub type_symbols: usize,
    pub weak_value_refs: usize,
    pub weak_type_refs: usize,
    pub collected_refs: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::symbol_table::InternedSymbol;

    use crate::interpreter::parser::ast::{Conjunction, Goal as AstGoal, Location};
    use crate::interpreter::parser::ast::{PredicateDefinition, PredicateKind, Visibility};

    fn create_test_predicate() -> PredicateDefinition {
        PredicateDefinition {
            attributes: vec![],
            visibility: Visibility::Public,
            predicate_kind: PredicateKind::Relation,
            name: "test_predicate".to_string().into(),
            parameters: vec![],
            search_strategy: None,
            body: vec![AstGoal::Conjunction(
                Conjunction {
                    body: vec![],
                    params: None,
                },
                Location::dummy(),
            )],
            span: Location::dummy(),
        }
    }

    fn create_test_struct() -> StructDefinition {
        use crate::interpreter::parser::ast::StructKind;
        StructDefinition {
            visibility: Visibility::Public,
            name: "TestStruct".to_string().into(),
            kind: StructKind::Tuple(vec![InternedSymbol::from_text("i32")]),
            span: Location::dummy(),
        }
    }

    #[test]
    fn test_symbol_registration() {
        let mut registry = SymbolRegistry::new();
        let module_path = ModulePath::from_string("test_module");

        // Register a value symbol
        let predicate = create_test_predicate();
        let value = RuntimeValue::Relation(predicate);
        let symbol_id =
            registry.register_value("test_symbol".to_string(), module_path.clone(), value, true);

        // Verify it was registered correctly
        assert!(registry.get_value(symbol_id).is_some());
        assert!(registry
            .lookup_symbol(&module_path, "test_symbol")
            .is_some());

        let metadata = registry.get_metadata(symbol_id).unwrap();
        assert_eq!(metadata.name, "test_symbol");
        assert_eq!(metadata.module_path, module_path);
        assert_eq!(metadata.symbol_type, SymbolType::Relation);
        assert!(metadata.is_public);
    }

    #[test]
    fn test_type_registration() {
        let mut registry = SymbolRegistry::new();
        let module_path = ModulePath::from_string("test_module");

        // Register a type symbol
        let struct_def = create_test_struct();
        let symbol_id = registry.register_type(
            "TestStruct".to_string(),
            module_path.clone(),
            struct_def,
            true,
        );

        // Verify it was registered correctly
        assert!(registry.get_type(symbol_id).is_some());
        assert!(registry.lookup_symbol(&module_path, "TestStruct").is_some());

        let metadata = registry.get_metadata(symbol_id).unwrap();
        assert_eq!(metadata.symbol_type, SymbolType::Type);
    }

    #[test]
    fn test_symbol_reference_types() {
        let mut registry = SymbolRegistry::new();
        let module_path = ModulePath::from_string("test_module");

        let predicate = create_test_predicate();
        let value = RuntimeValue::Relation(predicate);
        let symbol_id =
            registry.register_value("test_symbol".to_string(), module_path, value, true);

        // Test shared reference
        let shared_ref = registry.get_value(symbol_id).unwrap();
        assert!(shared_ref.is_valid());
        assert!(shared_ref.get().is_some());

        // Test weak reference
        let weak_ref = registry.create_weak_ref(symbol_id).unwrap();
        assert!(weak_ref.is_valid());
        assert!(weak_ref.get().is_some());
    }

    #[test]
    fn test_module_symbol_lookup() {
        let mut registry = SymbolRegistry::new();
        let module_path = ModulePath::from_string("test_module");

        // Register multiple symbols in the same module
        let predicate1 = create_test_predicate();
        let predicate2 = create_test_predicate();

        registry.register_value(
            "symbol1".to_string(),
            module_path.clone(),
            RuntimeValue::Relation(predicate1),
            true,
        );

        registry.register_value(
            "symbol2".to_string(),
            module_path.clone(),
            RuntimeValue::Relation(predicate2),
            true,
        );

        // Get all symbols in the module
        let module_symbols = registry.get_module_symbols(&module_path);
        assert_eq!(module_symbols.len(), 2);
    }

    #[test]
    fn test_symbol_reference_management() {
        let mut registry = SymbolRegistry::new();
        let module_path = ModulePath::from_string("test_module");

        let predicate = create_test_predicate();
        let value = RuntimeValue::Relation(predicate);
        let symbol_id =
            registry.register_value("test_symbol".to_string(), module_path.clone(), value, true);

        // Symbol should exist and be accessible
        assert!(registry.get_value(symbol_id).is_some());
        assert!(registry
            .lookup_symbol(&module_path, "test_symbol")
            .is_some());

        // Metadata should be available and properly reference counted by Rc
        let metadata = registry
            .get_metadata(symbol_id)
            .expect("Metadata should exist");
        assert_eq!(metadata.name, "test_symbol");
        assert_eq!(metadata.module_path, module_path);
        assert!(metadata.is_public);

        // Test Rc-based reference counting (should start with 1 reference in the registry)
        assert_eq!(registry.get_metadata_ref_count(symbol_id), Some(1));

        // Clone the metadata Rc to increase reference count
        let metadata_clone = registry
            .get_metadata_rc(symbol_id)
            .expect("Should get metadata Rc");
        assert_eq!(registry.get_metadata_ref_count(symbol_id), Some(2));

        // Drop the clone, reference count should go back to 1
        drop(metadata_clone);
        assert_eq!(registry.get_metadata_ref_count(symbol_id), Some(1));
    }
}
