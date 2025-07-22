//! This module is responsible for the runtime execution of the interpreter.
//! It takes the AST from the parser and converts it into a series of executable
//! goals that the solver can understand and process.
//!
//! The core component is the `ExecutionContext`, which manages the state of
//! execution, including variable scopes and relation lookups.

use super::deferred::DeferredRelationCall;
use super::environment::{Environment, TypeDefinition};
use super::metaprogramming::{
    expand_meta_statement, expand_term, MetaValue,
    TemplateExpansionContext,
};
use super::parser::ast::{
    Conjunction as AstConjunction, EnumVariantPatternKind, Goal as AstGoal, Literal, Pattern, PatternMatching,
    RelationCall, SearchStrategy, StructKind, Term,
};
use super::runtime_value::RuntimeValue;
use super::InterpreterError;
use crate::compound::{CompoundObject, CompoundWalkStar};
use crate::engine::Engine;
use crate::goal::{AnyGoal, Goal, GoalCast};
use crate::lterm::LTerm;
use crate::operator::conde::Conde;
use crate::operator::conj::Conj;

use crate::relation::eq;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;


/// Registry-based tuple struct that references type definitions by index
#[derive(Clone)]
pub struct RegistryTupleStruct<U: User, E: Engine<U>> {
    pub type_index: usize,
    pub args: Vec<LTerm<U, E>>,
    pub environment: Rc<RefCell<crate::interpreter::environment::Environment<U, E>>>,
}


impl<U: User, E: Engine<U>> std::fmt::Debug for RegistryTupleStruct<U, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RegistryTupleStruct(type_index={}, args=", self.type_index)?;
        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", arg)?;
        }
        write!(f, ")")
    }
}

impl<U: User, E: Engine<U>> CompoundObject<U, E> for RegistryTupleStruct<U, E> {
    fn type_name(&self) -> String {
        let env_ref = self.environment.borrow();
        env_ref.get_type_by_index(self.type_index)
            .and_then(|type_def| match type_def {
                crate::interpreter::environment::TypeDefinition::Struct(s) => Some(s.name.to_string()),
                _ => None,
            })
            .unwrap_or_else(|| "TupleStruct".to_string())
    }
    
    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject<U, E>> + 'a> {
        Box::new(self.args.iter().map(|arg| arg as &dyn CompoundObject<U, E>))
    }

    fn display_string(&self) -> String {
        let type_name = self.type_name();
        let arg_strings: Vec<String> = self.args.iter().map(|arg| {
            if let crate::lterm::LTermInner::Compound(compound) = arg.as_ref() {
                compound.display_string()
            } else {
                format!("{}", arg)
            }
        }).collect();
        format!("{}({})", type_name, arg_strings.join(", "))
    }
}

impl<U: User, E: Engine<U>> CompoundWalkStar<U, E> for RegistryTupleStruct<U, E> {
    fn compound_walk_star(&self, smap: &crate::state::SMap<U, E>) -> Self {
        Self {
            type_index: self.type_index,
            args: self.args.iter().map(|arg| arg.compound_walk_star(smap)).collect(),
            environment: self.environment.clone(),
        }
    }
}


impl<U: User, E: Engine<U>> PartialEq for RegistryTupleStruct<U, E> {
    fn eq(&self, other: &Self) -> bool {
        self.type_index == other.type_index && self.args == other.args
    }
}

impl<U: User, E: Engine<U>> Eq for RegistryTupleStruct<U, E> {}

impl<U: User, E: Engine<U>> std::hash::Hash for RegistryTupleStruct<U, E> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.type_index.hash(state);
        self.args.hash(state);
    }
}

impl<U: User, E: Engine<U>> Into<LTerm<U, E>> for RegistryTupleStruct<U, E> {
    fn into(self) -> LTerm<U, E> {
        LTerm::from(Rc::new(self) as Rc<dyn CompoundObject<U, E>>)
    }
}

/// Registry-based named struct that references type definitions by index
#[derive(Clone)]
pub struct RegistryNamedStruct<U: User, E: Engine<U>> {
    pub type_index: usize,
    pub fields: std::collections::HashMap<String, LTerm<U, E>>,
    pub environment: Rc<RefCell<crate::interpreter::environment::Environment<U, E>>>,
}


impl<U: User, E: Engine<U>> std::fmt::Debug for RegistryNamedStruct<U, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RegistryNamedStruct(type_index={}, fields={{", self.type_index)?;
        let mut first = true;
        for (field_name, field_value) in &self.fields {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}: {:?}", field_name, field_value)?;
            first = false;
        }
        write!(f, "}})")
    }
}

impl<U: User, E: Engine<U>> CompoundObject<U, E> for RegistryNamedStruct<U, E> {
    fn type_name(&self) -> String {
        let env_ref = self.environment.borrow();
        env_ref.get_type_by_index(self.type_index)
            .and_then(|type_def| match type_def {
                crate::interpreter::environment::TypeDefinition::Struct(s) => Some(s.name.to_string()),
                _ => None,
            })
            .unwrap_or_else(|| "NamedStruct".to_string())
    }
    
    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject<U, E>> + 'a> {
        // Sort fields by name to ensure deterministic iteration order
        let mut sorted_fields: Vec<_> = self.fields.iter().collect();
        sorted_fields.sort_by_key(|(name, _)| *name);
        Box::new(sorted_fields.into_iter().map(|(_, field)| field as &dyn CompoundObject<U, E>))
    }

    fn display_string(&self) -> String {
        let type_name = self.type_name();
        let mut sorted_fields: Vec<_> = self.fields.iter().collect();
        sorted_fields.sort_by_key(|(name, _)| *name);
        let field_strings: Vec<String> = sorted_fields.iter().map(|(name, value)| {
            let value_str = if let crate::lterm::LTermInner::Compound(compound) = value.as_ref() {
                compound.display_string()
            } else {
                format!("{}", value)
            };
            format!("{}: {}", name, value_str)
        }).collect();
        format!("{} {{ {} }}", type_name, field_strings.join(", "))
    }
}

impl<U: User, E: Engine<U>> CompoundWalkStar<U, E> for RegistryNamedStruct<U, E> {
    fn compound_walk_star(&self, smap: &crate::state::SMap<U, E>) -> Self {
        let mut walked_fields = std::collections::HashMap::new();
        for (name, value) in &self.fields {
            walked_fields.insert(name.clone(), value.compound_walk_star(smap));
        }
        Self {
            type_index: self.type_index,
            fields: walked_fields,
            environment: self.environment.clone(),
        }
    }
}


impl<U: User, E: Engine<U>> PartialEq for RegistryNamedStruct<U, E> {
    fn eq(&self, other: &Self) -> bool {
        self.type_index == other.type_index && self.fields == other.fields
    }
}

impl<U: User, E: Engine<U>> Eq for RegistryNamedStruct<U, E> {}

impl<U: User, E: Engine<U>> std::hash::Hash for RegistryNamedStruct<U, E> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.type_index.hash(state);
        // HashMap doesn't implement Hash, so we'll sort the fields first
        let mut sorted_fields: Vec<_> = self.fields.iter().collect();
        sorted_fields.sort_by_key(|(k, _)| *k);
        for (key, value) in sorted_fields {
            key.hash(state);
            value.hash(state);
        }
    }
}

impl<U: User, E: Engine<U>> Into<LTerm<U, E>> for RegistryNamedStruct<U, E> {
    fn into(self) -> LTerm<U, E> {
        LTerm::from(Rc::new(self) as Rc<dyn CompoundObject<U, E>>)
    }
}

/// Registry-based enum variant that references type definitions by index
#[derive(Clone)]
pub struct RegistryEnumVariant<U: User, E: Engine<U>> {
    pub enum_type_index: usize,
    pub variant_index: usize,
    pub variant_name: String,
    pub variant_data: VariantData<U, E>,
    pub environment: Rc<RefCell<crate::interpreter::environment::Environment<U, E>>>,
}

/// Data contained in an enum variant
#[derive(Clone)]
pub enum VariantData<U: User, E: Engine<U>> {
    Unit,                                               // Color::Red
    Tuple(Vec<LTerm<U, E>>),                           // Option::Some(42)
    Named(std::collections::HashMap<String, LTerm<U, E>>), // Person::Named { name: "John", age: 30 }
}

impl<U: User, E: Engine<U>> RegistryEnumVariant<U, E> {
    /// Get the enum name from an environment if available
    pub fn get_enum_name(&self, environment: Option<&crate::interpreter::environment::Environment<U, E>>) -> String {
        if let Some(env) = environment {
            if let Some(type_def) = env.get_type_by_index(self.enum_type_index) {
                if let crate::interpreter::environment::TypeDefinition::Enum(enum_def) = type_def {
                    return enum_def.name.to_string();
                }
            }
        }
        // Fallback to a generic name
        format!("Enum{}", self.enum_type_index)
    }
}


impl<U: User, E: Engine<U>> std::fmt::Debug for RegistryEnumVariant<U, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RegistryEnumVariant(enum_type_index={}, variant_name={}, data=", 
               self.enum_type_index, self.variant_name)?;
        match &self.variant_data {
            VariantData::Unit => write!(f, "Unit"),
            VariantData::Tuple(args) => {
                write!(f, "Tuple(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", arg)?;
                }
                write!(f, ")")
            }
            VariantData::Named(fields) => {
                write!(f, "Named({{")?;
                let mut first = true;
                for (field_name, field_value) in fields {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {:?}", field_name, field_value)?;
                    first = false;
                }
                write!(f, "}})")
            }
        }?;
        write!(f, ")")
    }
}

impl<U: User, E: Engine<U>> std::fmt::Debug for VariantData<U, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariantData::Unit => write!(f, "Unit"),
            VariantData::Tuple(args) => {
                write!(f, "Tuple(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", arg)?;
                }
                write!(f, ")")
            }
            VariantData::Named(fields) => {
                write!(f, "Named({{")?;
                let mut first = true;
                for (field_name, field_value) in fields {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {:?}", field_name, field_value)?;
                    first = false;
                }
                write!(f, "}})")
            }
        }
    }
}

impl<U: User, E: Engine<U>> CompoundObject<U, E> for RegistryEnumVariant<U, E> {
    fn type_name(&self) -> String {
        let env_ref = self.environment.borrow();
        env_ref.get_type_by_index(self.enum_type_index)
            .and_then(|type_def| match type_def {
                crate::interpreter::environment::TypeDefinition::Enum(e) => Some(e.name.to_string()),
                _ => None,
            })
            .unwrap_or_else(|| "Enum".to_string())
    }
    
    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject<U, E>> + 'a> {
        match &self.variant_data {
            VariantData::Unit => Box::new(std::iter::empty()),
            VariantData::Tuple(args) => {
                Box::new(args.iter().map(|arg| arg as &dyn CompoundObject<U, E>))
            }
            VariantData::Named(fields) => {
                // Sort fields by name to ensure deterministic iteration order
                let mut sorted_fields: Vec<_> = fields.iter().collect();
                sorted_fields.sort_by_key(|(name, _)| *name);
                Box::new(sorted_fields.into_iter().map(|(_, field)| field as &dyn CompoundObject<U, E>))
            }
        }
    }


    fn is_enum_variant(&self) -> bool {
        true
    }

    fn variant_index(&self) -> Option<usize> {
        Some(self.variant_index)
    }

    fn variant_name(&self) -> Option<String> {
        Some(self.variant_name.clone())
    }

    fn type_registry_index(&self) -> Option<usize> {
        Some(self.enum_type_index)
    }

    fn display_string(&self) -> String {
        let enum_name = self.type_name();
        
        match &self.variant_data {
            VariantData::Unit => format!("{}::{}", enum_name, self.variant_name),
            VariantData::Tuple(args) => {
                let arg_strings: Vec<String> = args.iter().map(|arg| {
                    if let crate::lterm::LTermInner::Compound(compound) = arg.as_ref() {
                        compound.display_string()
                    } else {
                        format!("{}", arg)
                    }
                }).collect();
                format!("{}::{}({})", enum_name, self.variant_name, arg_strings.join(", "))
            }
            VariantData::Named(fields) => {
                let mut sorted_fields: Vec<_> = fields.iter().collect();
                sorted_fields.sort_by_key(|(name, _)| *name);
                let field_strings: Vec<String> = sorted_fields.iter().map(|(name, value)| {
                    let value_str = if let crate::lterm::LTermInner::Compound(compound) = value.as_ref() {
                        compound.display_string()
                    } else {
                        format!("{}", value)
                    };
                    format!("{}: {}", name, value_str)
                }).collect();
                format!("{}::{} {{ {} }}", enum_name, self.variant_name, field_strings.join(", "))
            }
        }
    }
}

impl<U: User, E: Engine<U>> CompoundWalkStar<U, E> for RegistryEnumVariant<U, E> {
    fn compound_walk_star(&self, smap: &crate::state::SMap<U, E>) -> Self {
        let walked_data = match &self.variant_data {
            VariantData::Unit => VariantData::Unit,
            VariantData::Tuple(args) => {
                VariantData::Tuple(args.iter().map(|arg| arg.compound_walk_star(smap)).collect())
            }
            VariantData::Named(fields) => {
                let mut walked_fields = std::collections::HashMap::new();
                for (name, value) in fields {
                    walked_fields.insert(name.clone(), value.compound_walk_star(smap));
                }
                VariantData::Named(walked_fields)
            }
        };
        
        Self {
            enum_type_index: self.enum_type_index,
            variant_index: self.variant_index,
            variant_name: self.variant_name.clone(),
            variant_data: walked_data,
            environment: self.environment.clone(),
        }
    }
}


impl<U: User, E: Engine<U>> PartialEq for RegistryEnumVariant<U, E> {
    fn eq(&self, other: &Self) -> bool {
        self.enum_type_index == other.enum_type_index 
            && self.variant_index == other.variant_index
            && self.variant_data == other.variant_data
    }
}

impl<U: User, E: Engine<U>> PartialEq for VariantData<U, E> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (VariantData::Unit, VariantData::Unit) => true,
            (VariantData::Tuple(a), VariantData::Tuple(b)) => a == b,
            (VariantData::Named(a), VariantData::Named(b)) => a == b,
            _ => false,
        }
    }
}

impl<U: User, E: Engine<U>> Eq for RegistryEnumVariant<U, E> {}
impl<U: User, E: Engine<U>> Eq for VariantData<U, E> {}

impl<U: User, E: Engine<U>> std::hash::Hash for RegistryEnumVariant<U, E> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.enum_type_index.hash(state);
        self.variant_index.hash(state);
        self.variant_data.hash(state);
    }
}

impl<U: User, E: Engine<U>> std::hash::Hash for VariantData<U, E> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            VariantData::Unit => 0u8.hash(state),
            VariantData::Tuple(args) => {
                1u8.hash(state);
                args.hash(state);
            }
            VariantData::Named(fields) => {
                2u8.hash(state);
                // HashMap doesn't implement Hash, so we'll sort the fields first
                let mut sorted_fields: Vec<_> = fields.iter().collect();
                sorted_fields.sort_by_key(|(k, _)| *k);
                for (key, value) in sorted_fields {
                    key.hash(state);
                    value.hash(state);
                }
            }
        }
    }
}

impl<U: User, E: Engine<U>> Into<LTerm<U, E>> for RegistryEnumVariant<U, E> {
    fn into(self) -> LTerm<U, E> {
        LTerm::from(Rc::new(self) as Rc<dyn CompoundObject<U, E>>)
    }
}

/// A custom disjunction that uses DFS (depth-first search) semantics
/// This works within the BFS Goal<U, E> framework but uses DFS stream operations
#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
struct DFSDisjunction<U, E>
where
    U: User,
    E: Engine<U>,
{
    goals: Vec<Goal<U, E>>,
}

impl<U, E> DFSDisjunction<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn new(goals: Vec<Goal<U, E>>) -> Goal<U, E> {
        Goal::dynamic(Rc::new(DFSDisjunction { goals }))
    }
}

impl<U, E> Solve<U, E> for DFSDisjunction<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        // Implement DFS by exploring the first goal completely before moving to the next
        // This is the opposite of the interleaving behavior in BFS
        let mut stream = Stream::empty();

        // Process goals in reverse order, using DFS stream operations
        for goal in self.goals.iter().rev() {
            let new_stream = goal.solve(solver, state.clone());
            // Use mplus_dfs to get depth-first semantics
            stream = Stream::mplus_dfs(new_stream, LazyStream::delay(stream));
        }

        stream
    }
}

/// Represents a variable value that can be either relational (for logic computation)
/// or non-relational (for meta programming)
#[derive(Debug)]
pub enum VariableValue<U: User, E: Engine<U>> {
    /// Relational variable for logic computation, unification, etc.
    Relational(LTerm<U, E>),
    /// Non-relational variable for meta programming (integers, strings, booleans)
    Meta(MetaValue),
}

impl<U: User, E: Engine<U>> Clone for VariableValue<U, E> {
    fn clone(&self) -> Self {
        match self {
            VariableValue::Relational(lterm) => VariableValue::Relational(lterm.clone()),
            VariableValue::Meta(meta) => VariableValue::Meta(meta.clone()),
        }
    }
}

impl<U: User, E: Engine<U>> VariableValue<U, E> {
    /// Try to get this as an LTerm for relational operations
    pub fn as_lterm(&self) -> Option<&LTerm<U, E>> {
        match self {
            VariableValue::Relational(lterm) => Some(lterm),
            VariableValue::Meta(_) => None,
        }
    }

    /// Try to get this as a MetaValue for template expansion
    pub fn as_meta(&self) -> Option<&MetaValue> {
        match self {
            VariableValue::Relational(_) => None,
            VariableValue::Meta(meta) => Some(meta),
        }
    }

    /// Convert to LTerm if possible (for backward compatibility)
    pub fn to_lterm(&self) -> Option<LTerm<U, E>> {
        match self {
            VariableValue::Relational(lterm) => Some(lterm.clone()),
            VariableValue::Meta(meta) => {
                // Convert meta values to LTerms only when absolutely necessary
                match meta {
                    MetaValue::Integer(i) => Some(LTerm::from(*i as isize)),
                    MetaValue::String(s) => Some(LTerm::from(s.clone())),
                    MetaValue::Boolean(b) => Some(LTerm::from(*b)),
                }
            }
        }
    }
}

/// The `ExecutionContext` is the primary state manager for the interpreter's
/// runtime. It holds a reference to the broader `Environment` (which contains
/// all relation and module definitions) and, most importantly, manages the
/// lexical scope of variables.
///
/// The `locals` field is a stack of hashmaps, where each map represents a
/// lexical scope. When a new scope is entered (e.g., a relation call or a
/// match arm), a new map is pushed onto the stack. When the scope is exited,
/// the map is popped off. This ensures that variables are correctly scoped
/// and do not leak into parent scopes.
pub struct ExecutionContext<'a, U: User, E: Engine<U>> {
    /// A mutable reference to the global environment, containing all loaded relations.
    pub environment: Rc<RefCell<Environment<U, E>>>,

    /// A stack of scopes for local variables. Each scope is a `HashMap` from a
    /// variable name (String) to its corresponding variable value (either relational or meta).
    pub locals: Vec<HashMap<String, VariableValue<U, E>>>,

    /// A list of goals that need to be executed as part of the current goal's conjunction.
    /// This is used for complex operations that create intermediate goals, like arithmetic.
    deferred_goals: Vec<Goal<U, E>>,

    /// A counter to ensure that every fresh variable created has a unique ID.
    var_counter: usize,

    /// Current search strategy context - tracks whether we're in BFS or DFS mode
    /// This is used to enforce embedding rules: BFS cannot be embedded in DFS
    search_strategy_stack: Vec<SearchStrategy>,

    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, U: User, E: Engine<U>> ExecutionContext<'a, U, E> {
    /// Creates a new `ExecutionContext`.
    pub fn new(environment: Rc<RefCell<Environment<U, E>>>) -> Self {
        Self {
            environment,
            locals: vec![HashMap::new()], // Start with one base scope
            deferred_goals: vec![],
            var_counter: 0,
            // Start with BFS as the default search strategy
            search_strategy_stack: vec![SearchStrategy::Bfs],
            _phantom: std::marker::PhantomData,
        }
    }

    /// Creates a new, unique logical variable (`LTerm::Var`).
    pub fn create_fresh_var(&mut self) -> LTerm<U, E> {
        let var = LTerm::any(); // Use anonymous variables internally
        self.var_counter += 1;
        var
    }

    /// Pushes a new, empty scope onto the locals stack.
    pub fn push_scope(&mut self) {
        self.locals.push(HashMap::new());
    }

    /// Pops the current scope from the locals stack.
    pub fn pop_scope(&mut self) {
        self.locals.pop();
    }

    /// Searches for a variable in the current scope stack.
    /// It looks from the innermost scope outwards, returning only relational variables as LTerms.
    fn lookup_var<N: AsRef<str>>(&self, name: N) -> Option<LTerm<U, E>> {
        self.lookup_variable_value(name)?.to_lterm()
    }

    /// Searches for a variable value (relational or meta) in the current scope stack.
    fn lookup_variable_value<N: AsRef<str>>(&self, name: N) -> Option<VariableValue<U, E>> {
        for scope in self.locals.iter().rev() {
            if let Some(var) = scope.get(name.as_ref()) {
                return Some(var.clone());
            }
        }
        None
    }

    /// Searches for a meta variable in the current scope stack.
    fn lookup_meta_var<N: AsRef<str>>(&self, name: N) -> Option<MetaValue> {
        self.lookup_variable_value(name)?.as_meta().cloned()
    }

    /// Searches for a variable only in the current (innermost) scope.
    /// Used for pattern matching to ensure variables within a pattern are unified.
    fn lookup_var_current_scope<N: AsRef<str>>(&self, name: N) -> Option<LTerm<U, E>> {
        if let Some(scope) = self.locals.last() {
            scope.get(name.as_ref()).and_then(|var| var.to_lterm())
        } else {
            None
        }
    }

    /// Binds a variable name to an `LTerm` in the current (innermost) scope.
    pub fn bind_var<N: AsRef<str>>(&mut self, name: N, var: LTerm<U, E>) {
        self.bind_variable_value(name, VariableValue::Relational(var));
    }

    /// Binds a variable name to a `MetaValue` in the current (innermost) scope.
    pub fn bind_meta_var<N: AsRef<str>>(&mut self, name: N, meta: MetaValue) {
        self.bind_variable_value(name, VariableValue::Meta(meta));
    }

    /// Binds a variable name to a `VariableValue` in the current (innermost) scope.
    pub fn bind_variable_value<N: AsRef<str>>(&mut self, name: N, value: VariableValue<U, E>) {
        if let Some(scope) = self.locals.last_mut() {
            scope.insert(name.as_ref().to_string(), value);
        }
    }

    /// Returns the top-level variable bindings (converted to LTerms for backward compatibility).
    pub fn get_variable_bindings(&self) -> HashMap<String, LTerm<U, E>> {
        self.locals
            .first()
            .map(|scope| {
                scope
                    .iter()
                    .filter_map(|(name, value)| value.to_lterm().map(|lterm| (name.clone(), lterm)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns the top-level variable values (both relational and meta).
    pub fn get_all_variable_bindings(&self) -> HashMap<String, VariableValue<U, E>> {
        self.locals.first().cloned().unwrap_or_default()
    }

    /// Gets an existing variable by name, returns an error if it doesn't exist.
    /// This is useful for constraint domains that should only reference existing variables.
    pub fn get_existing_variable(&self, name: &str) -> Result<LTerm<U, E>, InterpreterError> {
        self.lookup_var(name)
            .ok_or_else(|| InterpreterError::UnknownVariable(name.to_string()))
    }
    
    /// Resolve a type name to a registry index
    fn resolve_type_to_index<S: AsRef<str>>(&self, name: S) -> Result<usize, InterpreterError> {
        // Look up the type name as a symbol
        let name_str = name.as_ref();
        let runtime_value = self.environment.borrow().lookup(name_str)
            .ok_or_else(|| InterpreterError::UnknownType(name_str.to_string()))?
            .clone();
        
        match runtime_value {
            RuntimeValue::Type(index) => Ok(index),
            _ => Err(InterpreterError::NotAType(name_str.to_string())),
        }
    }

    /// Adds a goal to the list of deferred goals to be executed.
    pub fn add_deferred_goal(&mut self, goal: Goal<U, E>) {
        self.deferred_goals.push(goal);
    }

    /// Get the current search strategy (top of stack)
    pub fn current_search_strategy(&self) -> SearchStrategy {
        self.search_strategy_stack
            .last()
            .copied()
            .unwrap_or(SearchStrategy::Bfs)
    }

    /// Push a new search strategy onto the context stack
    pub fn push_search_strategy(&mut self, strategy: SearchStrategy) {
        self.search_strategy_stack.push(strategy);
    }

    /// Pop the current search strategy from the context stack
    pub fn pop_search_strategy(&mut self) {
        if self.search_strategy_stack.len() > 1 {
            self.search_strategy_stack.pop();
        }
        // Always keep at least one strategy (BFS default)
    }

    /// Validate that a search strategy can be used in the current context
    /// BFS cannot be embedded in DFS, but DFS can be embedded in BFS
    pub fn validate_search_strategy(
        &self,
        requested_strategy: SearchStrategy,
    ) -> Result<(), InterpreterError> {
        let current = self.current_search_strategy();
        match (current, requested_strategy) {
            (SearchStrategy::Dfs, SearchStrategy::Bfs) => {
                Err(InterpreterError::IllegalSearchStrategyEmbedding {
                    attempted: "BFS".to_string(),
                    current_context: "DFS".to_string(),
                })
            }
            _ => Ok(()),
        }
    }

    /// Validates that all symbols referenced in a relation body can be found
    /// This catches UnknownRelation errors early, before deferred execution
    fn validate_relation_body_symbols(
        &self,
        _body: &[super::parser::ast::Goal],
    ) -> Result<(), InterpreterError> {
        // For now, skip validation to avoid issues with relation parameters
        // TODO: Implement proper validation that's aware of relation parameters
        Ok(())
    }

    /// Recursively validates symbols in a goal
    fn validate_goal_symbols(
        &self,
        goal: &super::parser::ast::Goal,
    ) -> Result<(), InterpreterError> {
        use super::parser::ast::Goal as AstGoal;

        match goal {
            AstGoal::RelationCall(call, _) => {
                // Check if the relation exists (either regular relation or relation handle)
                let env = self.environment.borrow();
                let rel_val = env.lookup_relation(&call.name)?.ok_or_else(|| {
                    InterpreterError::UnknownRelation(call.name.name().to_string())
                })?;

                // Validate that it's actually a callable relation
                match rel_val {
                    RuntimeValue::Relation(_)
                    | RuntimeValue::PredicateHandle(_)
                    | RuntimeValue::BuiltinRelation { .. } => Ok(()),
                    _ => Err(InterpreterError::RuntimeError(format!(
                        "'{}' is not a relation or relation handle.",
                        call.name
                    ))),
                }
            }
            AstGoal::Conjunction(conj, _) => {
                for g in &conj.body {
                    self.validate_goal_symbols(g)?;
                }
                Ok(())
            }
            AstGoal::Disjunction(disj, _) => {
                for g in &disj.body {
                    self.validate_goal_symbols(g)?;
                }
                Ok(())
            }
            AstGoal::PatternMatch(pattern_match, _) => {
                for arm in &pattern_match.arms {
                    for g in &arm.body {
                        self.validate_goal_symbols(g)?;
                    }
                }
                Ok(())
            }
            AstGoal::Parenthesized(body, _) => {
                for g in body {
                    self.validate_goal_symbols(g)?;
                }
                Ok(())
            }
            AstGoal::Let(_, _) => {
                // Let declarations don't contain relation calls to validate
                Ok(())
            }
            AstGoal::Fresh(_, _) => {
                // Fresh variable declarations don't contain relation calls to validate
                Ok(())
            }
            // These goal types don't contain relation calls
            AstGoal::Equality(_, _, _)
            | AstGoal::Disequality(_, _, _)
            | AstGoal::BooleanLiteral(..)
            | AstGoal::MethodCall(..)
            | AstGoal::ConstraintBlock(..) => Ok(()),
            AstGoal::MetaStatement(..) => {
                // TODO: Implement meta statement validation
                Ok(())
            }
        }
    }

    /// This is the main entry point for converting an AST goal into a runtime goal
    /// that can be solved. It dispatches to the appropriate helper function based
    /// on the AST goal type.
    pub fn ast_goal_to_runtime(&mut self, goal: &AstGoal) -> Result<Goal<U, E>, InterpreterError> {
        // Clear any deferred goals from a previous run
        self.deferred_goals.clear();

        let main_goal = match goal {
            AstGoal::Equality(lhs, rhs, _) => {
                let lhs_term = self.ast_term_to_runtime(lhs)?;
                let rhs_term = self.ast_term_to_runtime(rhs)?;
                Ok(eq(lhs_term, rhs_term).cast_into())
            }
            AstGoal::Disequality(lhs, rhs, _) => {
                let lhs_term = self.ast_term_to_runtime(lhs)?;
                let rhs_term = self.ast_term_to_runtime(rhs)?;
                Ok(crate::relation::diseq::diseq(lhs_term, rhs_term).cast_into())
            }
            AstGoal::RelationCall(call, _) => self.ast_relation_call_to_runtime(call),
            AstGoal::Conjunction(conj, _) => {
                // Check if any goals in the body are meta statements or contain interpolation
                let has_meta_features = conj
                    .body
                    .iter()
                    .any(|g| self.goal_contains_meta_features(g));

                if has_meta_features {
                    // Use template-aware processing for goal bodies with meta statements
                    self.process_goal_body_with_template_expansion(&conj.body)
                } else {
                    // Use regular processing for goal bodies without meta statements
                    let mut goals = vec![];
                    for g in &conj.body {
                        goals.push(self.ast_goal_to_runtime(g)?);
                    }
                    if goals.is_empty() {
                        Ok(Goal::succeed())
                    } else {
                        let mut iter = goals.into_iter();
                        let first = iter.next().unwrap();
                        Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
                    }
                }
            }
            AstGoal::Disjunction(disj, _) => {
                let mut goals = vec![];
                for g in &disj.body {
                    goals.push(self.ast_goal_to_runtime(g)?);
                }

                // Determine the search strategy to use for this disjunction
                let strategy = if let Some(params) = &disj.params {
                    // If explicit strategy is specified in the any block, use that
                    params
                        .strategy
                        .unwrap_or_else(|| self.current_search_strategy())
                } else {
                    // No params, inherit strategy from current context (e.g., relation-level @dfs)
                    self.current_search_strategy()
                };

                match strategy {
                    SearchStrategy::Dfs => {
                        // Use DFS disjunction for depth-first search
                        Ok(DFSDisjunction::new(goals))
                    }
                    SearchStrategy::Bfs => {
                        // Use regular BFS disjunction (Conde)
                        Ok(Conde::from_array(&goals).cast_into())
                    }
                }
            }
            AstGoal::PatternMatch(pm, _) => self.ast_pattern_match_to_runtime(pm),
            AstGoal::Parenthesized(goals, _) => {
                // Check if any goals in the body are meta statements or contain interpolation
                let has_meta_features = goals.iter().any(|g| self.goal_contains_meta_features(g));

                if has_meta_features {
                    // Use template-aware processing for goal bodies with meta statements
                    self.process_goal_body_with_template_expansion(goals)
                } else {
                    // Use regular processing for goal bodies without meta statements
                    let mut conj_goals = Vec::new();
                    for g in goals {
                        conj_goals.push(self.ast_goal_to_runtime(g)?);
                    }
                    if conj_goals.is_empty() {
                        Ok(Goal::succeed())
                    } else {
                        let mut iter = conj_goals.into_iter();
                        let first = iter.next().unwrap();
                        Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
                    }
                }
            }
            AstGoal::Let(let_decl, _) => {
                let value_term = if let Some(val) = &let_decl.value {
                    self.ast_term_to_runtime(val)?
                } else {
                    self.create_fresh_var()
                };
                self.bind_var(let_decl.var_name.clone(), value_term);
                Ok(Goal::succeed())
            }
            AstGoal::Fresh(fresh_vars, _) => {
                self.push_scope();
                for var_name in &fresh_vars.vars {
                    let fresh_var = self.create_fresh_var();
                    self.bind_var(var_name.clone(), fresh_var);
                }

                let mut goals = Vec::new();
                for g in &fresh_vars.body {
                    goals.push(self.ast_goal_to_runtime(g)?);
                }

                self.pop_scope();
                if goals.is_empty() {
                    Ok(Goal::succeed())
                } else {
                    let mut iter = goals.into_iter();
                    let first = iter.next().unwrap();
                    Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
                }
            }
            AstGoal::MethodCall(..) => Err(InterpreterError::RuntimeError(
                "Method calls not implemented yet".to_string(),
            )),
            AstGoal::ConstraintBlock(block, span) => self.convert_constraint_block(block, span),
            AstGoal::BooleanLiteral(val, _) => {
                if *val {
                    Ok(Goal::succeed())
                } else {
                    Ok(Goal::fail())
                }
            }
            AstGoal::MetaStatement(meta_stmt, span) => {
                // Expand the meta statement into concrete goals using template expansion
                self.expand_and_execute_meta_statement(meta_stmt, span)
            }
        }?;

        if self.deferred_goals.is_empty() {
            Ok(main_goal)
        } else {
            let mut all_goals = vec![main_goal];
            all_goals.extend(self.deferred_goals.drain(..));
            let mut iter = all_goals.into_iter();
            let first = iter.next().unwrap();
            Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
        }
    }

    /// Converts an AST conjunction into a runtime goal.
    /// It respects the search strategy specified in the `all` block.
    fn ast_conjunction_to_runtime(
        &mut self,
        conj: &AstConjunction,
    ) -> Result<Goal<U, E>, InterpreterError> {
        let mut goals = vec![];
        for g in &conj.body {
            goals.push(self.ast_goal_to_runtime(g)?);
        }
        if goals.is_empty() {
            Ok(Goal::succeed())
        } else {
            let mut iter = goals.into_iter();
            let first = iter.next().unwrap();
            Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
        }
    }

    /// Converts an AST disjunction into a runtime goal.
    /// It respects the search strategy specified in the `any` block.
    fn ast_disjunction_to_runtime(
        &mut self,
        disj: &super::parser::ast::Disjunction,
    ) -> Result<Goal<U, E>, InterpreterError> {
        let mut goals = vec![];
        for g in &disj.body {
            goals.push(self.ast_goal_to_runtime(g)?);
        }
        Ok(Conde::from_array(&goals).cast_into())
    }

    /// Private helper to handle constraint blocks.
    fn convert_constraint_block(
        &mut self,
        block: &super::parser::ast::ConstraintBlock,
        span: &super::parser::ast::Span,
    ) -> Result<Goal<U, E>, InterpreterError> {
        use super::constraint_domains::ConstraintDomainRegistry;
        let registry = ConstraintDomainRegistry::default();
        let domain = registry
            .get_domain(&block.domain)
            .ok_or_else(|| InterpreterError::UnknownConstraintDomain(block.domain.clone()))?;
        let parsed_constraints = domain.parse_constraints(&block.body, span)?;
        parsed_constraints.convert_to_goals(self)
    }

    /// Converts an AST term to a runtime LTerm.
    pub fn ast_term_to_runtime(&mut self, term: &Term) -> Result<LTerm<U, E>, InterpreterError> {
        match term {
            Term::Variable(name) => {
                // First check if it's a regular variable
                if let Some(var) = self.lookup_var(name) {
                    return Ok(var);
                }

                // Check if it refers to a relation for higher-order use
                let rel_val_opt = self.environment.borrow().lookup(name).cloned();
                if let Some(rel_val) = rel_val_opt {
                    match rel_val {
                        RuntimeValue::Relation(_)
                        | RuntimeValue::PredicateHandle(_)
                        | RuntimeValue::BuiltinRelation { .. } => {
                            // Register the relation in the registry and return a reference to the index
                            let registry_index =
                                self.environment.borrow_mut().register_relation(rel_val);
                            return Ok(LTerm::relation_ref(registry_index));
                        }
                        _ => {}
                    }
                }

                Err(InterpreterError::UnknownVariable(name.to_string()))
            }
            Term::Wildcard(_) => Ok(LTerm::any()),
            Term::Literal(literal, _) => convert_ast_literal_to_runtime(literal),
            Term::List(list, _) => {
                let mut elements = Vec::new();
                for el in &list.elements {
                    elements.push(self.ast_term_to_runtime(el)?);
                }

                let tail = match &list.tail {
                    Some(tail_term) => Some(self.ast_term_to_runtime(tail_term)?),
                    None => None,
                };

                Ok(lterm_from_vec_and_tail(elements, tail))
            }
            Term::Parenthesized(inner, _) => self.ast_term_to_runtime(inner),
            Term::NamedStruct(named_struct, _) => {
                // Named struct construction
                let type_index = self.resolve_type_to_index(&named_struct.name)?;
                
                // Verify it's a named struct
                let env = self.environment.borrow();
                match env.get_type_by_index(type_index) {
                    Some(TypeDefinition::Struct(def)) => {
                        if !matches!(def.kind, StructKind::Named(_)) {
                            return Err(InterpreterError::RuntimeError(
                                format!("{} is not a named struct", named_struct.name)
                            ));
                        }
                    }
                    _ => return Err(InterpreterError::NotAType(named_struct.name.to_string())),
                }
                drop(env);
                
                // Convert fields
                let mut fields = std::collections::HashMap::new();
                for field in &named_struct.fields {
                    let field_value = self.ast_term_to_runtime(&field.value)?;
                    fields.insert(field.name.to_string(), field_value);
                }
                
                let registry_struct = RegistryNamedStruct { type_index, fields, environment: self.environment.clone() };
                Ok(LTerm::from(Rc::new(registry_struct) as Rc<dyn CompoundObject<U, E>>))
            }
            Term::TupleStruct(compound, _) => {
                // Tuple struct construction
                let type_index = self.resolve_type_to_index(&compound.name.to_string())?;
                
                // Verify it's a tuple struct
                let env = self.environment.borrow();
                match env.get_type_by_index(type_index) {
                    Some(TypeDefinition::Struct(def)) => {
                        if !matches!(def.kind, StructKind::Tuple(_)) {
                            return Err(InterpreterError::RuntimeError(
                                format!("{} is not a tuple struct", compound.name)
                            ));
                        }
                    }
                    _ => return Err(InterpreterError::NotAType(compound.name.to_string())),
                }
                drop(env);
                
                // Convert arguments
                let mut args = Vec::new();
                for arg in &compound.args {
                    args.push(self.ast_term_to_runtime(arg)?);
                }
                
                let registry_struct = RegistryTupleStruct { type_index, args, environment: self.environment.clone() };
                Ok(LTerm::from(Rc::new(registry_struct) as Rc<dyn CompoundObject<U, E>>))
            }
            Term::Interpolation(expr, _) => {
                // Expand interpolation using template expansion context
                self.expand_and_evaluate_interpolation(expr)
            }
            Term::EnumVariant(enum_variant, _) => {
                // Handle enum variant construction
                self.ast_enum_variant_to_runtime(enum_variant)
            }
        }
    }

    /// Converts an AST enum variant to a runtime LTerm
    pub fn ast_enum_variant_to_runtime(&mut self, enum_variant: &super::parser::ast::EnumVariantConstruction) -> Result<LTerm<U, E>, InterpreterError> {
        // Resolve enum type
        let enum_type_index = self.resolve_type_to_index(&enum_variant.enum_name)?;
        
        // Verify it's an enum and the variant exists
        let (variant_kind, variant_name, variant_index) = {
            let env = self.environment.borrow();
            let enum_def = match env.get_type_by_index(enum_type_index) {
                Some(TypeDefinition::Enum(def)) => def,
                Some(_) => return Err(InterpreterError::RuntimeError(
                    format!("{} is not an enum", enum_variant.enum_name)
                )),
                None => return Err(InterpreterError::RuntimeError(
                    format!("Unknown enum type: {}", enum_variant.enum_name)
                )),
            };
            
            // Find the variant in the enum definition and its index
            let (variant_index, variant_def) = enum_def.variants.iter()
                .enumerate()
                .find(|(_, v)| v.name == enum_variant.variant_name)
                .ok_or_else(|| InterpreterError::RuntimeError(
                    format!("Unknown variant {} for enum {}", 
                           enum_variant.variant_name, enum_variant.enum_name)
                ))?;
            
            (variant_def.kind.clone(), variant_def.name.clone(), variant_index)
        }; // Drop the borrow here
        
        // Create the variant data based on the construction kind
        let variant_data = match (&enum_variant.kind, &variant_kind) {
            (super::parser::ast::EnumVariantConstructionKind::Unit, super::parser::ast::VariantKind::Unit) => {
                VariantData::Unit
            }
            (super::parser::ast::EnumVariantConstructionKind::Tuple(args), super::parser::ast::VariantKind::Tuple(_)) => {
                let mut runtime_args = Vec::new();
                for arg in args {
                    runtime_args.push(self.ast_term_to_runtime(arg)?);
                }
                VariantData::Tuple(runtime_args)
            }
            (super::parser::ast::EnumVariantConstructionKind::Named(fields), super::parser::ast::VariantKind::Named(_)) => {
                let mut runtime_fields = std::collections::HashMap::new();
                for field in fields {
                    let field_value = self.ast_term_to_runtime(&field.value)?;
                    runtime_fields.insert(field.name.to_string(), field_value);
                }
                VariantData::Named(runtime_fields)
            }
            _ => return Err(InterpreterError::RuntimeError(
                format!("Variant construction kind does not match enum definition for {}::{}", 
                       enum_variant.enum_name, enum_variant.variant_name)
            )),
        };
        
        let enum_variant_obj = RegistryEnumVariant {
            enum_type_index,
            variant_index,
            variant_name: variant_name.to_string(),
            variant_data,
            environment: self.environment.clone(),
        };
        
        Ok(LTerm::from(Rc::new(enum_variant_obj) as Rc<dyn CompoundObject<U, E>>))
    }

    pub fn ast_call_argument_to_runtime(
        &mut self,
        arg: &super::parser::ast::CallArgument,
    ) -> Result<LTerm<U, E>, InterpreterError> {
        use super::parser::ast::CallArgument;

        match arg {
            CallArgument::Term(term) => self.ast_term_to_runtime(term),
            CallArgument::MetaExpression(_) => {
                // Meta expressions should not be converted to LTerms
                // They should be handled during macro template expansion
                Err(InterpreterError::RuntimeError(
                    "Meta expressions can only be used in macro calls and should be handled during template expansion".to_string()
                ))
            }
        }
    }

    fn ast_relation_call_to_runtime(
        &mut self,
        call: &RelationCall,
    ) -> Result<Goal<U, E>, InterpreterError> {
        // First check if the relation name refers to a variable containing a relation reference
        if let Some(var_term) = self.lookup_var(call.name.name()) {
            if var_term.is_relation_ref() {
                // The variable contains a relation reference - resolve it from the registry
                if let Some(registry_index) = var_term.get_relation_ref() {
                    let rel_val = self
                        .environment
                        .borrow()
                        .get_relation_by_index(registry_index)
                        .cloned()
                        .ok_or_else(|| {
                            InterpreterError::UnknownRelation(format!(
                                "registry_index_{}",
                                registry_index
                            ))
                        })?;

                    // Convert call-site arguments to LTerms.
                    let mut arg_terms = Vec::new();
                    for arg in &call.args {
                        arg_terms.push(self.ast_call_argument_to_runtime(arg)?);
                    }

                    return self.handle_relation_value(
                        rel_val,
                        format!("registry_index_{}", registry_index),
                        arg_terms,
                    );
                }
            }
        }

        // Regular relation lookup
        let rel_val = self
            .environment
            .borrow()
            .lookup_relation(&call.name)?
            .cloned()
            .ok_or_else(|| InterpreterError::UnknownRelation(call.name.name().to_string()))?;

        // Check if this is a macro predicate - if so, handle call arguments directly
        if let RuntimeValue::Relation(ref rel_def) = rel_val {
            if matches!(
                rel_def.predicate_kind,
                super::parser::ast::PredicateKind::Macro
            ) {
                // For macro calls, handle CallArguments directly to preserve meta expressions
                return self.process_macro_call_with_arguments(rel_def, &call.args);
            }
        }

        // Regular relation - convert arguments to LTerms
        let mut arg_terms = Vec::new();
        for arg in &call.args {
            arg_terms.push(self.ast_call_argument_to_runtime(arg)?);
        }

        self.handle_relation_value(rel_val, call.name.name().to_string(), arg_terms)
    }

    fn handle_relation_value(
        &mut self,
        rel_val: RuntimeValue<U, E>,
        relation_name: String,
        arg_terms: Vec<LTerm<U, E>>,
    ) -> Result<Goal<U, E>, InterpreterError> {
        match rel_val {
            RuntimeValue::Relation(rel_def) => {
                if rel_def.parameters.len() != arg_terms.len() {
                    return Err(InterpreterError::ArityMismatch {
                        relation_name: relation_name.clone(),
                        expected: rel_def.parameters.len(),
                        actual: arg_terms.len(),
                    });
                }

                // Check if this is a macro predicate - if so, use eager expansion
                if matches!(
                    rel_def.predicate_kind,
                    super::parser::ast::PredicateKind::Macro
                ) {
                    // Eager expansion for macro predicates
                    // Validate that all symbols in the relation body can be found
                    self.validate_relation_body_symbols(&rel_def.body)?;

                    // Create a scope for the macro expansion
                    self.push_scope();

                    // Bind parameters for template expansion
                    for (param, arg) in rel_def.parameters.iter().zip(arg_terms.iter()) {
                        self.bind_var(param.name.clone(), arg.clone());
                    }

                    // Eagerly expand the macro body
                    let expanded_goal =
                        self.process_macro_body_with_parameter_binding(&rel_def, &arg_terms)?;

                    // Pop the scope
                    self.pop_scope();

                    Ok(expanded_goal)
                } else {
                    // Regular relation - use deferred execution
                    // Validate that all symbols in the relation body can be found
                    // This catches UnknownRelation errors early, before deferred execution
                    self.validate_relation_body_symbols(&rel_def.body)?;

                    let deferred_call = DeferredRelationCall::new(
                        self.environment.clone(),
                        rel_def.into(),
                        arg_terms,
                        self.current_search_strategy(),
                    );

                    Ok(Goal::Dynamic(Rc::new(deferred_call)))
                }
            }
            RuntimeValue::PredicateHandle(handle) => {
                // Higher-order predicate call: relation parameter being invoked
                if handle.arity != arg_terms.len() {
                    return Err(InterpreterError::ArityMismatch {
                        relation_name: relation_name.clone(),
                        expected: handle.arity,
                        actual: arg_terms.len(),
                    });
                }

                // Validate that all symbols in the relation body can be found
                self.validate_relation_body_symbols(&handle.definition.body)?;

                // Create a deferred call using the relation definition from the handle
                let deferred_call = DeferredRelationCall::new(
                    self.environment.clone(),
                    handle.definition.clone().into(),
                    arg_terms,
                    self.current_search_strategy(),
                );

                Ok(Goal::Dynamic(Rc::new(deferred_call)))
            }
            RuntimeValue::BuiltinRelation { func, arity } => {
                if arity != arg_terms.len() {
                    return Err(InterpreterError::ArityMismatch {
                        relation_name: relation_name.clone(),
                        expected: arity,
                        actual: arg_terms.len(),
                    });
                }
                Ok(func(arg_terms))
            }
            _ => Err(InterpreterError::RuntimeError(format!(
                "'{}' is not a relation or relation handle.",
                relation_name
            ))),
        }
    }

    fn ast_pattern_match_to_runtime(
        &mut self,
        pattern_match: &PatternMatching,
    ) -> Result<Goal<U, E>, InterpreterError> {
        // 1. Convert the term to be matched.
        let term_to_match = self.ast_term_to_runtime(&pattern_match.term)?;

        let mut arm_goals = Vec::new();

        // 2. Iterate through each arm.
        for arm in &pattern_match.arms {
            // 3a. Push a new scope for the arm.
            self.push_scope();

            // 3b. Build the pattern LTerm, binding fresh variables.
            let pattern_lterm = self.convert_pattern_to_lterm(&arm.pattern)?;

            // 3c. Create the unification goal.
            let unification_goal: Goal<U, E> = eq(term_to_match.clone(), pattern_lterm).cast_into();

            // 3d. Convert the body to a goal.
            let mut body_conj = Goal::succeed();
            for goal in arm.body.iter().rev() {
                let runtime_goal = self.ast_goal_to_runtime(goal)?;
                body_conj = Conj::new(runtime_goal, body_conj);
            }

            // 3e. Create the final arm goal.
            let arm_goal = Conj::new(unification_goal, body_conj);

            // 3f. Pop scope.
            self.pop_scope();

            arm_goals.push(arm_goal);
        }

        // 4. Combine all arms in a disjunction (conde).
        Ok(Conde::from_array(&arm_goals).cast_into())
    }

    fn convert_pattern_to_lterm(
        &mut self,
        pattern: &Pattern,
    ) -> Result<LTerm<U, E>, InterpreterError> {
        match pattern {
            Pattern::Variable(name) => {
                // Check if the variable already exists in the current scope
                if let Some(existing_var) = self.lookup_var_current_scope(name) {
                    Ok(existing_var)
                } else {
                    let var = self.create_fresh_var();
                    self.bind_var(name.clone(), var.clone());
                    Ok(var)
                }
            }
            Pattern::Wildcard => Ok(LTerm::any()),
            Pattern::Literal(lit) => convert_ast_literal_to_runtime(lit),
            Pattern::List(list) => {
                let mut elements = Vec::new();
                for el in &list.elements {
                    elements.push(self.convert_pattern_to_lterm(el)?);
                }

                let tail = match &list.tail {
                    Some(tail_pattern) => Some(self.convert_pattern_to_lterm(tail_pattern)?),
                    None => None,
                };

                Ok(lterm_from_vec_and_tail(elements, tail))
            }
            Pattern::NamedStruct(named_struct_pattern) => {
                // Named struct pattern
                let type_index = self.resolve_type_to_index(&named_struct_pattern.name)?;
                
                // Convert field patterns
                let mut field_patterns = std::collections::HashMap::new();
                for field_pattern in &named_struct_pattern.fields {
                    let field_term = self.convert_pattern_to_lterm(&field_pattern.pattern)?;
                    field_patterns.insert(field_pattern.name.to_string(), field_term);
                }
                
                let registry_struct = RegistryNamedStruct { type_index, fields: field_patterns, environment: self.environment.clone() };
                Ok(LTerm::from(Rc::new(registry_struct) as Rc<dyn CompoundObject<U, E>>))
            }
            Pattern::TupleStruct(compound_pattern) => {
                // Tuple struct pattern
                let type_index = self.resolve_type_to_index(&compound_pattern.name)?;
                
                // Convert pattern arguments
                let mut arg_patterns = Vec::new();
                for arg_pattern in &compound_pattern.args {
                    let arg_term = self.convert_pattern_to_lterm(arg_pattern)?;
                    arg_patterns.push(arg_term);
                }
                
                let registry_struct = RegistryTupleStruct { type_index, args: arg_patterns, environment: self.environment.clone() };
                Ok(LTerm::from(Rc::new(registry_struct) as Rc<dyn CompoundObject<U, E>>))
            }
            Pattern::EnumVariant(enum_variant_pattern) => {
                // Enum variant pattern
                let type_index = self.resolve_type_to_index(&enum_variant_pattern.enum_name)?;
                
                // Find the variant index
                let env = self.environment.borrow();
                let variant_index = if let Some(TypeDefinition::Enum(enum_def)) = env.get_type_by_index(type_index) {
                    enum_def.variants.iter().enumerate()
                        .find(|(_, variant)| variant.name == enum_variant_pattern.variant_name)
                        .map(|(index, _)| index)
                        .ok_or_else(|| InterpreterError::RuntimeError(
                            format!("Variant {} not found in enum {}", enum_variant_pattern.variant_name, enum_variant_pattern.enum_name)
                        ))?
                } else {
                    return Err(InterpreterError::RuntimeError(
                        format!("{} is not an enum", enum_variant_pattern.enum_name)
                    ));
                };
                drop(env);
                
                // Convert variant data based on kind
                let variant_data = match &enum_variant_pattern.kind {
                    EnumVariantPatternKind::Unit => VariantData::Unit,
                    EnumVariantPatternKind::Tuple(patterns) => {
                        let mut runtime_patterns = Vec::new();
                        for pattern in patterns {
                            runtime_patterns.push(self.convert_pattern_to_lterm(pattern)?);
                        }
                        VariantData::Tuple(runtime_patterns)
                    }
                    EnumVariantPatternKind::Named(field_patterns) => {
                        let mut runtime_field_patterns = std::collections::HashMap::new();
                        for field_pattern in field_patterns {
                            let field_term = self.convert_pattern_to_lterm(&field_pattern.pattern)?;
                            runtime_field_patterns.insert(field_pattern.name.to_string(), field_term);
                        }
                        VariantData::Named(runtime_field_patterns)
                    }
                };
                
                let enum_variant_obj = RegistryEnumVariant { 
                    enum_type_index: type_index, 
                    variant_index,
                    variant_name: enum_variant_pattern.variant_name.to_string(),
                    variant_data,
                    environment: self.environment.clone(),
                };
                Ok(LTerm::from(Rc::new(enum_variant_obj) as Rc<dyn CompoundObject<U, E>>))
            }
        }
    }

    /// Check if a goal contains meta features (meta statements or interpolation)
    pub fn goal_contains_meta_features(&self, goal: &super::parser::ast::Goal) -> bool {
        use super::parser::ast::{Goal as AstGoal};

        match goal {
            AstGoal::MetaStatement(..) => true,
            AstGoal::Equality(lhs, rhs, _) => {
                self.term_contains_interpolation(lhs) || self.term_contains_interpolation(rhs)
            }
            AstGoal::Disequality(lhs, rhs, _) => {
                self.term_contains_interpolation(lhs) || self.term_contains_interpolation(rhs)
            }
            AstGoal::RelationCall(call, _) => call
                .args
                .iter()
                .any(|arg| self.call_argument_contains_interpolation(arg)),
            AstGoal::Conjunction(conj, _) => conj
                .body
                .iter()
                .any(|g| self.goal_contains_meta_features(g)),
            AstGoal::Disjunction(disj, _) => disj
                .body
                .iter()
                .any(|g| self.goal_contains_meta_features(g)),
            AstGoal::Parenthesized(goals, _) => {
                goals.iter().any(|g| self.goal_contains_meta_features(g))
            }
            AstGoal::Fresh(fresh, _) => fresh
                .body
                .iter()
                .any(|g| self.goal_contains_meta_features(g)),
            AstGoal::PatternMatch(pm, _) => pm
                .arms
                .iter()
                .any(|arm| arm.body.iter().any(|g| self.goal_contains_meta_features(g))),
            _ => false,
        }
    }

    /// Check if a term contains interpolation expressions
    fn term_contains_interpolation(&self, term: &super::parser::ast::Term) -> bool {
        use super::parser::ast::Term;

        match term {
            Term::Interpolation(..) => true,
            Term::List(list, _) => {
                list.elements
                    .iter()
                    .any(|el| self.term_contains_interpolation(el))
                    || list
                        .tail
                        .as_ref()
                        .map_or(false, |tail| self.term_contains_interpolation(tail))
            }
            Term::Parenthesized(inner, _) => self.term_contains_interpolation(inner),
            Term::NamedStruct(named_struct, _) => named_struct
                .fields
                .iter()
                .any(|field| self.term_contains_interpolation(&field.value)),
            Term::TupleStruct(compound, _) => compound
                .args
                .iter()
                .any(|arg| self.term_contains_interpolation(arg)),
            _ => false,
        }
    }

    /// Check if a call argument contains interpolation expressions
    fn call_argument_contains_interpolation(&self, arg: &super::parser::ast::CallArgument) -> bool {
        use super::parser::ast::CallArgument;

        match arg {
            CallArgument::Term(term) => self.term_contains_interpolation(term),
            CallArgument::MetaExpression(_) => true, // Meta expressions are always considered to contain interpolation
        }
    }

    /// Process a goal body with template expansion awareness
    /// This ensures meta variables from let statements are available to subsequent statements
    pub fn process_goal_body_with_template_expansion(
        &mut self,
        goals: &[super::parser::ast::Goal],
    ) -> Result<Goal<U, E>, InterpreterError> {
        use super::metaprogramming::{expand_goal_body, TemplateExpansionContext};

        // Create a shared template expansion context for all goals in this body
        let mut template_context = TemplateExpansionContext::new(100);

        // First, expand all goals using the shared template context
        let goals_vec = goals.to_vec();
        let expanded_goals = expand_goal_body(&goals_vec, &mut template_context).map_err(|e| {
            InterpreterError::RuntimeError(format!("Template expansion error: {}", e))
        })?;

        // Convert all expanded goals to runtime goals
        let mut runtime_goals = Vec::new();
        for goal in &expanded_goals {
            runtime_goals.push(self.ast_goal_to_runtime(goal)?);
        }

        // Combine all runtime goals into a single conjunction
        if runtime_goals.is_empty() {
            Ok(Goal::succeed())
        } else {
            let mut iter = runtime_goals.into_iter();
            let first = iter.next().unwrap();
            Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
        }
    }

    /// Process a macro predicate body with parameter binding for template expansion
    pub fn process_macro_body_with_parameter_binding(
        &mut self,
        rel_def: &super::parser::ast::PredicateDefinition,
        call_args: &[LTerm<U, E>],
    ) -> Result<Goal<U, E>, InterpreterError> {
        use super::metaprogramming::{
            expand_goal_body, MetaValue, TemplateExpansionContext, TypeAnnotation,
        };

        // Create template expansion context
        let mut template_context = TemplateExpansionContext::new(100);

        // Bind non-relational parameters as meta variables
        for (param, arg_term) in rel_def.parameters.iter().zip(call_args.iter()) {
            if let Some(type_annotation) = &param.type_annotation {
                match type_annotation {
                    TypeAnnotation::Int => {
                        // Extract integer value from the argument term
                        if let Some(number) = arg_term.get_number() {
                            template_context
                                .bind(param.name.to_string(), MetaValue::Integer(number as i64));
                        } else {
                            return Err(InterpreterError::RuntimeError(format!(
                                "Macro parameter '{}' expects integer value, got: {:?}",
                                param.name, arg_term
                            )));
                        }
                    }
                    TypeAnnotation::String => {
                        // Extract string value from the argument term
                        if let crate::lterm::LTermInner::Val(crate::lvalue::LValue::String(
                            string_val,
                        )) = arg_term.as_ref()
                        {
                            template_context
                                .bind(param.name.to_string(), MetaValue::String(string_val.clone()));
                        } else {
                            return Err(InterpreterError::RuntimeError(format!(
                                "Macro parameter '{}' expects string value, got: {:?}",
                                param.name, arg_term
                            )));
                        }
                    }
                    TypeAnnotation::Bool => {
                        // Extract boolean value from the argument term
                        if let Some(bool_val) = arg_term.get_bool() {
                            template_context.bind(param.name.to_string(), MetaValue::Boolean(bool_val));
                        } else {
                            return Err(InterpreterError::RuntimeError(format!(
                                "Macro parameter '{}' expects boolean value, got: {:?}",
                                param.name, arg_term
                            )));
                        }
                    }
                    TypeAnnotation::Relation(_) => {
                        // Relational parameters are not bound as meta variables - they're handled normally
                        // They should have been bound during the regular parameter binding process
                    }
                    TypeAnnotation::Custom(_) => {
                        // Custom type parameters are not bound as meta variables - they're handled normally
                        // They should have been bound during the regular parameter binding process
                    }
                }
            }
            // Untyped parameters are also handled normally, not as meta variables
        }

        // Expand the macro body using the template context with bound parameters
        let goals_vec = rel_def.body.to_vec();
        let expanded_goals = expand_goal_body(&goals_vec, &mut template_context).map_err(|e| {
            InterpreterError::RuntimeError(format!("Template expansion error: {}", e))
        })?;

        // Convert all expanded goals to runtime goals
        let mut runtime_goals = Vec::new();
        for goal in &expanded_goals {
            runtime_goals.push(self.ast_goal_to_runtime(goal)?);
        }

        // Combine all runtime goals into a single conjunction
        if runtime_goals.is_empty() {
            Ok(Goal::succeed())
        } else {
            let mut iter = runtime_goals.into_iter();
            let first = iter.next().unwrap();
            Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
        }
    }

    /// Process a macro call with its arguments, handling meta expressions directly
    fn process_macro_call_with_arguments(
        &mut self,
        rel_def: &super::parser::ast::PredicateDefinition,
        args: &[super::parser::ast::CallArgument],
    ) -> Result<Goal<U, E>, InterpreterError> {
        use super::metaprogramming::{
            evaluate_meta_expression, expand_goal_body, MetaValue, TemplateExpansionContext,
            TypeAnnotation,
        };

        // Validate that all symbols in the relation body can be found
        self.validate_relation_body_symbols(&rel_def.body)?;

        // Create a scope for the macro expansion
        self.push_scope();

        // Create a template expansion context for the macro call
        let mut template_context = TemplateExpansionContext::new(100);

        // Process all parameters - bind appropriately based on type and argument
        for (param, arg) in rel_def.parameters.iter().zip(args.iter()) {
            match (&param.type_annotation, arg) {
                // Typed non-relational parameters
                (
                    Some(
                        TypeAnnotation::Int
                        | TypeAnnotation::String
                        | TypeAnnotation::Bool
                        | TypeAnnotation::Custom(_),
                    ),
                    _,
                ) => {
                    // For non-relational typed parameters, bind as meta variables for template expansion
                    let meta_value = match arg {
                        super::parser::ast::CallArgument::Term(term) => {
                            // Extract meta value from literal terms
                            match term {
                                super::parser::ast::Term::Literal(literal, _) => {
                                    match literal {
                                        super::parser::ast::Literal::Number(num_str) => {
                                            match num_str.parse::<i64>() {
                                                Ok(num) => MetaValue::Integer(num),
                                                Err(_) => return Err(InterpreterError::RuntimeError(format!(
                                                    "Invalid integer literal for parameter '{}': {}",
                                                    param.name, num_str
                                                ))),
                                            }
                                        }
                                        super::parser::ast::Literal::String(s) => MetaValue::String(s.clone()),
                                        super::parser::ast::Literal::Boolean(b) => MetaValue::Boolean(*b),
                                        _ => return Err(InterpreterError::RuntimeError(format!(
                                            "Literal type not supported for non-relational parameter '{}'",
                                            param.name
                                        ))),
                                    }
                                }
                                super::parser::ast::Term::Variable(var_name) => {
                                    // Look up variable value and convert to meta value if possible
                                    if let Some(var_value) = self.lookup_variable_value(var_name) {
                                        match var_value.as_meta() {
                                            Some(meta) => meta.clone(),
                                            None => {
                                                // Try to extract from LTerm if it's a relational variable
                                                if let Some(lterm) = var_value.as_lterm() {
                                                    match param.type_annotation.as_ref().unwrap() {
                                                        TypeAnnotation::Int => {
                                                            if let Some(n) = lterm.get_number() {
                                                                MetaValue::Integer(n as i64)
                                                            } else {
                                                                return Err(InterpreterError::RuntimeError(format!(
                                                                    "Variable '{}' is not an integer for parameter '{}'",
                                                                    var_name, param.name
                                                                )));
                                                            }
                                                        }
                                                        TypeAnnotation::String => {
                                                            if let Some(s) = lterm.get_name() {
                                                                MetaValue::String(s.to_string())
                                                            } else {
                                                                return Err(InterpreterError::RuntimeError(format!(
                                                                    "Variable '{}' is not a string for parameter '{}'",
                                                                    var_name, param.name
                                                                )));
                                                            }
                                                        }
                                                        TypeAnnotation::Bool => {
                                                            if let Some(b) = lterm.get_bool() {
                                                                MetaValue::Boolean(b)
                                                            } else {
                                                                return Err(InterpreterError::RuntimeError(format!(
                                                                    "Variable '{}' is not a boolean for parameter '{}'",
                                                                    var_name, param.name
                                                                )));
                                                            }
                                                        }
                                                        _ => unreachable!(),
                                                    }
                                                } else {
                                                    return Err(InterpreterError::RuntimeError(format!(
                                                        "Variable '{}' cannot be used for non-relational parameter '{}'",
                                                        var_name, param.name
                                                    )));
                                                }
                                            }
                                        }
                                    } else {
                                        return Err(InterpreterError::RuntimeError(format!(
                                            "Unbound variable '{}' for parameter '{}'",
                                            var_name, param.name
                                        )));
                                    }
                                }
                                _ => return Err(InterpreterError::RuntimeError(format!(
                                    "Only literals and variables are allowed for non-relational parameter '{}'",
                                    param.name
                                ))),
                            }
                        }
                        super::parser::ast::CallArgument::MetaExpression(expr) => {
                            // Evaluate meta expression
                            evaluate_meta_expression(expr, &template_context.bindings).map_err(
                                |e| {
                                    InterpreterError::RuntimeError(format!(
                                        "Meta expression evaluation failed for parameter '{}': {}",
                                        param.name, e
                                    ))
                                },
                            )?
                        }
                    };

                    // Bind as meta variable for template expansion
                    self.bind_meta_var(param.name.clone(), meta_value);
                }

                // Relational typed parameters
                (Some(TypeAnnotation::Relation(_)), arg) => {
                    match arg {
                        super::parser::ast::CallArgument::Term(term) => {
                            // Relational parameter - bind as relational variable
                            let runtime_arg = self.ast_term_to_runtime(term)?;
                            self.bind_var(param.name.clone(), runtime_arg);
                        }
                        super::parser::ast::CallArgument::MetaExpression(_) => {
                            return Err(InterpreterError::RuntimeError(format!(
                                "Meta expressions cannot be used for relational parameters ({})",
                                param.name
                            )));
                        }
                    }
                }

                // Untyped parameters
                (None, arg) => {
                    match arg {
                        super::parser::ast::CallArgument::Term(term) => {
                            // Untyped parameter - bind as relational variable
                            let runtime_arg = self.ast_term_to_runtime(term)?;
                            self.bind_var(param.name.clone(), runtime_arg);
                        }
                        super::parser::ast::CallArgument::MetaExpression(_) => {
                            return Err(InterpreterError::RuntimeError(format!(
                                "Meta expressions require typed parameters for macro calls. Parameter '{}' needs a type annotation (int, string, bool)",
                                param.name
                            )));
                        }
                    }
                }
            }

            // Now handle template expansion context binding for typed parameters
            if let Some(type_annotation) = &param.type_annotation {
                match type_annotation {
                    TypeAnnotation::Int
                    | TypeAnnotation::String
                    | TypeAnnotation::Bool
                    | TypeAnnotation::Custom(_) => {
                        // Only bind meta variables (non-relational) for template expansion
                        // Do NOT convert LTerms to meta values - maintain strict separation
                        if let Some(meta_value) = self.lookup_meta_var(&param.name) {
                            template_context.bind(param.name.to_string(), meta_value);
                        }
                        // If the parameter was bound as a relational variable (LTerm),
                        // it cannot be used for meta template expansion
                    }
                    TypeAnnotation::Relation(_) => {
                        // Relational parameters are not bound as meta variables - they're handled normally
                        // They will be processed during the expanded goal execution
                    }
                }
            }
        }

        // Expand the macro body using the template context with bound parameters
        let goals_vec = rel_def.body.to_vec();
        let expanded_goals = expand_goal_body(&goals_vec, &mut template_context).map_err(|e| {
            InterpreterError::RuntimeError(format!("Template expansion error: {}", e))
        })?;

        // Convert all expanded goals to runtime goals
        let mut runtime_goals = Vec::new();
        for goal in &expanded_goals {
            runtime_goals.push(self.ast_goal_to_runtime(goal)?);
        }

        // Combine all runtime goals into a single conjunction
        let result = if runtime_goals.is_empty() {
            Goal::succeed()
        } else {
            let mut iter = runtime_goals.into_iter();
            let first = iter.next().unwrap();
            iter.fold(first, |acc, next| Conj::new(acc, next))
        };

        // Pop the scope
        self.pop_scope();

        Ok(result)
    }

    /// Expand and execute a meta statement using template expansion
    fn expand_and_execute_meta_statement(
        &mut self,
        meta_stmt: &super::metaprogramming::MetaStatement,
        span: &super::parser::ast::Span,
    ) -> Result<Goal<U, E>, InterpreterError> {
        // Create template expansion context with a reasonable recursion limit
        let mut context = TemplateExpansionContext::new(100);

        // Expand the meta statement into concrete goals
        let expanded_goals = expand_meta_statement(meta_stmt, &mut context, span).map_err(|e| {
            InterpreterError::RuntimeError(format!("Template expansion error: {}", e))
        })?;

        // Convert expanded goals to runtime goals
        let mut runtime_goals = Vec::new();
        for goal in &expanded_goals {
            runtime_goals.push(self.ast_goal_to_runtime(goal)?);
        }

        // Combine all runtime goals into a single conjunction
        if runtime_goals.is_empty() {
            Ok(Goal::succeed())
        } else {
            let mut iter = runtime_goals.into_iter();
            let first = iter.next().unwrap();
            Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
        }
    }

    /// Expand and evaluate an interpolation expression
    fn expand_and_evaluate_interpolation(
        &mut self,
        expr: &super::metaprogramming::MetaExpression,
    ) -> Result<LTerm<U, E>, InterpreterError> {
        // Create empty template expansion context (interpolation should only use existing bindings)
        let context = TemplateExpansionContext::new(100);

        // Create a dummy term with the interpolation and expand it
        let dummy_term = Term::Interpolation(expr.clone(), Default::default());
        let expanded_term = expand_term(&dummy_term, &context).map_err(|e| {
            InterpreterError::RuntimeError(format!("Interpolation expansion error: {}", e))
        })?;

        // Convert the expanded term to runtime
        self.ast_term_to_runtime(&expanded_term)
    }
}

/// Helper to construct a list LTerm from elements and an optional tail.
fn lterm_from_vec_and_tail<U: User, E: Engine<U>>(
    elements: Vec<LTerm<U, E>>,
    tail: Option<LTerm<U, E>>,
) -> LTerm<U, E> {
    let mut list = tail.unwrap_or_else(LTerm::empty_list);
    for el in elements.into_iter().rev() {
        list = LTerm::cons(el, list);
    }
    list
}

fn convert_ast_literal_to_runtime<U: User, E: Engine<U>>(
    literal: &Literal,
) -> Result<LTerm<U, E>, InterpreterError> {
    match literal {
        Literal::Boolean(b) => Ok(LTerm::from(*b)),
        Literal::Number(n) => {
            if let Ok(num) = n.parse::<isize>() {
                Ok(LTerm::from(num))
            } else {
                Err(InterpreterError::RuntimeError(format!(
                    "Invalid number literal: {}",
                    n
                )))
            }
        }
        Literal::String(s) => Ok(LTerm::from(s.clone())),
        Literal::Char(c) => Ok(LTerm::from(*c)),
    }
}
