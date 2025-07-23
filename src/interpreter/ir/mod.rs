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

use crate::interpreter::symbol_table::InternedSymbol;
use crate::interpreter::constraint_domains::IrDomainConstraintTemplate;
use std::borrow::Borrow;
use std::fmt::{self, Display};
use std::rc::Rc;

pub mod compiler;
pub mod display;
pub mod errors;
pub mod normalizer;
pub mod registry;
pub mod validation;

/// Stable identifier for any item in the IR (Module, Type, or Predicate)
/// Uses path-based identification with separate namespaces by kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemId {
    /// Absolute path to the item (e.g., "::std::collections::HashMap")
    pub path: Rc<str>,
    /// Item kind for namespace separation
    pub kind: ItemKind,
}

impl ItemId {
    /// Create a new ItemId
    pub fn new(path: impl Into<Rc<str>>, kind: ItemKind) -> Self {
        Self {
            path: path.into(),
            kind,
        }
    }

    /// Create a global ItemId (no module prefix)
    pub fn global(name: &str, kind: ItemKind) -> Self {
        Self::new(format!("::{}", name), kind)
    }

    /// Create an ItemId from path segments
    pub fn from_segments(segments: &[&str], kind: ItemKind) -> Self {
        let path = format!("::{}", segments.join("::"));
        Self::new(path, kind)
    }
}

impl Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
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
    pub fn new(path: impl Into<Rc<str>>) -> Self {
        Self {
            id: ItemId::new(path, ItemKind::Type),
        }
    }

    /// Convert to ItemId (cleaner than .as_ref().clone())
    pub fn to_item_id(&self) -> ItemId {
        self.id.clone()
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
    pub fn new(path: impl Into<Rc<str>>) -> Self {
        Self {
            id: ItemId::new(path, ItemKind::Predicate),
        }
    }

    /// Convert to ItemId (cleaner than .as_ref().clone())
    pub fn to_item_id(&self) -> ItemId {
        self.id.clone()
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
    pub fn new(path: impl Into<Rc<str>>) -> Self {
        Self {
            id: ItemId::new(path, ItemKind::Module),
        }
    }

    /// Convert to ItemId (cleaner than .as_ref().clone())
    pub fn to_item_id(&self) -> ItemId {
        self.id.clone()
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

impl Borrow<str> for ModuleId {
    fn borrow(&self) -> &str {
        self.id.path.as_ref()
    }
}

/// Represents an import in the IR
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    /// Unique identifier for this import
    pub id: ItemId,
    /// Local name of the imported symbol in the importing module
    pub local_name: String,
    /// Reference to the actual item being imported
    pub source_ref: ItemId,
    /// Visibility of this import (pub use vs private use)
    pub visibility: Visibility,
    /// Optional alias (Some("alias") for "use item as alias", None for "use item")
    pub alias: Option<String>,
    /// Kind of import (simple, glob, list)
    pub import_kind: ImportKind,
}

/// Different kinds of import statements
#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    /// Simple import: use path::item;
    Simple,
    /// Glob import: use path::*;
    Glob,
    /// List import: use path::{item1, item2 as alias};
    List,
}

/// Any item in the IR - unified storage for all item types
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Module(Module),
    Type(TypeDefinition),
    Predicate(Predicate),
    Import(Import),
}

impl Item {
    /// Get the ItemId for this item
    pub fn id(&self) -> &ItemId {
        match self {
            Item::Module(m) => &m.id.id,
            Item::Type(t) => &t.id.id,
            Item::Predicate(p) => &p.id.id,
            Item::Import(i) => &i.id,
        }
    }

    /// Get the name of this item (last segment of the path)
    pub fn name(&self) -> &str {
        let path = &self.id().path;
        path.split("::").last().unwrap_or(path)
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
    /// Create a new empty program
    pub fn new() -> Self {
        Self {
            registry: Rc::new(ItemRegistry::new()),
            symbol_table: Rc::new(crate::interpreter::symbol_table::SymbolTable::new()),
        }
    }

    /// Get a mutable reference to the registry using copy-on-write
    pub fn registry_mut(&mut self) -> &mut ItemRegistry {
        Rc::make_mut(&mut self.registry)
    }
}

/// Module definition
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// Module identifier
    pub id: ModuleId,
    /// Parent module reference (None for root module)
    pub parent: Option<ModuleId>,
    /// Child items in this module
    pub items: Vec<ItemId>,
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
    /// Integer type for meta programming
    Int,
    /// String type for meta programming
    String,
    /// Boolean type for meta programming
    Bool,
    /// Relation type with arity
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

/// Struct definition
#[derive(Debug, Clone, PartialEq)]
pub struct StructDefinition {
    pub fields: StructFields,
}

/// Struct field types
#[derive(Debug, Clone, PartialEq)]
pub enum StructFields {
    Named(Vec<NamedField>),
    Tuple(Vec<TypeId>),
}

/// Named field in a struct
#[derive(Debug, Clone, PartialEq)]
pub struct NamedField {
    pub name: InternedSymbol,
    pub type_ref: TypeId,
    pub visibility: Visibility,
}

/// Enum definition
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDefinition {
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
    Tuple(Vec<TypeId>),
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
    content_hash: u64,     // Exact goal structure hash (including variable names)
    structural_hash: u64,  // Variable-normalized structure hash (for alpha-equivalence)
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
            inner: Rc::new(HashedGoal::new(goal))
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
        &self.inner.goal  // Double deref: StructuralGoal -> HashedGoal -> Goal
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
    content_hash: u64,     // Exact type structure hash (including field names)
    structural_hash: u64,  // Field-normalized structure hash (for alpha-equivalence)
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
            inner: Rc::new(HashedType::new(type_def))
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
        &self.inner.type_def  // Double deref: StructuralType -> HashedType -> TypeDefinition
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
            call.predicate.id.path.hash(hasher);
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
        Goal::MetaIf(meta_if) => {
            "meta_if".hash(hasher);
            // Note: Full meta construct hashing would be implemented here
        }
        Goal::MetaFor(meta_for) => {
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
            struct_construction.type_ref.id.path.hash(hasher);
            // Note: Full struct field hashing would be implemented here
        }
        Term::EnumVariant(enum_construction) => {
            "enum_variant".hash(hasher);
            enum_construction.enum_ref.id.path.hash(hasher);
            enum_construction.variant_name.hash(hasher);
            // Note: Full enum variant hashing would be implemented here
        }
        Term::MetaInterpolation(_meta_expr) => {
            "meta_interpolation".hash(hasher);
            // Note: Full meta expression hashing would be implemented here
        }
    }
}

/// Hash the exact content of a type definition (including field names)
fn hash_type_content(type_def: &TypeDefinition, hasher: &mut impl Hasher) {
    // Hash the type ID path
    type_def.id.id.path.hash(hasher);
    
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
    match &struct_def.fields {
        StructFields::Named(named_fields) => {
            "named_fields".hash(hasher);
            named_fields.len().hash(hasher);
            for field in named_fields {
                field.name.hash(hasher);
                field.type_ref.id.path.hash(hasher);
                hash_visibility(&field.visibility, hasher);
            }
        }
        StructFields::Tuple(type_refs) => {
            "tuple_fields".hash(hasher);
            type_refs.len().hash(hasher);
            for type_ref in type_refs {
                type_ref.id.path.hash(hasher);
            }
        }
    }
}

/// Hash an enum definition
fn hash_enum_definition(enum_def: &EnumDefinition, hasher: &mut impl Hasher) {
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
                    type_ref.id.path.hash(hasher);
                }
            }
            EnumVariantKind::Named(named_fields) => {
                "named".hash(hasher);
                named_fields.len().hash(hasher);
                for field in named_fields {
                    field.name.hash(hasher);
                    field.type_ref.id.path.hash(hasher);
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
            item_id.path.hash(hasher);
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
}

/// Literal values
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Literal {
    Boolean(bool),
    Integer(i64),
    String(Rc<str>),
    Char(char),
}

/// Predicate call with resolved reference
#[derive(Debug, Clone, PartialEq)]
pub struct PredicateCall {
    pub predicate: PredicateId,
    pub arguments: Vec<Term>,
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
    pub type_ref: TypeId,
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
    pub enum_ref: TypeId,
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
    pub template: Rc<dyn IrDomainConstraintTemplate>,
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
        let structural_goals: Vec<StructuralGoal> = goals
            .into_iter()
            .map(StructuralGoal::new)
            .collect();
        Rc::from(structural_goals)
    }
}

// Registry implementation will be in registry.rs
pub use registry::ItemRegistry;

// Compiler error types and main compiler
pub use errors::CompileError;
pub use compiler::Compiler;
