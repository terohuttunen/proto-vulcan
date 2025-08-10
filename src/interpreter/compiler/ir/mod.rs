//! Intermediate Representation (IR) for the Proto-Vulcan interpreter
//!
//! The IR is a compiled, immutable representation of a Proto-Vulcan program where:
//! - All symbols are resolved to stable ItemId references
//! - All qualified paths are resolved to concrete items
//! - All types are validated and checked
//! - Semantic ambiguities are eliminated
//!
//! The IR supports dynamic modification through Rc::make_mut while maintaining
//! path-based stable identifiers for efficient symbol resolution.

use crate::interpreter::constraint_domains::DomainConstraintTemplate;
use crate::interpreter::symbol_table::InternedSymbol;
use std::borrow::Borrow;
use std::fmt::{self, Display};
use std::rc::Rc;

pub mod display;
pub mod normalizer;
pub mod registry;
pub mod validation;

/// Structured module path using functional-style nested pairs
/// Hides internal Rc<str> implementation and ensures type safety
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModulePath {
    /// Current segment name (internal representation)
    head: Rc<str>,
    /// Rest of the path (recursive structure for efficient sharing)
    tail: Option<Rc<ModulePath>>,
}

impl ModulePath {
    /// Create root module path (empty)
    pub fn root() -> Self {
        ModulePath {
            head: "".into(),
            tail: None,
        }
    }

    /// Extend this path with a child segment (safe constructor)
    /// Used by IR registry to build validated paths incrementally
    pub fn extend(&self, child_name: impl Into<Rc<str>>) -> Self {
        let c = child_name.into();
        assert!(!c.is_empty());
        ModulePath {
            head: c,
            tail: Some(Rc::new(self.clone())),
        }
    }

    /// Check if this is the root path
    pub fn is_root(&self) -> bool {
        self.head.is_empty() && self.tail.is_none()
    }

    /// Get parent path (returns Rc for sharing)
    pub fn parent(&self) -> Option<Rc<ModulePath>> {
        self.tail.clone()
    }

    /// Get the current segment name
    pub fn head(&self) -> &str {
        &self.head
    }

    /// Convert this ModulePath to a ModuleId
    pub fn to_module_id(&self) -> ModuleId {
        match (&self.head, &self.tail) {
            // Root case: empty head, no tail -> create root module
            (head, None) if head.is_empty() => ModuleId::root(),
            // Normal case: head is module name, tail is parent path
            (head, tail) => {
                let parent_path = tail.clone().unwrap_or_else(|| Rc::new(ModulePath::root()));
                ModuleId::with_parent(parent_path, head.clone())
            }
        }
    }
}

impl Display for ModulePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_root() {
            Ok(()) // Don't print anything for root
        } else {
            // Build path by walking the structure
            let mut segments = Vec::new();
            let mut current = self;

            loop {
                segments.push(current.head.as_ref());
                if let Some(ref tail) = current.tail {
                    current = tail;
                } else {
                    break;
                }
            }

            segments.reverse();

            // Remove the root module (last segment after reversing becomes first)
            if !segments.is_empty() {
                segments.remove(0);
            }

            write!(f, "::{}", segments.join("::"))
        }
    }
}

/// Stable identifier for any item in the IR (Module, Type, or Predicate)
/// Uses structured path identification with separate namespaces by kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemId {
    /// Structured module path to the item (shared for efficiency)
    /// None for global items without parent module
    pub module_path: Option<Rc<ModulePath>>,
    /// Local name with kind for namespace separation
    pub name: ItemName,
}

impl ItemId {
    /// Create a new ItemId from module path and name
    pub fn new(module_path: Option<Rc<ModulePath>>, name: ItemName) -> Self {
        Self { module_path, name }
    }

    /// Helper: Create ItemId with parent path (for migration compatibility)
    pub fn with_parent(module_path: Rc<ModulePath>, name: ItemName) -> Self {
        Self::new(Some(module_path), name)
    }

    /// Create a global ItemId (no module prefix)
    pub fn global(name: &str, kind: ItemKind) -> Self {
        let item_name = ItemName::new(name, kind).expect("Invalid global item name");
        Self::new(None, item_name)
    }

    /// Split this ItemId into parent module path and local name
    /// Direct field access, no string parsing
    /// Returns (parent_module_path, local_name) where parent is None for root items
    pub fn split(&self) -> (Option<Rc<ModulePath>>, ItemName) {
        (self.module_path.clone(), self.name.clone())
    }

    /// Get the item kind
    pub fn kind(&self) -> ItemKind {
        self.name.kind
    }

    /// Get the ModuleId for this item's parent module (None if no parent)
    pub fn parent_module_id(&self) -> Option<ModuleId> {
        self.module_path.as_ref().map(|p| p.to_module_id())
    }
}

impl Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.module_path.is_none() {
            // Root case: just show ::name
            write!(f, "::{}", self.name)
        } else {
            // Non-root case: show full path
            write!(f, "{}::{}", self.module_path.as_ref().unwrap(), self.name)
        }
    }
}

impl AsRef<ItemId> for ItemId {
    fn as_ref(&self) -> &ItemId {
        self
    }
}

/// Kind of item for namespace separation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Module,
    Type,
    Predicate,
}

/// Local name identifier - single identifier only, no :: paths
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemName {
    /// Single identifier name (no :: allowed)
    pub name: Rc<str>,
    /// Item kind for namespace separation  
    pub kind: ItemKind,
}

impl ItemName {
    /// Create new ItemName with validation
    pub fn new(name: impl Into<Rc<str>>, kind: ItemKind) -> Result<Self, ItemNameError> {
        let name = name.into();

        // Validate: no path separators allowed
        if name.contains("::") {
            return Err(ItemNameError::ContainsPathSeparator(name));
        }

        // Validate: non-empty
        if name.is_empty() {
            return Err(ItemNameError::Empty);
        }

        Ok(Self { name, kind })
    }

    /// Create from validated string (for internal use)
    pub(crate) fn new_unchecked(name: impl Into<Rc<str>>, kind: ItemKind) -> Self {
        let name = name.into();
        debug_assert!(!name.contains("::"));
        Self { name, kind }
    }

    /// Get the simple name
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// Convenience constructor for predicates
    pub fn predicate(name: impl Into<Rc<str>>) -> Result<Self, ItemNameError> {
        Self::new(name, ItemKind::Predicate)
    }

    /// Convenience constructor for types
    pub fn type_name(name: impl Into<Rc<str>>) -> Result<Self, ItemNameError> {
        Self::new(name, ItemKind::Type)
    }

    /// Convenience constructor for modules
    pub fn module(name: impl Into<Rc<str>>) -> Result<Self, ItemNameError> {
        Self::new(name, ItemKind::Module)
    }
}

impl AsRef<ItemName> for ItemName {
    fn as_ref(&self) -> &ItemName {
        self
    }
}

impl Into<Rc<str>> for ItemName {
    fn into(self) -> Rc<str> {
        self.name
    }
}

impl Into<Rc<str>> for &ItemName {
    fn into(self) -> Rc<str> {
        self.name.clone()
    }
}

impl Display for ItemName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemNameError {
    ContainsPathSeparator(Rc<str>),
    Empty,
}

/// Identifier for a type in the registry
///
/// Implements both `Borrow<ItemId>` for efficient HashMap lookups and
/// `AsRef<ItemId>` for convenient API access. This allows both direct
/// usage with HashMap operations and flexible method calls.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeId {
    /// ItemId for this type
    pub id: ItemId,
}

impl TypeId {
    pub fn new(parent_path: Option<Rc<ModulePath>>, name: impl Into<Rc<str>>) -> Self {
        let item_name = ItemName::new(name, ItemKind::Type).expect("Invalid type name");
        Self {
            id: ItemId::new(parent_path, item_name),
        }
    }

    /// Helper: Create TypeId with parent path (for migration compatibility)
    pub fn with_parent(parent_path: Rc<ModulePath>, name: impl Into<Rc<str>>) -> Self {
        Self::new(Some(parent_path), name)
    }

    /// Convert to ItemId (cleaner than .as_ref().clone())
    pub fn to_item_id(&self) -> ItemId {
        self.id.clone()
    }

    pub fn name(&self) -> &ItemName {
        &self.id.name
    }

    pub fn parent_module_id(&self) -> Option<ModuleId> {
        self.id.module_path.as_ref().map(|path| path.to_module_id())
    }

    /// Split this TypeId into parent module path and type name
    pub fn split(&self) -> (Option<Rc<ModulePath>>, TypeName) {
        let (parent_module_path, item_name) = self.id.split();
        let type_name = TypeName { name: item_name };
        (parent_module_path, type_name)
    }

    /// Test helper: Create TypeId from path string (for testing only)
    #[cfg(test)]
    pub fn from_path(path: &str) -> Self {
        let path = path.strip_prefix("::").unwrap_or(path);
        let segments: Vec<&str> = if path.is_empty() {
            vec![]
        } else {
            path.split("::").collect()
        };

        if segments.is_empty() {
            panic!("Invalid type path: {}", path);
        }

        let name = segments.last().unwrap().to_string();
        let parent_segments = &segments[..segments.len().saturating_sub(1)];

        let mut current_path = Rc::new(ModulePath::root());
        for segment in parent_segments {
            current_path = current_path.extend(*segment).into();
        }

        Self::new(Some(current_path), name)
    }
}

impl Borrow<ItemId> for TypeId {
    fn borrow(&self) -> &ItemId {
        &self.id
    }
}

impl AsRef<ItemId> for TypeId {
    fn as_ref(&self) -> &ItemId {
        &self.id
    }
}

impl Into<ItemId> for TypeId {
    fn into(self) -> ItemId {
        self.id
    }
}

impl Into<ItemId> for &TypeId {
    fn into(self) -> ItemId {
        self.id.clone()
    }
}

/// Identifier for a predicate in the registry
///
/// Implements both `Borrow<ItemId>` for efficient HashMap lookups and
/// `AsRef<ItemId>` for convenient API access. This allows both direct
/// usage with HashMap operations and flexible method calls.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PredicateId {
    /// ItemId for this predicate
    pub id: ItemId,
}

impl PredicateId {
    pub fn new(parent_path: Option<Rc<ModulePath>>, name: impl Into<Rc<str>>) -> Self {
        let item_name = ItemName::new(name, ItemKind::Predicate).expect("Invalid predicate name");
        Self {
            id: ItemId::new(parent_path, item_name),
        }
    }

    /// Helper: Create PredicateId with parent path (for migration compatibility)
    pub fn with_parent(parent_path: Rc<ModulePath>, name: impl Into<Rc<str>>) -> Self {
        Self::new(Some(parent_path), name)
    }

    /// Convert to ItemId (cleaner than .as_ref().clone())
    pub fn to_item_id(&self) -> ItemId {
        self.id.clone()
    }

    pub fn name(&self) -> &ItemName {
        &self.id.name
    }

    pub fn parent_module_id(&self) -> Option<ModuleId> {
        self.id.module_path.as_ref().map(|path| path.to_module_id())
    }

    /// Split this PredicateId into parent module path and predicate name
    pub fn split(&self) -> (Option<Rc<ModulePath>>, PredicateName) {
        let (parent_module_path, item_name) = self.id.split();
        let predicate_name = PredicateName { name: item_name };
        (parent_module_path, predicate_name)
    }

    /// Test helper: Create PredicateId from path string (for testing only)
    #[cfg(test)]
    pub fn from_path(path: &str) -> Self {
        let path = path.strip_prefix("::").unwrap_or(path);
        let segments: Vec<&str> = if path.is_empty() {
            vec![]
        } else {
            path.split("::").collect()
        };

        if segments.is_empty() {
            panic!("Invalid predicate path: {}", path);
        }

        let name = segments.last().unwrap().to_string();
        let parent_segments = &segments[..segments.len().saturating_sub(1)];

        let mut current_path = Rc::new(ModulePath::root());
        for segment in parent_segments {
            current_path = current_path.extend(*segment).into();
        }

        Self::new(Some(current_path), name)
    }
}

impl Borrow<ItemId> for PredicateId {
    fn borrow(&self) -> &ItemId {
        &self.id
    }
}

impl AsRef<ItemId> for PredicateId {
    fn as_ref(&self) -> &ItemId {
        &self.id
    }
}

impl Into<ItemId> for PredicateId {
    fn into(self) -> ItemId {
        self.id
    }
}

impl Into<ItemId> for &PredicateId {
    fn into(self) -> ItemId {
        self.id.clone()
    }
}

/// Identifier for a module in the registry
///
/// Implements both `Borrow<ItemId>` for efficient HashMap lookups and
/// `AsRef<ItemId>` for convenient API access. This allows both direct
/// usage with HashMap operations and flexible method calls.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleId {
    /// ItemId for this module
    pub id: ItemId,
}

impl ModuleId {
    pub fn new(parent_path: Option<Rc<ModulePath>>, name: impl Into<Rc<str>>) -> Self {
        let item_name = ItemName::new(name, ItemKind::Module).expect("Invalid module name");
        Self {
            id: ItemId::new(parent_path, item_name),
        }
    }

    /// Helper: Create ModuleId with parent path (for migration compatibility)
    pub fn with_parent(parent_path: Rc<ModulePath>, name: impl Into<Rc<str>>) -> Self {
        Self::new(Some(parent_path), name)
    }

    pub fn root() -> Self {
        // Special case for root module - no parent path
        let root_name = ItemName::new_unchecked("", ItemKind::Module);
        Self {
            id: ItemId::new(None, root_name),
        }
    }

    /// Create a global module with a specific name (for crates)
    /// These modules have no parent (module_path = None) but have a name
    pub fn root_with_name(name: impl Into<Rc<str>>) -> Self {
        let item_name = ItemName::new(name, ItemKind::Module).expect("Invalid crate name");
        Self {
            id: ItemId::new(None, item_name),
        }
    }

    /// Get the parent module path for creating child items
    pub fn parent_path(&self) -> Option<Rc<ModulePath>> {
        self.id.module_path.clone()
    }

    /// Convert this ModuleId to a full ModulePath that includes this module's name
    pub fn full_path(&self) -> Rc<ModulePath> {
        match &self.id.module_path {
            None => {
                // Root module case: create path with just the module name
                // For root module with empty name, return root path
                if self.id.name.name.is_empty() {
                    Rc::new(ModulePath::root())
                } else {
                    Rc::new(ModulePath::root().extend(self.id.name.name.clone()))
                }
            }
            Some(parent_path) => {
                // Normal case: extend parent path with this module's name
                Rc::new(parent_path.extend(self.id.name.name.clone()))
            }
        }
    }

    /// Convert to ItemId (cleaner than .as_ref().clone())
    pub fn to_item_id(&self) -> ItemId {
        self.id.clone()
    }

    pub fn name(&self) -> &ItemName {
        &self.id.name
    }

    pub fn parent_module_id(&self) -> Option<ModuleId> {
        self.id.module_path.as_ref().map(|path| path.to_module_id())
    }

    /// Test helper: Create ModuleId from path string (for testing only)
    #[cfg(test)]
    pub fn from_path(path: &str) -> Self {
        if path == "::" || path.is_empty() {
            return Self::root();
        }

        let path = path.strip_prefix("::").unwrap_or(path);
        let segments: Vec<&str> = if path.is_empty() {
            vec![]
        } else {
            path.split("::").collect()
        };

        if segments.is_empty() {
            return Self::root();
        }

        let name = segments.last().unwrap().to_string();
        let parent_segments = &segments[..segments.len().saturating_sub(1)];

        let mut current_path = Rc::new(ModulePath::root());
        for segment in parent_segments {
            current_path = current_path.extend(*segment).into();
        }

        Self::new(Some(current_path), name)
    }
}

impl Borrow<ItemId> for ModuleId {
    fn borrow(&self) -> &ItemId {
        &self.id
    }
}

impl AsRef<ItemId> for ModuleId {
    fn as_ref(&self) -> &ItemId {
        &self.id
    }
}

impl Into<ItemId> for ModuleId {
    fn into(self) -> ItemId {
        self.id
    }
}

impl Into<ItemId> for &ModuleId {
    fn into(self) -> ItemId {
        self.id.clone()
    }
}

// TODO: Fix lifetime issue - temporarily commented out
// impl Borrow<str> for ModuleId {
//     fn borrow(&self) -> &str {
//         self.id.to_string().as_ref()
//     }
// }

/// Identifier for a predicate name in local scope
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PredicateName {
    pub name: ItemName,
}

impl PredicateName {
    pub fn new(name: impl Into<Rc<str>>) -> Result<Self, ItemNameError> {
        Ok(Self {
            name: ItemName::new(name, ItemKind::Predicate)?,
        })
    }

    pub fn to_item_name(&self) -> ItemName {
        self.name.clone()
    }
}

impl AsRef<ItemName> for PredicateName {
    fn as_ref(&self) -> &ItemName {
        &self.name
    }
}

impl Borrow<ItemName> for PredicateName {
    fn borrow(&self) -> &ItemName {
        &self.name
    }
}

/// Identifier for a type name in local scope
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeName {
    pub name: ItemName,
}

impl TypeName {
    pub fn new(name: impl Into<Rc<str>>) -> Result<Self, ItemNameError> {
        Ok(Self {
            name: ItemName::new(name, ItemKind::Type)?,
        })
    }

    pub fn to_item_name(&self) -> ItemName {
        self.name.clone()
    }
}

impl AsRef<ItemName> for TypeName {
    fn as_ref(&self) -> &ItemName {
        &self.name
    }
}

impl Borrow<ItemName> for TypeName {
    fn borrow(&self) -> &ItemName {
        &self.name
    }
}

/// Identifier for a module name in local scope
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleName {
    pub name: ItemName,
}

impl ModuleName {
    pub fn new(name: impl Into<Rc<str>>) -> Result<Self, ItemNameError> {
        Ok(Self {
            name: ItemName::new(name, ItemKind::Module)?,
        })
    }

    pub fn to_item_name(&self) -> ItemName {
        self.name.clone()
    }
}

impl AsRef<ItemName> for ModuleName {
    fn as_ref(&self) -> &ItemName {
        &self.name
    }
}

impl Borrow<ItemName> for ModuleName {
    fn borrow(&self) -> &ItemName {
        &self.name
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Alias {
    pub id: ItemId,
    pub target: ItemId,
}

/// Any item in the IR - unified storage for all item types
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Module(Module),
    Type(TypeDefinition),
    Predicate(Predicate),
    Alias(Alias),
}

impl Item {
    /// Get the ItemId for this item
    pub fn id(&self) -> &ItemId {
        match self {
            Item::Module(m) => &m.id.id,
            Item::Type(t) => &t.id.id,
            Item::Predicate(p) => &p.id.id,
            Item::Alias(i) => &i.id,
        }
    }

    /// Get the name of this item (local name only)
    pub fn name(&self) -> &str {
        &self.id().name.name
    }
}

/// A compiled Proto-Vulcan program with all symbols resolved
#[derive(Debug, Clone)]
pub struct Program {
    /// Unified registry of all items
    pub registry: Rc<ItemRegistry>,
    /// Symbol table for interned identifiers
    pub symbol_table: Rc<crate::interpreter::symbol_table::SymbolTable>,
}

impl Program {
    /// Create a new empty program with root module
    pub fn new() -> Self {
        let mut registry = ItemRegistry::new();

        // Always create the root module
        let root_module = Module {
            id: ModuleId::root(),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility: Visibility::Public,
        };

        // Add root module to registry (this should never fail since it's the first item)
        registry.add_module(root_module);

        Self {
            registry: Rc::new(registry),
            symbol_table: Rc::new(crate::interpreter::symbol_table::SymbolTable::new()),
        }
    }

    pub fn registry(&self) -> &ItemRegistry {
        &self.registry
    }

    /// Get a mutable reference to the registry using copy-on-write
    pub fn registry_mut(&mut self) -> &mut ItemRegistry {
        Rc::make_mut(&mut self.registry)
    }

    /// Get the root module from the registry (always exists)
    pub fn get_root_module(&self) -> &Module {
        let root_module_id = ModuleId::root();
        self.registry
            .get_module(&root_module_id)
            .expect("Root module should always exist in program")
    }

    /// Get the root module's ModulePath from the registry (always exists)
    pub fn get_root_module_path(&self) -> Rc<ModulePath> {
        // Root module has module_path = None, so return the root path
        match &self.get_root_module().id.id.module_path {
            None => Rc::new(ModulePath::root()),
            Some(path) => path.clone(),
        }
    }

    /*
    /// Resolve symbol in specific module
    pub fn resolve_symbol_in_module(
        &self,
        module_id: &ModuleId,
        name: &str,
        kind: ItemKind,
    ) -> Option<&ItemId> {
        let module = self.registry.get_module(module_id)?;
        let item_name = ItemName::new(name, kind).ok()?;
        module.items.get(&item_name)
    }

    /// Resolve public symbol in specific module
    pub fn resolve_public_symbol_in_module(
        &self,
        module_id: &ModuleId,
        name: &str,
        kind: ItemKind,
    ) -> Option<&ItemId> {
        let item_id = self.resolve_symbol_in_module(module_id, name, kind)?;

        // Check visibility using registry
        match kind {
            ItemKind::Predicate => {
                let predicate = self.registry.get_predicate(item_id)?;
                (predicate.visibility == Visibility::Public).then_some(item_id)
            }
            ItemKind::Type => {
                let type_def = self.registry.get_type(item_id)?;
                (type_def.visibility == Visibility::Public).then_some(item_id)
            }
            ItemKind::Module => {
                let module = self.registry.get_module(item_id)?;
                (module.visibility == Visibility::Public).then_some(item_id)
            }
        }
    }

    /// Get all items in a module
    pub fn items_in_module(&self, module_id: &ModuleId) -> Option<impl Iterator<Item = &ItemId>> {
        let module = self.registry.get_module(module_id)?;
        Some(module.items.values())
    }

    /// Get all public items in a module
    pub fn public_items_in_module(&self, module_id: &ModuleId) -> Option<Vec<&ItemId>> {
        let module = self.registry.get_module(module_id)?;
        let public_items: Vec<&ItemId> = module
            .items
            .iter()
            .filter(|(_, item_id)| self.is_item_public(item_id))
            .map(|(_, item_id)| item_id)
            .collect();
        Some(public_items)
    }

    /// Get predicates in a module
    pub fn predicates_in_module(
        &self,
        module_id: &ModuleId,
    ) -> Option<impl Iterator<Item = &ItemId>> {
        let module = self.registry.get_module(module_id)?;
        Some(
            module
                .items
                .iter()
                .filter(|(name, _)| name.kind == ItemKind::Predicate)
                .map(|(_, item_id)| item_id),
        )
    }

    /// Get public predicates in a module
    pub fn public_predicates_in_module(&self, module_id: &ModuleId) -> Option<Vec<&ItemId>> {
        let module = self.registry.get_module(module_id)?;
        let public_predicates: Vec<&ItemId> = module
            .items
            .iter()
            .filter(|(name, item_id)| {
                name.kind == ItemKind::Predicate && self.is_item_public(item_id)
            })
            .map(|(_, item_id)| item_id)
            .collect();
        Some(public_predicates)
    }

    /// Get types in a module
    pub fn types_in_module(&self, module_id: &ModuleId) -> Option<impl Iterator<Item = &ItemId>> {
        let module = self.registry.get_module(module_id)?;
        Some(
            module
                .items
                .iter()
                .filter(|(name, _)| name.kind == ItemKind::Type)
                .map(|(_, item_id)| item_id),
        )
    }

    /// Get public types in a module
    pub fn public_types_in_module(&self, module_id: &ModuleId) -> Option<Vec<&ItemId>> {
        let module = self.registry.get_module(module_id)?;
        let public_types: Vec<&ItemId> = module
            .items
            .iter()
            .filter(|(name, item_id)| name.kind == ItemKind::Type && self.is_item_public(item_id))
            .map(|(_, item_id)| item_id)
            .collect();
        Some(public_types)
    }

    /// Get submodules in a module
    pub fn modules_in_module(&self, module_id: &ModuleId) -> Option<impl Iterator<Item = &ItemId>> {
        let module = self.registry.get_module(module_id)?;
        Some(
            module
                .items
                .iter()
                .filter(|(name, _)| name.kind == ItemKind::Module)
                .map(|(_, item_id)| item_id),
        )
    }

    /// Get public submodules in a module
    pub fn public_modules_in_module(&self, module_id: &ModuleId) -> Option<Vec<&ItemId>> {
        let module = self.registry.get_module(module_id)?;
        let public_modules: Vec<&ItemId> = module
            .items
            .iter()
            .filter(|(name, item_id)| name.kind == ItemKind::Module && self.is_item_public(item_id))
            .map(|(_, item_id)| item_id)
            .collect();
        Some(public_modules)
    }

    /// Check if an item is public by looking it up in the registry
    fn is_item_public(&self, item_id: &ItemId) -> bool {
        match item_id.kind() {
            ItemKind::Predicate => self
                .registry
                .get_predicate(item_id)
                .map(|p| p.visibility == Visibility::Public)
                .unwrap_or(false),
            ItemKind::Type => self
                .registry
                .get_type(item_id)
                .map(|t| t.visibility == Visibility::Public)
                .unwrap_or(false),
            ItemKind::Module => self
                .registry
                .get_module(item_id)
                .map(|m| m.visibility == Visibility::Public)
                .unwrap_or(false),
        }
    }
    */
}

/// Module definition
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// Module identifier
    pub id: ModuleId,
    // Maps local names to global ItemIds for all items in this module
    //pub items: ImRcHashMap<ItemName, ItemId>,
    // Imported items from other modules
    //pub imports: ImRcHashMap<ItemName, ItemId>,
    /// Module visibility
    pub visibility: Visibility,
}

/// Type definition (struct or enum)
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDefinition {
    /// Type identifier
    pub id: TypeId,
    /// Type kind (struct or enum)
    pub kind: TypeKind,
    /// Type visibility
    pub visibility: Visibility,
}

/// Predicate definition
#[derive(Debug, Clone, PartialEq)]
pub struct Predicate {
    /// Predicate identifier
    pub id: PredicateId,
    /// Parameter list
    pub parameters: Vec<Parameter>,
    /// Body goals
    pub body: Rc<[StructuralGoal]>,
    /// Predicate kind (relation vs macro)
    pub kind: PredicateKind,
    /// Visibility
    pub visibility: Visibility,
}

/// Parameter in a predicate definition
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// Parameter name
    pub name: InternedSymbol,
    /// Type annotation (if present)
    pub type_annotation: Option<TypeAnnotation>,
}

/// Type annotation for parameters
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    /// Integer type for meta programming (non-relational)
    Int,
    /// String type for meta programming (non-relational)
    String,
    /// Boolean type for meta programming (non-relational)
    Bool,
    /// Relational integer type (Int)
    RelInt,
    /// Relational string type (String)
    RelString,
    /// Relational boolean type (Bool)
    RelBool,
    /// Relational character type (Char)
    RelChar,
    /// Base logic term type (LTerm)
    LTerm,
    /// Relation type with arity (rel(n))
    Relation(usize),
    /// Custom type reference
    Custom(TypeId),
}

/// Predicate kind
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PredicateKind {
    /// Regular relation
    Relation,
    /// Macro predicate for template expansion
    Macro,
    /// Grammar rule (DCG)
    Grammar,
}

/// Visibility levels
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Private,
    Public,
    Crate,
    Super,
    SelfModule,
    Restricted(ItemId),
}

/// Type definition kinds
#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Struct(StructDefinition),
    Enum(EnumDefinition),
}

/// Built-in primitive types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BuiltinType {
    Bool,
    Number,
    Char,
    String,
    Relation(usize), // rel(n) where n is the arity
    LTerm,           // Base type that all other types can be converted to
}

impl BuiltinType {
    /// Parse a built-in type from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Bool" => Some(BuiltinType::Bool),
            "Number" => Some(BuiltinType::Number),
            "Char" => Some(BuiltinType::Char),
            "String" => Some(BuiltinType::String),
            "LTerm" => Some(BuiltinType::LTerm),
            _ => {
                // Check for rel(n) pattern
                if s.starts_with("rel(") && s.ends_with(")") {
                    let arity_str = &s[4..s.len() - 1];
                    if let Ok(arity) = arity_str.parse::<usize>() {
                        return Some(BuiltinType::Relation(arity));
                    }
                }
                None
            }
        }
    }

    /// Get the string representation of a built-in type
    pub fn as_str(&self) -> String {
        match self {
            BuiltinType::Bool => "Bool".to_string(),
            BuiltinType::Number => "Number".to_string(),
            BuiltinType::Char => "Char".to_string(),
            BuiltinType::String => "String".to_string(),
            BuiltinType::Relation(arity) => format!("rel({})", arity),
            BuiltinType::LTerm => "LTerm".to_string(),
        }
    }
}

impl Display for BuiltinType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Type reference that can be either a built-in type or a user-defined type
#[derive(Debug, Clone, PartialEq)]
pub enum TypeReference {
    Builtin(BuiltinType),
    UserDefined(TypeId),
}

impl Display for TypeReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeReference::Builtin(builtin) => write!(f, "{}", builtin),
            TypeReference::UserDefined(type_id) => write!(f, "{}", type_id),
        }
    }
}

/// Struct definition
#[derive(Debug, Clone, PartialEq)]
pub struct StructDefinition {
    pub name: InternedSymbol,
    pub fields: StructFields,
}

/// Struct field types
#[derive(Debug, Clone, PartialEq)]
pub enum StructFields {
    Named(Vec<NamedField>),
    Tuple(Vec<TypeReference>),
}

/// Named field in a struct
#[derive(Debug, Clone, PartialEq)]
pub struct NamedField {
    pub name: InternedSymbol,
    pub type_ref: TypeReference,
    pub visibility: Visibility,
}

/// Enum definition
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDefinition {
    pub name: InternedSymbol,
    pub variants: Vec<EnumVariant>,
}

/// Enum variant
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: InternedSymbol,
    pub kind: EnumVariantKind,
}

/// Enum variant kinds
#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantKind {
    Unit,
    Tuple(Vec<TypeReference>),
    Named(Vec<NamedField>),
}

/// A resolved goal in the IR
#[derive(Debug, Clone, PartialEq)]
pub enum Goal {
    /// Equality goal: term == term
    Equality(Term, Term),
    /// Disequality goal: term != term
    Disequality(Term, Term),
    /// Predicate call with resolved reference
    PredicateCall(PredicateCall),
    /// Conjunction of goals
    Conjunction(Rc<[StructuralGoal]>),
    /// Disjunction of goals
    Disjunction(Rc<[StructuralGoal]>),
    /// Pattern matching
    PatternMatch(PatternMatch),
    /// Fresh variable introduction
    Fresh(Fresh),
    /// Let binding
    Let(Let),
    /// Boolean literal
    Boolean(bool),
    /// Constraint block
    Constraint(ConstraintBlock),
    /// Meta let statement for compile-time variable binding
    MetaLet(MetaLet),
    /// Meta if statement for conditional compilation
    MetaIf(MetaIf),
    /// Meta for statement for iterative expansion
    MetaFor(MetaFor),
}

/// A Goal enhanced with precomputed content and structural hashes
/// for efficient comparison and structural analysis operations.
#[derive(Debug, Clone)]
pub struct HashedGoal {
    goal: Goal,
    content_hash: u64,    // Exact goal structure hash (including variable names)
    structural_hash: u64, // Variable-normalized structure hash (for alpha-equivalence)
}

impl HashedGoal {
    /// Create a new HashedGoal with computed hashes
    pub fn new(goal: Goal) -> Self {
        let content_hash = compute_content_hash(&goal);
        let structural_hash = compute_structural_hash(&goal);
        HashedGoal {
            goal,
            content_hash,
            structural_hash,
        }
    }

    /// Get the content hash (exact structure including variable names)
    pub fn content_hash(goal: &HashedGoal) -> u64 {
        goal.content_hash
    }

    /// Get the structural hash (variable-normalized for alpha-equivalence)
    pub fn structural_hash(goal: &HashedGoal) -> u64 {
        goal.structural_hash
    }

    /// Check if two goals have the same content (exact equality)
    pub fn same_content(a: &HashedGoal, b: &HashedGoal) -> bool {
        a.content_hash == b.content_hash
    }

    /// Check if two goals have the same structure (alpha-equivalent)
    pub fn same_structure(a: &HashedGoal, b: &HashedGoal) -> bool {
        a.structural_hash == b.structural_hash
    }
}

impl std::ops::Deref for HashedGoal {
    type Target = Goal;
    fn deref(&self) -> &Self::Target {
        &self.goal
    }
}

impl PartialEq for HashedGoal {
    fn eq(&self, other: &Self) -> bool {
        // Use content hash for exact equality (O(1) instead of deep comparison)
        self.content_hash == other.content_hash
    }
}

impl Eq for HashedGoal {}

impl std::hash::Hash for HashedGoal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Use precomputed content hash
        self.content_hash.hash(state);
    }
}

/// A handle to a hashed goal that provides structural analysis capabilities
/// while hiding the Rc complexity and providing natural goal access via Deref.
#[derive(Debug, Clone)]
pub struct StructuralGoal {
    inner: Rc<HashedGoal>,
}

impl StructuralGoal {
    /// Create a new StructuralGoal from a Goal
    pub fn new(goal: Goal) -> Self {
        StructuralGoal {
            inner: Rc::new(HashedGoal::new(goal)),
        }
    }

    /// Get the content hash (exact structure including variable names)
    pub fn content_hash(goal: &StructuralGoal) -> u64 {
        HashedGoal::content_hash(&goal.inner)
    }

    /// Get the structural hash (variable-normalized for alpha-equivalence)
    pub fn structural_hash(goal: &StructuralGoal) -> u64 {
        HashedGoal::structural_hash(&goal.inner)
    }

    /// Check if two goals have the same content (exact equality)
    pub fn same_content(a: &StructuralGoal, b: &StructuralGoal) -> bool {
        HashedGoal::same_content(&a.inner, &b.inner)
    }

    /// Check if two goals have the same structure (alpha-equivalent)
    pub fn same_structure(a: &StructuralGoal, b: &StructuralGoal) -> bool {
        HashedGoal::same_structure(&a.inner, &b.inner)
    }
}

impl std::ops::Deref for StructuralGoal {
    type Target = Goal;
    fn deref(&self) -> &Self::Target {
        &self.inner.goal // Double deref: StructuralGoal -> HashedGoal -> Goal
    }
}

impl PartialEq for StructuralGoal {
    fn eq(&self, other: &Self) -> bool {
        // Use content hash for exact equality semantics
        self.inner.content_hash == other.inner.content_hash
    }
}

impl Eq for StructuralGoal {}

impl std::hash::Hash for StructuralGoal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Use content hash to match PartialEq semantics
        self.inner.content_hash.hash(state);
    }
}

impl PartialOrd for StructuralGoal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Fast ordering by content hash for deterministic ordering
        Some(self.inner.content_hash.cmp(&other.inner.content_hash))
    }
}

impl Ord for StructuralGoal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.content_hash.cmp(&other.inner.content_hash)
    }
}

/// A TypeDefinition enhanced with precomputed content and structural hashes
/// for efficient comparison and structural analysis operations.
#[derive(Debug, Clone)]
pub struct HashedType {
    type_def: TypeDefinition,
    content_hash: u64,    // Exact type structure hash (including field names)
    structural_hash: u64, // Field-normalized structure hash (for alpha-equivalence)
}

impl HashedType {
    /// Create a new HashedType with computed hashes
    pub fn new(type_def: TypeDefinition) -> Self {
        let content_hash = compute_type_content_hash(&type_def);
        let structural_hash = compute_type_structural_hash(&type_def);
        HashedType {
            type_def,
            content_hash,
            structural_hash,
        }
    }

    /// Get the content hash (exact structure including field names)
    pub fn content_hash(type_def: &HashedType) -> u64 {
        type_def.content_hash
    }

    /// Get the structural hash (field-normalized for alpha-equivalence)
    pub fn structural_hash(type_def: &HashedType) -> u64 {
        type_def.structural_hash
    }

    /// Check if two types have the same content (exact equality)
    pub fn same_content(a: &HashedType, b: &HashedType) -> bool {
        a.content_hash == b.content_hash
    }

    /// Check if two types have the same structure (alpha-equivalent)
    pub fn same_structure(a: &HashedType, b: &HashedType) -> bool {
        a.structural_hash == b.structural_hash
    }
}

impl std::ops::Deref for HashedType {
    type Target = TypeDefinition;
    fn deref(&self) -> &Self::Target {
        &self.type_def
    }
}

impl PartialEq for HashedType {
    fn eq(&self, other: &Self) -> bool {
        // Use content hash for exact equality (O(1) instead of deep comparison)
        self.content_hash == other.content_hash
    }
}

impl Eq for HashedType {}

impl std::hash::Hash for HashedType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Use precomputed content hash
        self.content_hash.hash(state);
    }
}

/// A handle to a hashed type that provides structural analysis capabilities
/// while hiding the Rc complexity and providing natural type access via Deref.
#[derive(Debug, Clone)]
pub struct StructuralType {
    inner: Rc<HashedType>,
}

impl StructuralType {
    /// Create a new StructuralType from a TypeDefinition
    pub fn new(type_def: TypeDefinition) -> Self {
        StructuralType {
            inner: Rc::new(HashedType::new(type_def)),
        }
    }

    /// Get the content hash (exact structure including field names)
    pub fn content_hash(type_def: &StructuralType) -> u64 {
        HashedType::content_hash(&type_def.inner)
    }

    /// Get the structural hash (field-normalized for alpha-equivalence)
    pub fn structural_hash(type_def: &StructuralType) -> u64 {
        HashedType::structural_hash(&type_def.inner)
    }

    /// Check if two types have the same content (exact equality)
    pub fn same_content(a: &StructuralType, b: &StructuralType) -> bool {
        HashedType::same_content(&a.inner, &b.inner)
    }

    /// Check if two types have the same structure (alpha-equivalent)
    pub fn same_structure(a: &StructuralType, b: &StructuralType) -> bool {
        HashedType::same_structure(&a.inner, &b.inner)
    }
}

impl std::ops::Deref for StructuralType {
    type Target = TypeDefinition;
    fn deref(&self) -> &Self::Target {
        &self.inner.type_def // Double deref: StructuralType -> HashedType -> TypeDefinition
    }
}

impl PartialEq for StructuralType {
    fn eq(&self, other: &Self) -> bool {
        // Use content hash for exact equality semantics
        self.inner.content_hash == other.inner.content_hash
    }
}

impl Eq for StructuralType {}

impl std::hash::Hash for StructuralType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Use content hash to match PartialEq semantics
        self.inner.content_hash.hash(state);
    }
}

impl PartialOrd for StructuralType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Fast ordering by content hash for deterministic ordering
        Some(self.inner.content_hash.cmp(&other.inner.content_hash))
    }
}

impl Ord for StructuralType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.content_hash.cmp(&other.inner.content_hash)
    }
}

// Hash computation functions

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Compute content hash for exact goal structure (including variable names)
fn compute_content_hash(goal: &Goal) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_goal_content(goal, &mut hasher);
    hasher.finish()
}

/// Compute structural hash with variable normalization for alpha-equivalence
fn compute_structural_hash(goal: &Goal) -> u64 {
    let mut normalizer = normalizer::VariableNormalizer::new();
    let normalized = normalizer.normalize_goal(goal);

    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    hasher.finish()
}

/// Compute content hash for exact type structure (including field names)
fn compute_type_content_hash(type_def: &TypeDefinition) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_type_content(type_def, &mut hasher);
    hasher.finish()
}

/// Compute structural hash for type with field normalization for alpha-equivalence
fn compute_type_structural_hash(type_def: &TypeDefinition) -> u64 {
    // For now, use the same as content hash since type normalization is more complex
    // This can be enhanced later with proper field name normalization
    compute_type_content_hash(type_def)
}

/// Hash the exact content of a goal (including variable names)
fn hash_goal_content(goal: &Goal, hasher: &mut impl Hasher) {
    match goal {
        Goal::Equality(left, right) => {
            "equality".hash(hasher);
            hash_term_content(left, hasher);
            hash_term_content(right, hasher);
        }
        Goal::Disequality(left, right) => {
            "disequality".hash(hasher);
            hash_term_content(left, hasher);
            hash_term_content(right, hasher);
        }
        Goal::PredicateCall(call) => {
            "predicate_call".hash(hasher);
            match &call.target {
                PredicateCallTarget::Predicate(predicate_id) => {
                    predicate_id.id.to_string().hash(hasher);
                }
                PredicateCallTarget::Variable(var) => {
                    var.to_string().hash(hasher);
                }
                PredicateCallTarget::Builtin(name) => {
                    name.hash(hasher);
                }
            }
            call.arguments.len().hash(hasher);
            for arg in &call.arguments {
                hash_term_content(arg, hasher);
            }
        }
        Goal::Conjunction(goals) => {
            "conjunction".hash(hasher);
            goals.len().hash(hasher);
            for goal in goals.iter() {
                StructuralGoal::content_hash(goal).hash(hasher);
            }
        }
        Goal::Disjunction(goals) => {
            "disjunction".hash(hasher);
            goals.len().hash(hasher);
            for goal in goals.iter() {
                StructuralGoal::content_hash(goal).hash(hasher);
            }
        }
        Goal::Fresh(fresh) => {
            "fresh".hash(hasher);
            fresh.variables.hash(hasher);
            fresh.body.len().hash(hasher);
            for goal in fresh.body.iter() {
                StructuralGoal::content_hash(goal).hash(hasher);
            }
        }
        Goal::Let(let_goal) => {
            "let".hash(hasher);
            let_goal.variable.hash(hasher);
            match &let_goal.value {
                Some(value) => {
                    true.hash(hasher);
                    hash_term_content(value, hasher);
                }
                None => false.hash(hasher),
            }
            let_goal.body.len().hash(hasher);
            for goal in let_goal.body.iter() {
                StructuralGoal::content_hash(goal).hash(hasher);
            }
        }
        Goal::Boolean(b) => {
            "boolean".hash(hasher);
            b.hash(hasher);
        }
        Goal::PatternMatch(pm) => {
            "pattern_match".hash(hasher);
            hash_term_content(&pm.term, hasher);
            pm.arms.len().hash(hasher);
            // Note: Full pattern hashing would be implemented here
        }
        Goal::Constraint(constraint) => {
            "constraint".hash(hasher);
            constraint.domain.hash(hasher);
            // Use template pointer for hashing since we can't hash trait objects directly
            std::ptr::addr_of!(*constraint.template.as_ref()).hash(hasher);
        }
        Goal::MetaLet(meta_let) => {
            "meta_let".hash(hasher);
            meta_let.variable.hash(hasher);
            // Note: Full meta expression hashing would be implemented here
        }
        Goal::MetaIf(_meta_if) => {
            "meta_if".hash(hasher);
            // Note: Full meta construct hashing would be implemented here
        }
        Goal::MetaFor(_meta_for) => {
            "meta_for".hash(hasher);
            // Note: Full meta construct hashing would be implemented here
        }
    }
}

/// Hash the exact content of a term (including variable names)
fn hash_term_content(term: &Term, hasher: &mut impl Hasher) {
    match term {
        Term::Variable(var) => {
            "variable".hash(hasher);
            var.hash(hasher);
        }
        Term::Wildcard => {
            "wildcard".hash(hasher);
        }
        Term::Literal(literal) => {
            "literal".hash(hasher);
            literal.hash(hasher);
        }
        Term::List(list) => {
            "list".hash(hasher);
            list.elements.len().hash(hasher);
            for element in &list.elements {
                hash_term_content(element, hasher);
            }
            match &list.tail {
                Some(tail) => {
                    true.hash(hasher);
                    hash_term_content(tail, hasher);
                }
                None => false.hash(hasher),
            }
        }
        Term::Struct(struct_construction) => {
            "struct".hash(hasher);
            struct_construction.type_ref.id.to_string().hash(hasher);
            // Note: Full struct field hashing would be implemented here
        }
        Term::EnumVariant(enum_construction) => {
            "enum_variant".hash(hasher);
            enum_construction.enum_ref.id.to_string().hash(hasher);
            enum_construction.variant_name.hash(hasher);
            // Note: Full enum variant hashing would be implemented here
        }
        Term::MetaInterpolation(_meta_expr) => {
            "meta_interpolation".hash(hasher);
            // Note: Full meta expression hashing would be implemented here
        }
        Term::Predicate(predicate_id) => {
            "predicate".hash(hasher);
            predicate_id.hash(hasher);
        }
    }
}

/// Hash the exact content of a type definition (including field names)
fn hash_type_content(type_def: &TypeDefinition, hasher: &mut impl Hasher) {
    // Hash the type ID path
    type_def.id.id.to_string().hash(hasher);

    // Hash the visibility
    hash_visibility(&type_def.visibility, hasher);

    // Hash the type kind
    match &type_def.kind {
        TypeKind::Struct(struct_def) => {
            "struct".hash(hasher);
            hash_struct_definition(struct_def, hasher);
        }
        TypeKind::Enum(enum_def) => {
            "enum".hash(hasher);
            hash_enum_definition(enum_def, hasher);
        }
    }
}

/// Hash a struct definition
fn hash_struct_definition(struct_def: &StructDefinition, hasher: &mut impl Hasher) {
    struct_def.name.hash(hasher);
    match &struct_def.fields {
        StructFields::Named(named_fields) => {
            "named_fields".hash(hasher);
            named_fields.len().hash(hasher);
            for field in named_fields {
                field.name.hash(hasher);
                match &field.type_ref {
                    TypeReference::Builtin(builtin_type) => builtin_type.as_str().hash(hasher),
                    TypeReference::UserDefined(type_id) => type_id.id.to_string().hash(hasher),
                }
                hash_visibility(&field.visibility, hasher);
            }
        }
        StructFields::Tuple(type_refs) => {
            "tuple_fields".hash(hasher);
            type_refs.len().hash(hasher);
            for type_ref in type_refs {
                match type_ref {
                    TypeReference::Builtin(builtin_type) => builtin_type.as_str().hash(hasher),
                    TypeReference::UserDefined(type_id) => type_id.id.to_string().hash(hasher),
                }
            }
        }
    }
}

/// Hash an enum definition
fn hash_enum_definition(enum_def: &EnumDefinition, hasher: &mut impl Hasher) {
    enum_def.name.hash(hasher);
    enum_def.variants.len().hash(hasher);
    for variant in &enum_def.variants {
        variant.name.hash(hasher);
        match &variant.kind {
            EnumVariantKind::Unit => {
                "unit".hash(hasher);
            }
            EnumVariantKind::Tuple(type_refs) => {
                "tuple".hash(hasher);
                type_refs.len().hash(hasher);
                for type_ref in type_refs {
                    match type_ref {
                        TypeReference::Builtin(builtin_type) => builtin_type.as_str().hash(hasher),
                        TypeReference::UserDefined(type_id) => type_id.id.to_string().hash(hasher),
                    }
                }
            }
            EnumVariantKind::Named(named_fields) => {
                "named".hash(hasher);
                named_fields.len().hash(hasher);
                for field in named_fields {
                    field.name.hash(hasher);
                    match &field.type_ref {
                        TypeReference::Builtin(builtin_type) => builtin_type.as_str().hash(hasher),
                        TypeReference::UserDefined(type_id) => type_id.id.to_string().hash(hasher),
                    }
                    hash_visibility(&field.visibility, hasher);
                }
            }
        }
    }
}

/// Hash a visibility level
fn hash_visibility(visibility: &Visibility, hasher: &mut impl Hasher) {
    match visibility {
        Visibility::Private => "private".hash(hasher),
        Visibility::Public => "public".hash(hasher),
        Visibility::Crate => "crate".hash(hasher),
        Visibility::Super => "super".hash(hasher),
        Visibility::SelfModule => "self".hash(hasher),
        Visibility::Restricted(item_id) => {
            "restricted".hash(hasher);
            item_id.to_string().hash(hasher);
        }
    }
}

// Variable normalization is now handled in the normalizer module

/// A resolved term in the IR
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    /// Variable reference
    Variable(InternedSymbol),
    /// Wildcard pattern
    Wildcard,
    /// Literal value
    Literal(Literal),
    /// List construction
    List(List),
    /// Struct construction (resolved to specific struct)
    Struct(StructConstruction),
    /// Enum variant construction (resolved to specific variant)
    EnumVariant(EnumVariantConstruction),
    /// Meta expression interpolation
    MetaInterpolation(MetaExpression),
    /// Predicate reference for higher-order predicates
    Predicate(PredicateId),
}

/// Literal values
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Literal {
    Boolean(bool),
    Integer(i64),
    String(Rc<str>),
    Char(char),
}

/// Target of a predicate call - can be a concrete predicate, a variable, or a builtin
#[derive(Debug, Clone, PartialEq)]
pub enum PredicateCallTarget {
    /// Direct call to a named predicate in the IR registry
    Predicate(PredicateId),
    /// Call to a predicate stored in a variable (higher-order predicate)
    Variable(InternedSymbol),
    /// Call to a builtin predicate (resolved at runtime)
    Builtin(String),
}

/// Predicate call with resolved reference
#[derive(Debug, Clone, PartialEq)]
pub struct PredicateCall {
    pub target: PredicateCallTarget,
    pub arguments: Vec<Term>,
}

impl PredicateCall {
    /// Create a predicate call to a named predicate (for backwards compatibility)
    pub fn to_predicate(predicate: PredicateId, arguments: Vec<Term>) -> Self {
        Self {
            target: PredicateCallTarget::Predicate(predicate),
            arguments,
        }
    }
    
    /// Create a predicate call to a predicate variable
    pub fn to_variable(variable: InternedSymbol, arguments: Vec<Term>) -> Self {
        Self {
            target: PredicateCallTarget::Variable(variable),
            arguments,
        }
    }
    
    /// Create a predicate call to a builtin
    pub fn to_builtin(name: String, arguments: Vec<Term>) -> Self {
        Self {
            target: PredicateCallTarget::Builtin(name),
            arguments,
        }
    }
    
    /// Get the predicate ID if this is a direct predicate call
    pub fn predicate(&self) -> Option<&PredicateId> {
        match &self.target {
            PredicateCallTarget::Predicate(id) => Some(id),
            PredicateCallTarget::Variable(_) => None,
            PredicateCallTarget::Builtin(_) => None,
        }
    }
}

/// Pattern matching
#[derive(Debug, Clone, PartialEq)]
pub struct PatternMatch {
    pub term: Term,
    pub arms: Vec<PatternArm>,
}

/// Pattern matching arm
#[derive(Debug, Clone, PartialEq)]
pub struct PatternArm {
    pub pattern: Pattern,
    pub guard: Option<Goal>,
    pub body: Rc<[StructuralGoal]>,
}

/// Patterns for matching
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Variable(InternedSymbol),
    Wildcard,
    Literal(Literal),
    List(ListPattern),
    Struct(StructPattern),
    EnumVariant(EnumVariantPattern),
}

/// List pattern
#[derive(Debug, Clone, PartialEq)]
pub struct ListPattern {
    pub elements: Vec<Pattern>,
    pub tail: Option<Box<Pattern>>,
}

/// Struct pattern
#[derive(Debug, Clone, PartialEq)]
pub struct StructPattern {
    pub type_ref: TypeReference,
    pub fields: StructPatternFields,
}

/// Struct pattern fields
#[derive(Debug, Clone, PartialEq)]
pub enum StructPatternFields {
    Named(Vec<NamedFieldPattern>),
    Tuple(Vec<Pattern>),
}

/// Named field pattern
#[derive(Debug, Clone, PartialEq)]
pub struct NamedFieldPattern {
    pub name: InternedSymbol,
    pub pattern: Pattern,
}

/// Enum variant pattern
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariantPattern {
    pub enum_ref: TypeReference,
    pub variant_name: InternedSymbol,
    pub kind: EnumVariantPatternKind,
}

/// Enum variant pattern kinds
#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantPatternKind {
    Unit,
    Tuple(Vec<Pattern>),
    Named(Vec<NamedFieldPattern>),
}

/// Fresh variable introduction
#[derive(Debug, Clone, PartialEq)]
pub struct Fresh {
    pub variables: Vec<InternedSymbol>,
    pub body: Rc<[StructuralGoal]>,
}

/// Let binding
#[derive(Debug, Clone, PartialEq)]
pub struct Let {
    pub variable: InternedSymbol,
    pub value: Option<Term>,
    pub body: Rc<[StructuralGoal]>,
}

/// List construction
#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub elements: Vec<Term>,
    pub tail: Option<Box<Term>>,
}

/// Struct construction
#[derive(Debug, Clone, PartialEq)]
pub struct StructConstruction {
    pub type_ref: TypeId,
    pub fields: StructConstructionFields,
}

/// Struct construction fields
#[derive(Debug, Clone, PartialEq)]
pub enum StructConstructionFields {
    Named(Vec<NamedFieldConstruction>),
    Tuple(Vec<Term>),
}

/// Named field construction
#[derive(Debug, Clone, PartialEq)]
pub struct NamedFieldConstruction {
    pub name: InternedSymbol,
    pub value: Term,
}

/// Enum variant construction
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariantConstruction {
    pub enum_ref: TypeId,
    pub variant_name: InternedSymbol,
    pub kind: EnumVariantConstructionKind,
}

/// Enum variant construction kinds
#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantConstructionKind {
    Unit,
    Tuple(Vec<Term>),
    Named(Vec<NamedFieldConstruction>),
}

/// Constraint block with compiled template
#[derive(Debug, Clone)]
pub struct ConstraintBlock {
    pub domain: Rc<str>,
    pub template: Rc<dyn DomainConstraintTemplate>,
}

impl PartialEq for ConstraintBlock {
    fn eq(&self, other: &Self) -> bool {
        // Compare domains and template pointers (pointer equality for templates)
        self.domain == other.domain && Rc::ptr_eq(&self.template, &other.template)
    }
}

/// Meta value for template expansion (compile-time values)
#[derive(Debug, Clone, PartialEq)]
pub enum MetaValue {
    Integer(i64),
    String(Rc<str>),
    Boolean(bool),
}

/// Meta expression for compile-time computation
#[derive(Debug, Clone, PartialEq)]
pub enum MetaExpression {
    Variable(InternedSymbol),
    Literal(MetaValue),
    BinaryOp(MetaBinaryOp, Box<MetaExpression>, Box<MetaExpression>),
}

/// Binary operators for meta expressions
#[derive(Debug, Clone, PartialEq)]
pub enum MetaBinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    Equal,
    NotEqual,
    And,
    Or,
}

/// Meta let statement for variable binding at compile time
#[derive(Debug, Clone, PartialEq)]
pub struct MetaLet {
    pub variable: InternedSymbol,
    pub variable_type: TypeAnnotation,
    pub expression: MetaExpression,
}

/// Meta if statement for conditional compilation
#[derive(Debug, Clone, PartialEq)]
pub struct MetaIf {
    pub condition: MetaExpression,
    pub then_body: Rc<[StructuralGoal]>,
    pub else_ifs: Vec<(MetaExpression, Rc<[StructuralGoal]>)>,
    pub else_body: Option<Rc<[StructuralGoal]>>,
}

/// Meta for statement for iterative expansion
#[derive(Debug, Clone, PartialEq)]
pub struct MetaFor {
    pub variable: InternedSymbol,
    pub variable_type: TypeAnnotation,
    pub start: MetaExpression,
    pub end: MetaExpression,
    pub body: Rc<[StructuralGoal]>,
}

// Helper functions for creating goal containers
impl StructuralGoal {
    /// Create an empty goal container
    pub fn empty_container() -> Rc<[StructuralGoal]> {
        Rc::from(Vec::<StructuralGoal>::new())
    }

    /// Create a goal container from a vector of goals
    pub fn from_vec(goals: Vec<Goal>) -> Rc<[StructuralGoal]> {
        let structural_goals: Vec<StructuralGoal> =
            goals.into_iter().map(StructuralGoal::new).collect();
        Rc::from(structural_goals)
    }
}

// Registry implementation will be in registry.rs
pub use registry::ItemRegistry;
