use crate::interpreter::metaprogramming::{MetaExpression, MetaStatement, TypeAnnotation};
use crate::interpreter::symbol_table::InternedSymbol;
use std::fmt::{self, Display};

/// Helper function to join InternedSymbol slices with a separator
fn join_symbols(symbols: &[InternedSymbol], separator: &str) -> String {
    symbols.iter()
        .map(|s| s.text())
        .collect::<Vec<_>>()
        .join(separator)
}

/// Represents a qualified path for module resolution
#[derive(Debug, Clone, PartialEq)]
pub enum QualifiedPath {
    /// Global namespace root: ::module::item  
    Global(Vec<InternedSymbol>),
    /// Absolute path from crate root: crate::module::item
    Absolute(Vec<InternedSymbol>),
    /// Relative path: module::item
    Relative(Vec<InternedSymbol>),
    /// Parent module path: super::module::item (levels, segments)
    Super(usize, Vec<InternedSymbol>),
    /// Current module path: self::module::item
    Self_(Vec<InternedSymbol>),
    /// External crate path: crate_name::module::item
    External(InternedSymbol, Vec<InternedSymbol>),
}

impl QualifiedPath {
    /// Create a simple relative path from segments
    pub fn simple(segments: Vec<InternedSymbol>) -> Self {
        QualifiedPath::Relative(segments)
    }

    /// Create a global namespace path  
    pub fn global(segments: Vec<InternedSymbol>) -> Self {
        QualifiedPath::Global(segments)
    }

    /// Create a crate-absolute path
    pub fn absolute(segments: Vec<InternedSymbol>) -> Self {
        QualifiedPath::Absolute(segments)
    }

    /// Create a super path with given levels up
    pub fn super_path(levels: usize, segments: Vec<InternedSymbol>) -> Self {
        QualifiedPath::Super(levels, segments)
    }

    /// Create a self path
    pub fn self_path(segments: Vec<InternedSymbol>) -> Self {
        QualifiedPath::Self_(segments)
    }

    /// Create an external crate path
    pub fn external_path(crate_name: InternedSymbol, segments: Vec<InternedSymbol>) -> Self {
        QualifiedPath::External(crate_name, segments)
    }

    /// Get all segments as a single vector (excluding crate/std/super prefixes)
    pub fn segments(&self) -> &[InternedSymbol] {
        match self {
            QualifiedPath::Global(segments)
            | QualifiedPath::Absolute(segments)
            | QualifiedPath::Relative(segments)
            | QualifiedPath::Super(_, segments)
            | QualifiedPath::Self_(segments) => segments,
            QualifiedPath::External(_, segments) => segments,
        }
    }

    /// Get the final segment (the actual item name)
    pub fn final_segment(&self) -> Option<&InternedSymbol> {
        self.segments().last()
    }

    /// Get all segments except the final one (the module path)
    pub fn module_segments(&self) -> &[InternedSymbol] {
        let segments = self.segments();
        if segments.is_empty() {
            segments
        } else {
            &segments[..segments.len() - 1]
        }
    }
}

impl Display for QualifiedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QualifiedPath::Global(segments) => {
                write!(f, "::{}", join_symbols(segments, "::"))
            }
            QualifiedPath::Absolute(segments) => {
                write!(f, "crate::{}", join_symbols(segments, "::"))
            }
            QualifiedPath::Relative(segments) => {
                write!(f, "{}", join_symbols(segments, "::"))
            }
            QualifiedPath::Super(levels, segments) => {
                let super_part = "super::".repeat(*levels);
                write!(f, "{}{}", super_part, join_symbols(segments, "::"))
            }
            QualifiedPath::Self_(segments) => {
                write!(f, "self::{}", join_symbols(segments, "::"))
            }
            QualifiedPath::External(crate_name, segments) => {
                write!(f, "{}::{}", crate_name, join_symbols(segments, "::"))
            }
        }
    }
}

/// Represents a qualified name (path + final identifier)
#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedName {
    pub path: QualifiedPath,
    pub name: InternedSymbol,
}

impl QualifiedName {
    pub fn new(path: QualifiedPath, name: InternedSymbol) -> Self {
        Self { path, name }
    }

    /// Create from a simple identifier (no path)
    pub fn simple(name: InternedSymbol) -> Self {
        Self {
            path: QualifiedPath::Relative(vec![]),
            name,
        }
    }

    /// Create from path segments where the last segment is the name
    pub fn from_segments(segments: Vec<InternedSymbol>) -> Self {
        if segments.is_empty() {
            panic!("Cannot create QualifiedName from empty segments");
        }

        let name = segments.last().unwrap().clone();
        let path_segments = segments[..segments.len() - 1].to_vec();

        Self {
            path: QualifiedPath::Relative(path_segments),
            name,
        }
    }
}

impl Display for QualifiedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path_str = self.path.to_string();
        if path_str.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}::{}", path_str, self.name)
        }
    }
}

/// Source location information for better error reporting
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    /// Start position (byte offset)
    pub start: usize,
    /// End position (byte offset)
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Combine two spans into one that covers both
    pub fn union(&self, other: &Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::dummy()
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// Visibility modifier for items
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    /// No visibility modifier - private to current module
    Private,
    /// `pub` - public to everyone
    Public,
    /// `pub(crate)` - visible within the current crate
    Crate,
    /// `pub(super)` - visible to parent module
    Super,
    /// `pub(self)` - visible within current module (same as Private)
    SelfModule,
    /// `pub(module::path)` - visible to specific module path
    Restricted(QualifiedPath),
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Private
    }
}

impl Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Private => write!(f, ""),
            Visibility::Public => write!(f, "pub "),
            Visibility::Crate => write!(f, "pub(crate) "),
            Visibility::Super => write!(f, "pub(super) "),
            Visibility::SelfModule => write!(f, "pub(self) "),
            Visibility::Restricted(path) => write!(f, "pub({}) ", path),
        }
    }
}

/// Trait for AST nodes that have source location information
pub trait Spanned {
    fn span(&self) -> &Span;
}

/// A convenient wrapper for AST nodes with span information
#[derive(Debug, Clone)]
pub struct Located<T> {
    pub node: T,
    pub span: Span,
}

impl<T: PartialEq> PartialEq for Located<T> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl<T> Located<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }

    pub fn dummy(node: T) -> Self {
        Self {
            node,
            span: Span::dummy(),
        }
    }
}

impl<T> Default for Located<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            node: T::default(),
            span: Span::default(),
        }
    }
}

impl<T> Spanned for Located<T> {
    fn span(&self) -> &Span {
        &self.span
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}

impl PartialEq for Program {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}

impl Spanned for Program {
    fn span(&self) -> &Span {
        &self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Use(UseStatement),
    ModuleDeclaration(ModuleDeclaration),
    Module(ModuleDefinition),
    Struct(StructDefinition),
    Enum(EnumDefinition),
    Impl(ImplBlock),
    Predicate(PredicateDefinition),
}

impl Spanned for Item {
    fn span(&self) -> &Span {
        match self {
            Item::Use(use_stmt) => use_stmt.span(),
            Item::ModuleDeclaration(mod_decl) => mod_decl.span(),
            Item::Module(module) => module.span(),
            Item::Struct(struct_def) => struct_def.span(),
            Item::Enum(enum_def) => enum_def.span(),
            Item::Impl(impl_block) => impl_block.span(),
            Item::Predicate(predicate) => predicate.span(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UseStatement {
    pub path: UsePath,
    pub span: Span,
}

impl PartialEq for UseStatement {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Spanned for UseStatement {
    fn span(&self) -> &Span {
        &self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UsePath {
    /// Simple import: use path::to::item;
    Simple(QualifiedPath, String),
    /// Glob import: use path::to::*;
    Glob(QualifiedPath),
    /// List import: use path::to::{item1, item2 as alias};
    List(QualifiedPath, Vec<(String, Option<String>)>),
}

#[derive(Debug, Clone)]
pub struct ModuleDefinition {
    pub visibility: Visibility,
    pub name: InternedSymbol,
    pub search_strategy: Option<SearchStrategy>,
    pub items: Vec<Item>,
    pub span: Span,
}

impl PartialEq for ModuleDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.visibility == other.visibility
            && self.name == other.name
            && self.search_strategy == other.search_strategy
            && self.items == other.items
    }
}

impl Spanned for ModuleDefinition {
    fn span(&self) -> &Span {
        &self.span
    }
}

/// Module declaration referencing an external file (mod name;)
#[derive(Debug, Clone)]
pub struct ModuleDeclaration {
    pub visibility: Visibility,
    pub name: InternedSymbol,
    pub span: Span,
}

impl PartialEq for ModuleDeclaration {
    fn eq(&self, other: &Self) -> bool {
        self.visibility == other.visibility && self.name == other.name
    }
}

impl Spanned for ModuleDeclaration {
    fn span(&self) -> &Span {
        &self.span
    }
}

impl Display for ModuleDeclaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}mod {};", self.visibility, self.name)
    }
}

#[derive(Debug, Clone)]
pub struct StructDefinition {
    pub visibility: Visibility,
    pub name: InternedSymbol,
    pub kind: StructKind,
    pub span: Span,
}

impl PartialEq for StructDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.visibility == other.visibility && self.name == other.name && self.kind == other.kind
    }
}

impl Spanned for StructDefinition {
    fn span(&self) -> &Span {
        &self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StructKind {
    Tuple(Vec<InternedSymbol>),
    Named(Vec<NamedField>),
}

#[derive(Debug, Clone)]
pub struct NamedField {
    pub visibility: Visibility,
    pub name: InternedSymbol,
    pub type_name: QualifiedPath,
    pub span: Span,
}

impl PartialEq for NamedField {
    fn eq(&self, other: &Self) -> bool {
        self.visibility == other.visibility
            && self.name == other.name
            && self.type_name == other.type_name
    }
}

impl Spanned for NamedField {
    fn span(&self) -> &Span {
        &self.span
    }
}

#[derive(Debug, Clone)]
pub struct EnumDefinition {
    pub visibility: Visibility,
    pub name: InternedSymbol,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

impl PartialEq for EnumDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.visibility == other.visibility 
            && self.name == other.name 
            && self.variants == other.variants
    }
}

impl Spanned for EnumDefinition {
    fn span(&self) -> &Span {
        &self.span
    }
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: InternedSymbol,
    pub kind: VariantKind,
    pub span: Span,
}

impl PartialEq for EnumVariant {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.kind == other.kind
    }
}

impl Spanned for EnumVariant {
    fn span(&self) -> &Span {
        &self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariantKind {
    Unit,                           // Color::Red
    Tuple(Vec<InternedSymbol>),            // Option::Some(T)
    Named(Vec<NamedField>),        // Person::Named { name: String, age: u32 }
}

#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub type_name: InternedSymbol,
    pub predicates: Vec<PredicateDefinition>,
    pub span: Span,
}

impl PartialEq for ImplBlock {
    fn eq(&self, other: &Self) -> bool {
        self.type_name == other.type_name && self.predicates == other.predicates
    }
}

impl Spanned for ImplBlock {
    fn span(&self) -> &Span {
        &self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeArg {
    Flag(String),
    Named(String, Term),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: InternedSymbol,
    pub args: Vec<AttributeArg>,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PredicateKind {
    Relation, // Regular relation defined with 'rel'
    Macro,    // Template relation defined with 'macro'
}

impl Display for PredicateKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PredicateKind::Relation => write!(f, "rel"),
            PredicateKind::Macro => write!(f, "macro"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PredicateDefinition {
    pub visibility: Visibility,
    pub predicate_kind: PredicateKind,
    pub attributes: Vec<Attribute>,
    pub name: InternedSymbol,
    pub parameters: Vec<Parameter>,
    pub search_strategy: Option<SearchStrategy>,
    pub body: GoalBody,
    pub span: Span,
}

impl PartialEq for PredicateDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.visibility == other.visibility
            && self.predicate_kind == other.predicate_kind
            && self.attributes == other.attributes
            && self.name == other.name
            && self.parameters == other.parameters
            && self.search_strategy == other.search_strategy
            && self.body == other.body
    }
}

impl Spanned for PredicateDefinition {
    fn span(&self) -> &Span {
        &self.span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: InternedSymbol,
    pub type_annotation: Option<TypeAnnotation>,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum SearchStrategy {
    Bfs, // default
    Dfs,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchParams {
    pub strategy: Option<SearchStrategy>,
    pub limit: Option<u64>,
    pub depth: Option<u64>,
    pub custom_params: Vec<(String, SearchParamValue)>,
}

impl SearchParams {
    pub fn new() -> Self {
        Self {
            strategy: None,
            limit: None,
            depth: None,
            custom_params: Vec::new(),
        }
    }

    pub fn with_strategy(mut self, strategy: SearchStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    pub fn with_limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_depth(mut self, depth: u64) -> Self {
        self.depth = Some(depth);
        self
    }

    pub fn with_custom_param(mut self, name: InternedSymbol, value: SearchParamValue) -> Self {
        self.custom_params.push((name.to_string(), value));
        self
    }
}

impl Display for SearchParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        let mut first = true;

        if let Some(strategy) = &self.strategy {
            write!(f, "strategy = {}", strategy)?;
            first = false;
        }

        if let Some(limit) = self.limit {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "limit = {}", limit)?;
            first = false;
        }

        if let Some(depth) = self.depth {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "depth = {}", depth)?;
            first = false;
        }

        for (name, value) in &self.custom_params {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{} = {}", name, value)?;
            first = false;
        }

        write!(f, ")")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchParamValue {
    Number(i64),
    String(String),
    Identifier(String),
    Boolean(bool),
}

impl Display for SearchParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchParamValue::Number(n) => write!(f, "{}", n),
            SearchParamValue::String(s) => write!(f, "\"{}\"", s),
            SearchParamValue::Identifier(id) => write!(f, "{}", id),
            SearchParamValue::Boolean(b) => write!(f, "{}", b),
        }
    }
}

pub type GoalBody = Vec<Goal>;

#[derive(Debug, Clone)]
pub enum Goal {
    Let(LetDeclaration, Span),
    Fresh(FreshVariables, Span),
    Disjunction(Disjunction, Span),
    Conjunction(Conjunction, Span),
    PatternMatch(PatternMatching, Span),
    RelationCall(RelationCall, Span),
    MethodCall(MethodCall, Span),
    Equality(Term, Term, Span),
    Disequality(Term, Term, Span),
    Parenthesized(GoalBody, Span),
    BooleanLiteral(bool, Span),
    ConstraintBlock(ConstraintBlock, Span),
    // NEW: Meta programming constructs
    MetaStatement(MetaStatement, Span),
}

impl PartialEq for Goal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Goal::Let(a, _), Goal::Let(b, _)) => a == b,
            (Goal::Fresh(a, _), Goal::Fresh(b, _)) => a == b,
            (Goal::Disjunction(a, _), Goal::Disjunction(b, _)) => a == b,
            (Goal::Conjunction(a, _), Goal::Conjunction(b, _)) => a == b,
            (Goal::PatternMatch(a, _), Goal::PatternMatch(b, _)) => a == b,
            (Goal::RelationCall(a, _), Goal::RelationCall(b, _)) => a == b,
            (Goal::MethodCall(a, _), Goal::MethodCall(b, _)) => a == b,
            (Goal::Equality(a1, a2, _), Goal::Equality(b1, b2, _)) => a1 == b1 && a2 == b2,
            (Goal::Disequality(a1, a2, _), Goal::Disequality(b1, b2, _)) => a1 == b1 && a2 == b2,
            (Goal::Parenthesized(a, _), Goal::Parenthesized(b, _)) => a == b,
            (Goal::BooleanLiteral(a, _), Goal::BooleanLiteral(b, _)) => a == b,
            (Goal::ConstraintBlock(a, _), Goal::ConstraintBlock(b, _)) => a == b,
            (Goal::MetaStatement(a, _), Goal::MetaStatement(b, _)) => a == b,
            _ => false,
        }
    }
}

impl Spanned for Goal {
    fn span(&self) -> &Span {
        match self {
            Goal::Let(_, span) => span,
            Goal::Fresh(_, span) => span,
            Goal::Disjunction(_, span) => span,
            Goal::Conjunction(_, span) => span,
            Goal::PatternMatch(_, span) => span,
            Goal::RelationCall(_, span) => span,
            Goal::MethodCall(_, span) => span,
            Goal::Equality(_, _, span) => span,
            Goal::Disequality(_, _, span) => span,
            Goal::Parenthesized(_, span) => span,
            Goal::BooleanLiteral(_, span) => span,
            Goal::ConstraintBlock(_, span) => span,
            Goal::MetaStatement(_, span) => span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetDeclaration {
    pub var_name: InternedSymbol,
    pub value: Option<Term>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FreshVariables {
    pub vars: Vec<InternedSymbol>,
    pub body: GoalBody,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Disjunction {
    pub body: GoalBody,
    pub params: Option<SearchParams>,
}

impl Disjunction {
    pub fn new(body: GoalBody) -> Self {
        Self { body, params: None }
    }

    pub fn with_params(body: GoalBody, params: SearchParams) -> Self {
        Self {
            body,
            params: Some(params),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conjunction {
    pub body: GoalBody,
    pub params: Option<SearchParams>,
}

impl Conjunction {
    pub fn new(body: GoalBody) -> Self {
        Self { body, params: None }
    }

    pub fn with_params(body: GoalBody, params: SearchParams) -> Self {
        Self {
            body,
            params: Some(params),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatternMatching {
    pub term: Term,
    pub arms: Vec<PatternArm>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatternArm {
    pub pattern: Pattern,
    pub body: GoalBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationName {
    /// Simple unqualified name
    Simple(InternedSymbol),
    /// Qualified name with module path
    Qualified(QualifiedName),
}

impl RelationName {
    /// Get the final name component
    pub fn name(&self) -> &str {
        match self {
            RelationName::Simple(name) => name.text(),
            RelationName::Qualified(qualified) => qualified.name.text(),
        }
    }

    /// Check if this is a simple (unqualified) name
    pub fn is_simple(&self) -> bool {
        matches!(self, RelationName::Simple(_))
    }

    /// Check if this is a qualified name
    pub fn is_qualified(&self) -> bool {
        matches!(self, RelationName::Qualified(_))
    }

    /// Convert to a qualified name, using empty path for simple names
    pub fn to_qualified(&self) -> QualifiedName {
        match self {
            RelationName::Simple(name) => QualifiedName::simple(name.clone()),
            RelationName::Qualified(qualified) => qualified.clone(),
        }
    }
}

impl Display for RelationName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelationName::Simple(name) => write!(f, "{}", name),
            RelationName::Qualified(qualified) => write!(f, "{}", qualified),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationCall {
    pub name: RelationName,
    pub args: Vec<CallArgument>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallArgument {
    Term(Term),
    MetaExpression(MetaExpression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodCall {
    pub receiver: Box<Term>,
    pub method: InternedSymbol,
    pub args: Vec<Term>,
}


#[derive(Debug, Clone)]
pub enum Term {
    Literal(Literal, Span),
    Variable(InternedSymbol),
    Wildcard(Span),
    List(ListConstruction, Span),
    NamedStruct(NamedStructConstruction, Span),
    TupleStruct(TupleStructConstruction, Span),
    EnumVariant(EnumVariantConstruction, Span),
    Parenthesized(Box<Term>, Span),
    // NEW: Interpolation for meta expressions
    Interpolation(MetaExpression, Span),
}

impl PartialEq for Term {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Term::Literal(a, _), Term::Literal(b, _)) => a == b,
            (Term::Variable(a), Term::Variable(b)) => a == b,
            (Term::Wildcard(_), Term::Wildcard(_)) => true,
            (Term::List(a, _), Term::List(b, _)) => a == b,
            (Term::NamedStruct(a, _), Term::NamedStruct(b, _)) => a == b,
            (Term::TupleStruct(a, _), Term::TupleStruct(b, _)) => a == b,
            (Term::EnumVariant(a, _), Term::EnumVariant(b, _)) => a == b,
            (Term::Parenthesized(a, _), Term::Parenthesized(b, _)) => a == b,
            (Term::Interpolation(a, _), Term::Interpolation(b, _)) => a == b,
            _ => false,
        }
    }
}

impl Spanned for Term {
    fn span(&self) -> &Span {
        match self {
            Term::Literal(_, span) => span,
            Term::Variable(symbol) => symbol.span_ref(),
            Term::Wildcard(span) => span,
            Term::List(_, span) => span,
            Term::NamedStruct(_, span) => span,
            Term::TupleStruct(_, span) => span,
            Term::EnumVariant(_, span) => span,
            Term::Parenthesized(_, span) => span,
            Term::Interpolation(_, span) => span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListConstruction {
    pub elements: Vec<Term>,
    pub tail: Option<Box<Term>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedStructConstruction {
    pub name: InternedSymbol,
    pub fields: Vec<FieldInitializer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldInitializer {
    pub name: InternedSymbol,
    pub value: Term,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TupleStructConstruction {
    pub name: QualifiedPath,
    pub args: Vec<Term>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariantConstruction {
    pub enum_name: InternedSymbol,
    pub variant_name: InternedSymbol,
    pub kind: EnumVariantConstructionKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantConstructionKind {
    Unit,                         // Color::Red
    Tuple(Vec<Term>),            // Option::Some(42)
    Named(Vec<FieldInitializer>),     // Result::Ok { value: 42 }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Boolean(bool),
    Number(String),
    String(String),
    Char(char),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Literal(Literal),
    Variable(InternedSymbol),
    Wildcard,
    List(ListPattern),
    NamedStruct(NamedStructPattern),
    TupleStruct(TupleStructPattern),
    EnumVariant(EnumVariantPattern),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListPattern {
    pub elements: Vec<Pattern>,
    pub tail: Option<Box<Pattern>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedStructPattern {
    pub name: InternedSymbol,
    pub fields: Vec<FieldPattern>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldPattern {
    pub name: InternedSymbol,
    pub pattern: Pattern,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TupleStructPattern {
    pub name: InternedSymbol,
    pub args: Vec<Pattern>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariantPattern {
    pub enum_name: InternedSymbol,
    pub variant_name: InternedSymbol,
    pub kind: EnumVariantPatternKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantPatternKind {
    Unit,
    Tuple(Vec<Pattern>),
    Named(Vec<FieldPattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintBlock {
    pub domain: String,
    pub body: ConstraintBody,
}

#[derive(Debug, Clone)]
pub struct ConstraintBody {
    pub raw_content: String,
    pub span: Span,
}

impl PartialEq for ConstraintBody {
    fn eq(&self, other: &Self) -> bool {
        self.raw_content == other.raw_content
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for item in &self.items {
            writeln!(f, "{}", item)?;
        }
        Ok(())
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Item::Use(u) => write!(f, "{}", u),
            Item::ModuleDeclaration(md) => write!(f, "{}", md),
            Item::Module(m) => write!(f, "{}", m),
            Item::Struct(s) => write!(f, "{}", s),
            Item::Enum(e) => write!(f, "{}", e),
            Item::Impl(i) => write!(f, "{}", i),
            Item::Predicate(r) => write!(f, "{}", r),
        }
    }
}

impl Display for UseStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "use {};", self.path)
    }
}

impl Display for UsePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsePath::Simple(path, item) => write!(f, "{}::{}", path, item),
            UsePath::Glob(path) => write!(f, "{}::*", path),
            UsePath::List(path, imports) => {
                let import_str = imports
                    .iter()
                    .map(|(name, alias)| {
                        if let Some(alias) = alias {
                            format!("{} as {}", name, alias)
                        } else {
                            name.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{}::{{{}}}", path, import_str)
            }
        }
    }
}

impl Display for ModuleDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "mod {} {{", self.name)?;
        for item in &self.items {
            writeln!(f, "    {}", item)?;
        }
        writeln!(f, "}}")
    }
}

impl Display for StructDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}struct {} {}", self.visibility, self.name, self.kind)
    }
}

impl Display for StructKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StructKind::Tuple(types) => {
                write!(f, "(")?;
                let mut first = true;
                for ty in types {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                    first = false;
                }
                writeln!(f, ");")
            }
            StructKind::Named(fields) => {
                writeln!(f, "{{")?;
                for field in fields {
                    writeln!(f, "    {},", field)?;
                }
                writeln!(f, "}}")
            }
        }
    }
}

impl Display for NamedField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}: {}", self.visibility, self.name, self.type_name)
    }
}

impl Display for EnumDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}enum {} {{", self.visibility, self.name)?;
        for variant in &self.variants {
            writeln!(f, "    {},", variant)?;
        }
        writeln!(f, "}}")
    }
}

impl Display for EnumVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.name, self.kind)
    }
}

impl Display for VariantKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariantKind::Unit => Ok(()),
            VariantKind::Tuple(types) => {
                write!(f, "(")?;
                let mut first = true;
                for ty in types {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                    first = false;
                }
                write!(f, ")")
            }
            VariantKind::Named(fields) => {
                write!(f, " {{")?;
                let mut first = true;
                for field in fields {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", field)?;
                    first = false;
                }
                write!(f, "}}")
            }
        }
    }
}

impl Display for ImplBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "impl {} {{", self.type_name)?;
        for predicate in &self.predicates {
            writeln!(f, "{}", predicate)?;
        }
        writeln!(f, "}}")
    }
}

impl Display for PredicateDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for attr in &self.attributes {
            writeln!(f, "{}", attr)?;
        }
        write!(f, "{}", self.visibility)?;
        write!(f, "rel {}(", self.name)?;
        for (i, p) in self.parameters.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", p)?;
        }
        write!(f, ")")?;
        if let Some(s) = &self.search_strategy {
            write!(f, " {}", s)?;
        }
        writeln!(f, " {{")?;
        for goal in &self.body {
            writeln!(f, "    {};", goal)?;
        }
        write!(f, "}}")
    }
}

impl Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}(", self.name)?;
        let mut first = true;
        for arg in &self.args {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
            first = false;
        }
        write!(f, ")")
    }
}

impl Display for AttributeArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttributeArg::Flag(name) => write!(f, "{}", name),
            AttributeArg::Named(name, value) => write!(f, "{} = {}", name, value),
        }
    }
}

impl Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(type_annotation) = &self.type_annotation {
            write!(f, ": {}", type_annotation)?;
        }
        Ok(())
    }
}

impl Display for SearchStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchStrategy::Bfs => write!(f, "bfs"),
            SearchStrategy::Dfs => write!(f, "dfs"),
        }
    }
}

impl Display for Goal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Goal::Let(g, _) => write!(f, "{}", g),
            Goal::Fresh(g, _) => write!(f, "{}", g),
            Goal::Disjunction(g, _) => write!(f, "{}", g),
            Goal::Conjunction(g, _) => write!(f, "{}", g),
            Goal::PatternMatch(g, _) => write!(f, "{}", g),
            Goal::RelationCall(g, _) => write!(f, "{}", g),
            Goal::MethodCall(g, _) => write!(f, "{}", g),
            Goal::Equality(a, b, _) => write!(f, "{} == {}", a, b),
            Goal::Disequality(a, b, _) => write!(f, "{} != {}", a, b),
            Goal::Parenthesized(g, _) => {
                write!(f, "(")?;
                for goal in g {
                    write!(f, "{}, ", goal)?;
                }
                write!(f, ")")
            }
            Goal::BooleanLiteral(b, _) => write!(f, "{}", b),
            Goal::ConstraintBlock(c, _) => {
                write!(
                    f,
                    "constraint {} {{ {} }}",
                    c.domain,
                    c.body.raw_content.trim()
                )
            }
            Goal::MetaStatement(meta, _) => write!(f, "{}", meta),
        }
    }
}

impl Display for LetDeclaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "let {}", self.var_name)?;
        if let Some(val) = &self.value {
            write!(f, " = {}", val)?;
        }
        Ok(())
    }
}

impl Display for FreshVariables {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "|{}| {{", self.vars.join(", "))?;
        for goal in &self.body {
            write!(f, "{}, ", goal)?;
        }
        write!(f, "}}")
    }
}

impl Display for Disjunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "any")?;
        if let Some(params) = &self.params {
            write!(f, "{}", params)?;
        }
        writeln!(f, " {{")?;
        for goal in &self.body {
            writeln!(f, "    {}", goal)?;
        }
        write!(f, "}}")
    }
}

impl Display for Conjunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "all")?;
        if let Some(params) = &self.params {
            write!(f, "{}", params)?;
        }
        writeln!(f, " {{")?;
        for goal in &self.body {
            writeln!(f, "    {}", goal)?;
        }
        write!(f, "}}")
    }
}

impl Display for PatternMatching {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "match {} {{", self.term)?;
        for arm in &self.arms {
            writeln!(f, "    {},", arm)?;
        }
        write!(f, "}}")
    }
}

impl Display for PatternArm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} => {{ ", self.pattern)?;
        for goal in &self.body {
            write!(f, "{}, ", goal)?;
        }
        write!(f, "}}")
    }
}

impl Display for RelationCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.name)?;
        let mut first = true;
        for arg in &self.args {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
            first = false;
        }
        write!(f, ")")
    }
}

impl Display for MethodCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}(", self.receiver, self.method)?;
        let mut first = true;
        for arg in &self.args {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
            first = false;
        }
        write!(f, ")")
    }
}

impl Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Literal(lit, _) => write!(f, "{}", lit),
            Term::Variable(v) => write!(f, "{}", v),
            Term::Wildcard(_) => write!(f, "_"),
            Term::List(list, _) => write!(f, "{}", list),
            Term::NamedStruct(s, _) => write!(f, "{}", s),
            Term::TupleStruct(c, _) => write!(f, "{}", c),
            Term::EnumVariant(e, _) => write!(f, "{}", e),
            Term::Parenthesized(t, _) => write!(f, "({})", t),
            Term::Interpolation(expr, _) => write!(f, "{{{}}}", expr),
        }
    }
}

impl Display for ListConstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut first = true;
        for elem in &self.elements {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", elem)?;
            first = false;
        }
        if let Some(tail) = &self.tail {
            write!(f, " | {}", tail)?;
        }
        write!(f, "]")
    }
}

impl Display for NamedStructConstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} {{", self.name)?;
        for field in &self.fields {
            writeln!(f, "    {},", field)?;
        }
        write!(f, "}}")
    }
}

impl Display for FieldInitializer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.value)
    }
}

impl Display for TupleStructConstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.name)?;
        let mut first = true;
        for arg in &self.args {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
            first = false;
        }
        write!(f, ")")
    }
}

impl Display for EnumVariantConstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.enum_name, self.variant_name)?;
        match &self.kind {
            EnumVariantConstructionKind::Unit => Ok(()),
            EnumVariantConstructionKind::Tuple(args) => {
                write!(f, "(")?;
                let mut first = true;
                for arg in args {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                    first = false;
                }
                write!(f, ")")
            }
            EnumVariantConstructionKind::Named(fields) => {
                write!(f, " {{ ")?;
                let mut first = true;
                for field in fields {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", field)?;
                    first = false;
                }
                write!(f, " }}")
            }
        }
    }
}


impl Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Boolean(b) => write!(f, "{}", b),
            Literal::Number(n) => write!(f, "{}", n),
            Literal::String(s) => write!(f, "\"{}\"", s),
            Literal::Char(c) => write!(f, "'{}'", c),
        }
    }
}

impl Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pattern::Literal(lit) => write!(f, "{}", lit),
            Pattern::Variable(v) => write!(f, "{}", v),
            Pattern::Wildcard => write!(f, "_"),
            Pattern::List(p) => write!(f, "{}", p),
            Pattern::NamedStruct(p) => write!(f, "{}", p),
            Pattern::TupleStruct(p) => write!(f, "{}", p),
            Pattern::EnumVariant(p) => write!(f, "{}", p),
        }
    }
}

impl Display for ListPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut first = true;
        for elem in &self.elements {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", elem)?;
            first = false;
        }
        if let Some(tail) = &self.tail {
            write!(f, " | {}", tail)?;
        }
        write!(f, "]")
    }
}

impl Display for NamedStructPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} {{", self.name)?;
        for field in &self.fields {
            writeln!(f, "    {},", field)?;
        }
        write!(f, "}}")
    }
}

impl Display for FieldPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.pattern)
    }
}

impl Display for TupleStructPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.name)?;
        let mut first = true;
        for arg in &self.args {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
            first = false;
        }
        write!(f, ")")
    }
}

impl Display for EnumVariantPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            EnumVariantPatternKind::Unit => write!(f, "{}::{}", self.enum_name, self.variant_name),
            EnumVariantPatternKind::Tuple(patterns) => {
                write!(f, "{}::{}(", self.enum_name, self.variant_name)?;
                let mut first = true;
                for pattern in patterns {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", pattern)?;
                    first = false;
                }
                write!(f, ")")
            }
            EnumVariantPatternKind::Named(field_patterns) => {
                write!(f, "{}::{} {{ ", self.enum_name, self.variant_name)?;
                let mut first = true;
                for field_pattern in field_patterns {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", field_pattern)?;
                    first = false;
                }
                write!(f, " }}")
            }
        }
    }
}

impl Display for ConstraintBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "constraint(domain = \"{}\") {{ {} }}",
            self.domain,
            self.body.raw_content.trim()
        )
    }
}

impl Display for ConstraintBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw_content)
    }
}

impl Display for CallArgument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallArgument::Term(term) => write!(f, "{}", term),
            CallArgument::MetaExpression(expr) => write!(f, "{}", expr),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_params_display() {
        let params = SearchParams::new()
            .with_strategy(SearchStrategy::Dfs)
            .with_limit(100)
            .with_depth(5)
            .with_custom_param(
                "mode".to_string().into(),
                SearchParamValue::String("exhaustive".to_string()),
            );

        assert_eq!(
            params.to_string(),
            "(strategy = dfs, limit = 100, depth = 5, mode = \"exhaustive\")"
        );
    }

    #[test]
    fn test_search_params_empty() {
        let params = SearchParams::new();
        assert_eq!(params.to_string(), "()");
    }

    #[test]
    fn test_search_params_single() {
        let params = SearchParams::new().with_strategy(SearchStrategy::Bfs);
        assert_eq!(params.to_string(), "(strategy = bfs)");
    }

    #[test]
    fn test_search_param_value_display() {
        assert_eq!(SearchParamValue::Number(42).to_string(), "42");
        assert_eq!(
            SearchParamValue::String("test".to_string()).to_string(),
            "\"test\""
        );
        assert_eq!(
            SearchParamValue::Identifier("var".to_string()).to_string(),
            "var"
        );
        assert_eq!(SearchParamValue::Boolean(true).to_string(), "true");
    }
}
