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
pub struct RelationDefinition {
    pub is_pub: bool,
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

#[derive(Debug, Clone, PartialEq)]
pub enum SearchStrategy {
    Bfs,
    Dfs,
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conjunction {
    pub body: GoalBody,
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
            writeln!(f, "    {}", rel)?;
        }
        writeln!(f, "}}")
    }
}

impl Display for RelationDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_pub {
            write!(f, "pub ")?;
        }
        write!(f, "rel {}(", self.name)?;
        let mut first = true;
        for param in &self.parameters {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
            first = false;
        }
        write!(f, ")")?;
        if let Some(ss) = &self.search_strategy {
            write!(f, " {}", ss)?;
        }
        writeln!(f, " {{")?;
        for goal in &self.body {
            writeln!(f, "    {},", goal)?;
        }
        writeln!(f, "}}")
    }
}

impl Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(ty) = &self.type_name {
            write!(f, ": {}", ty)?;
        }
        Ok(())
    }
}

impl Display for SearchStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchStrategy::Bfs => write!(f, "@bfs"),
            SearchStrategy::Dfs => write!(f, "@dfs"),
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
        writeln!(f, "conde {{")?;
        for goal in &self.body {
            writeln!(f, "    {},", goal)?;
        }
        write!(f, "}}")
    }
}

impl Display for Conjunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for goal in &self.body {
            write!(f, "{}, ", goal)?;
        }
        write!(f, "]")
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
