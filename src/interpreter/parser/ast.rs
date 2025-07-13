use std::fmt::{self, Display};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Use(UseStatement),
    Module(ModuleDefinition),
    Struct(StructDefinition),
    Impl(ImplBlock),
    Relation(RelationDefinition),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseStatement {
    pub path: UsePath,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UsePath {
    Simple(Vec<String>),
    Glob(Vec<String>),
    List(Vec<String>, Vec<(String, Option<String>)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDefinition {
    pub name: String,
    pub search_strategy: Option<SearchStrategy>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDefinition {
    pub is_pub: bool,
    pub name: String,
    pub kind: StructKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StructKind {
    Tuple(Vec<String>),
    Named(Vec<NamedField>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedField {
    pub is_pub: bool,
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplBlock {
    pub type_name: String,
    pub relations: Vec<RelationDefinition>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeArg {
    Flag(String),
    Named(String, Term),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<AttributeArg>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationDefinition {
    pub is_pub: bool,
    pub attributes: Vec<Attribute>,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub search_strategy: Option<SearchStrategy>,
    pub body: GoalBody,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub type_name: Option<String>,
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

    pub fn with_custom_param(mut self, name: String, value: SearchParamValue) -> Self {
        self.custom_params.push((name, value));
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

#[derive(Debug, Clone, PartialEq)]
pub enum Goal {
    Let(LetDeclaration),
    Fresh(FreshVariables),
    Disjunction(Disjunction),
    Conjunction(Conjunction),
    PatternMatch(PatternMatching),
    RelationCall(RelationCall),
    MethodCall(MethodCall),
    Equality(Term, Term),
    Disequality(Term, Term),
    Parenthesized(GoalBody),
    BooleanLiteral(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetDeclaration {
    pub var_name: String,
    pub value: Option<Term>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FreshVariables {
    pub vars: Vec<String>,
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
pub struct RelationCall {
    pub name: String,
    pub args: Vec<Term>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodCall {
    pub receiver: Box<Term>,
    pub method: String,
    pub args: Vec<Term>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Literal(Literal),
    Variable(String),
    Wildcard,
    List(ListConstruction),
    NamedStruct(NamedStructConstruction),
    Compound(CompoundConstruction),
    Parenthesized(Box<Term>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListConstruction {
    pub elements: Vec<Term>,
    pub tail: Option<Box<Term>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedStructConstruction {
    pub name: String,
    pub fields: Vec<FieldInitializer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldInitializer {
    pub name: String,
    pub value: Term,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompoundConstruction {
    pub name: String,
    pub args: Vec<Term>,
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
    Variable(String),
    Wildcard,
    List(ListPattern),
    NamedStruct(NamedStructPattern),
    Compound(CompoundPattern),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListPattern {
    pub elements: Vec<Pattern>,
    pub tail: Option<Box<Pattern>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedStructPattern {
    pub name: String,
    pub fields: Vec<FieldPattern>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldPattern {
    pub name: String,
    pub pattern: Pattern,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompoundPattern {
    pub name: String,
    pub args: Vec<Pattern>,
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
            Item::Module(m) => write!(f, "{}", m),
            Item::Struct(s) => write!(f, "{}", s),
            Item::Impl(i) => write!(f, "{}", i),
            Item::Relation(r) => write!(f, "{}", r),
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
            UsePath::Simple(parts) => write!(f, "{}", parts.join("::")),
            UsePath::Glob(parts) => write!(f, "{}::*", parts.join("::")),
            UsePath::List(prefix, imports) => {
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
                if prefix.is_empty() {
                    write!(f, "{{{}}}", import_str)
                } else {
                    write!(f, "{}::{{{}}}", prefix.join("::"), import_str)
                }
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
        if self.is_pub {
            write!(f, "pub ")?;
        }
        write!(f, "struct {} ", self.name)?;
        write!(f, "{}", self.kind)
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
        if self.is_pub {
            write!(f, "pub ")?;
        }
        write!(f, "{}: {}", self.name, self.type_name)
    }
}

impl Display for ImplBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "impl {} {{", self.type_name)?;
        for rel in &self.relations {
            writeln!(f, "{}", rel)?;
        }
        writeln!(f, "}}")
    }
}

impl Display for RelationDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for attr in &self.attributes {
            writeln!(f, "{}", attr)?;
        }
        if self.is_pub {
            write!(f, "pub ")?;
        }
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
        if let Some(type_name) = &self.type_name {
            write!(f, ": {}", type_name)?;
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
            Goal::Let(g) => write!(f, "{}", g),
            Goal::Fresh(g) => write!(f, "{}", g),
            Goal::Disjunction(g) => write!(f, "{}", g),
            Goal::Conjunction(g) => write!(f, "{}", g),
            Goal::PatternMatch(g) => write!(f, "{}", g),
            Goal::RelationCall(g) => write!(f, "{}", g),
            Goal::MethodCall(g) => write!(f, "{}", g),
            Goal::Equality(a, b) => write!(f, "{} == {}", a, b),
            Goal::Disequality(a, b) => write!(f, "{} != {}", a, b),
            Goal::Parenthesized(g) => {
                write!(f, "(")?;
                for goal in g {
                    write!(f, "{}, ", goal)?;
                }
                write!(f, ")")
            }
            Goal::BooleanLiteral(b) => write!(f, "{}", b),
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
            Term::Literal(lit) => write!(f, "{}", lit),
            Term::Variable(v) => write!(f, "{}", v),
            Term::Wildcard => write!(f, "_"),
            Term::List(list) => write!(f, "{}", list),
            Term::NamedStruct(s) => write!(f, "{}", s),
            Term::Compound(c) => write!(f, "{}", c),
            Term::Parenthesized(t) => write!(f, "({})", t),
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

impl Display for CompoundConstruction {
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
            Pattern::Compound(p) => write!(f, "{}", p),
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

impl Display for CompoundPattern {
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
                "mode".to_string(),
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
