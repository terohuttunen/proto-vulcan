/// Semantic analysis pass that resolves ambiguous AST nodes
///
/// This module performs semantic disambiguation by transforming ambiguous
/// TupleStruct nodes that are actually enum variants into EnumVariant nodes.
/// This is done using type registry information to determine if a qualified
/// name refers to an enum variant.
use super::environment::{Environment, TypeDefinition};
use super::parser::ast::*;
use super::runtime_value::RuntimeValue;
use super::symbol_table::InternedSymbol;
use super::InterpreterError;
use std::cell::RefCell;
use std::rc::Rc;

pub struct SemanticAnalyzer<'a> {
    environment: Rc<RefCell<Environment>>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> SemanticAnalyzer<'a> {
    pub fn new(environment: Rc<RefCell<Environment>>) -> Self {
        Self {
            environment,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Perform semantic analysis on a program
    pub fn analyze_program(&mut self, program: &mut Program) -> Result<(), InterpreterError> {
        for item in &mut program.items {
            self.analyze_item(item)?;
        }
        Ok(())
    }

    /// Analyze a program item
    fn analyze_item(&mut self, item: &mut Item) -> Result<(), InterpreterError> {
        match item {
            Item::Predicate(relation) => self.analyze_relation(relation),
            Item::Enum(_)
            | Item::Struct(_)
            | Item::Module(_)
            | Item::Use(_)
            | Item::ModuleDeclaration(_)
            | Item::Impl(_) => {
                // Type definitions and module items don't need term disambiguation
                Ok(())
            }
        }
    }

    /// Analyze a relation definition  
    fn analyze_relation(
        &mut self,
        relation: &mut PredicateDefinition,
    ) -> Result<(), InterpreterError> {
        // Analyze attributes first (this handles test expectations)
        for attribute in &mut relation.attributes {
            self.analyze_attribute(attribute)?;
        }

        // Then analyze the relation body
        for goal in &mut relation.body {
            self.analyze_goal(goal)?;
        }
        Ok(())
    }

    /// Analyze an attribute and its arguments
    fn analyze_attribute(&mut self, attribute: &mut Attribute) -> Result<(), InterpreterError> {
        for arg in &mut attribute.args {
            match arg {
                AttributeArg::Flag(_) => {
                    // Flags don't contain terms to analyze
                }
                AttributeArg::Named(_, term) => {
                    self.analyze_term(term)?;
                }
            }
        }
        Ok(())
    }

    /// Analyze a goal and its sub-terms
    fn analyze_goal(&mut self, goal: &mut Goal) -> Result<(), InterpreterError> {
        match goal {
            Goal::Equality(left, right, _) => {
                self.analyze_term(left)?;
                self.analyze_term(right)?;
            }
            Goal::Disequality(left, right, _) => {
                self.analyze_term(left)?;
                self.analyze_term(right)?;
            }
            Goal::Conjunction(conj, _) => {
                for goal in &mut conj.body {
                    self.analyze_goal(goal)?;
                }
            }
            Goal::Disjunction(disj, _) => {
                for goal in &mut disj.body {
                    self.analyze_goal(goal)?;
                }
            }
            Goal::Fresh(fresh, _) => {
                for goal in &mut fresh.body {
                    self.analyze_goal(goal)?;
                }
            }
            Goal::RelationCall(call, _) => {
                for arg in &mut call.args {
                    self.analyze_call_argument(arg)?;
                }
            }
            Goal::MethodCall(call, _) => {
                self.analyze_term(&mut call.receiver)?;
                for arg in &mut call.args {
                    self.analyze_term(arg)?;
                }
            }
            Goal::Let(let_decl, _) => {
                if let Some(value) = &mut let_decl.value {
                    self.analyze_term(value)?;
                }
            }
            Goal::Parenthesized(body, _) => {
                for goal in body {
                    self.analyze_goal(goal)?;
                }
            }
            Goal::PatternMatch(pattern_match, _) => {
                self.analyze_term(&mut pattern_match.term)?;
                for arm in &mut pattern_match.arms {
                    self.analyze_pattern(&mut arm.pattern)?;
                    for goal in &mut arm.body {
                        self.analyze_goal(goal)?;
                    }
                }
            }
            Goal::BooleanLiteral(..) => {
                // Boolean literals don't contain terms to analyze
            }
            Goal::ConstraintBlock(..) => {
                // TODO: Analyze constraint blocks if they contain terms
            }
            Goal::MetaStatement(..) => {
                // TODO: Analyze meta statements if they contain terms
            }
        }
        Ok(())
    }

    /// Analyze a call argument
    fn analyze_call_argument(&mut self, arg: &mut CallArgument) -> Result<(), InterpreterError> {
        match arg {
            CallArgument::Term(term) => self.analyze_term(term),
            CallArgument::MetaExpression(_) => {
                // Meta expressions don't typically contain enum variants
                Ok(())
            }
        }
    }

    /// Analyze and potentially transform a term
    fn analyze_term(&mut self, term: &mut Term) -> Result<(), InterpreterError> {
        match term {
            Term::Variable(name) => {
                // Check if this Variable is actually a unit enum variant
                if let Some(enum_variant) = self.try_disambiguate_variable_as_enum_variant(name)? {
                    // Transform this Variable into an EnumVariant
                    *term = Term::EnumVariant(enum_variant, name.span_ref().clone());
                }
                Ok(())
            }
            Term::Wildcard(_) | Term::Literal(_, _) => {
                // These don't need disambiguation
                Ok(())
            }
            Term::List(list, _) => {
                for element in &mut list.elements {
                    self.analyze_term(element)?;
                }
                if let Some(tail) = &mut list.tail {
                    self.analyze_term(tail)?;
                }
                Ok(())
            }
            Term::NamedStruct(named_struct, span) => {
                // First analyze field values
                for field in &mut named_struct.fields {
                    self.analyze_term(&mut field.value)?;
                }

                // Check if this NamedStruct is actually an enum variant
                if let Some(enum_variant) =
                    self.try_disambiguate_named_struct_as_enum_variant(named_struct)?
                {
                    // Transform this NamedStruct into an EnumVariant
                    *term = Term::EnumVariant(enum_variant, span.clone());
                }
                Ok(())
            }
            Term::TupleStruct(tuple_struct, span) => {
                // First analyze arguments
                for arg in &mut tuple_struct.args {
                    self.analyze_term(arg)?;
                }

                // Check if this TupleStruct is actually an enum variant
                if let Some(enum_variant) = self.try_disambiguate_as_enum_variant(tuple_struct)? {
                    // Transform this TupleStruct into an EnumVariant
                    *term = Term::EnumVariant(enum_variant, span.clone());
                }
                Ok(())
            }
            Term::EnumVariant(enum_variant, _) => {
                // Analyze arguments in enum variants
                match &mut enum_variant.kind {
                    EnumVariantConstructionKind::Unit => {}
                    EnumVariantConstructionKind::Tuple(args) => {
                        for arg in args {
                            self.analyze_term(arg)?;
                        }
                    }
                    EnumVariantConstructionKind::Named(fields) => {
                        for field in fields {
                            self.analyze_term(&mut field.value)?;
                        }
                    }
                }
                Ok(())
            }
            Term::Parenthesized(inner, _) => self.analyze_term(inner),
            Term::Interpolation(_, _) => {
                // Meta interpolations don't typically contain enum variants
                Ok(())
            }
        }
    }

    /// Perform semantic analysis on a pattern
    fn analyze_pattern(&mut self, pattern: &mut Pattern) -> Result<(), InterpreterError> {
        match pattern {
            Pattern::Literal(_) | Pattern::Variable(_) | Pattern::Wildcard => {
                // These patterns don't need disambiguation
                Ok(())
            }
            Pattern::List(list_pattern) => {
                // Analyze nested patterns
                for element in &mut list_pattern.elements {
                    self.analyze_pattern(element)?;
                }
                if let Some(tail) = &mut list_pattern.tail {
                    self.analyze_pattern(tail)?;
                }
                Ok(())
            }
            Pattern::NamedStruct(named_struct_pattern) => {
                // First analyze field patterns
                for field_pattern in &mut named_struct_pattern.fields {
                    self.analyze_pattern(&mut field_pattern.pattern)?;
                }

                // Check if this NamedStruct pattern is actually an enum variant pattern
                if let Some(enum_variant_pattern) = self
                    .try_disambiguate_named_struct_pattern_as_enum_variant(named_struct_pattern)?
                {
                    // Transform this NamedStruct pattern into an EnumVariant pattern
                    *pattern = Pattern::EnumVariant(enum_variant_pattern);
                }
                Ok(())
            }
            Pattern::TupleStruct(tuple_struct_pattern) => {
                // First analyze argument patterns
                for arg_pattern in &mut tuple_struct_pattern.args {
                    self.analyze_pattern(arg_pattern)?;
                }

                // Check if this TupleStruct pattern is actually an enum variant pattern
                if let Some(enum_variant_pattern) = self
                    .try_disambiguate_tuple_struct_pattern_as_enum_variant(tuple_struct_pattern)?
                {
                    // Transform this TupleStruct pattern into an EnumVariant pattern
                    *pattern = Pattern::EnumVariant(enum_variant_pattern);
                }
                Ok(())
            }
            Pattern::EnumVariant(enum_variant_pattern) => {
                // Analyze patterns within enum variants
                match &mut enum_variant_pattern.kind {
                    EnumVariantPatternKind::Unit => {}
                    EnumVariantPatternKind::Tuple(patterns) => {
                        for pattern in patterns {
                            self.analyze_pattern(pattern)?;
                        }
                    }
                    EnumVariantPatternKind::Named(field_patterns) => {
                        for field_pattern in field_patterns {
                            self.analyze_pattern(&mut field_pattern.pattern)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    /// Try to disambiguate a Variable as a unit enum variant
    /// Returns Some(EnumVariant) if the Variable should be a unit enum variant, None otherwise
    fn try_disambiguate_variable_as_enum_variant(
        &self,
        variable_name: &InternedSymbol,
    ) -> Result<Option<EnumVariantConstruction>, InterpreterError> {
        // Check if the variable name contains "::" which indicates potential enum variant
        if let Some((enum_name, variant_name)) = variable_name.to_string().rsplit_once("::") {
            // Try to resolve the enum type
            let env = self.environment.borrow();

            // Try to lookup the enum type by name using the same approach as execution
            // Drop the borrow before calling resolve_type_to_index to avoid borrow conflict
            drop(env);
            if let Ok(type_index) = self.resolve_type_to_index(enum_name) {
                let env = self.environment.borrow();
                if let Some(type_def) = env.get_type_by_index(type_index) {
                    if let TypeDefinition::Enum(enum_def) = type_def {
                        // Check if this variant exists in the enum and is a unit variant
                        if let Some(variant_def) =
                            enum_def.variants.iter().find(|v| v.name == variant_name)
                        {
                            // Only disambiguate unit variants (no arguments)
                            if let VariantKind::Unit = &variant_def.kind {
                                return Ok(Some(EnumVariantConstruction {
                                    enum_name: InternedSymbol::from_text(enum_name),
                                    variant_name: InternedSymbol::from_text(variant_name),
                                    kind: EnumVariantConstructionKind::Unit,
                                }));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Try to disambiguate a TupleStruct as an enum variant
    /// Returns Some(EnumVariant) if the TupleStruct should be an enum variant, None otherwise
    fn try_disambiguate_as_enum_variant(
        &self,
        tuple_struct: &TupleStructConstruction,
    ) -> Result<Option<EnumVariantConstruction>, InterpreterError> {
        // Check if the name contains "::" which indicates potential enum variant
        if let Some((enum_name, variant_name)) = tuple_struct.name.to_string().rsplit_once("::") {
            // Try to resolve the enum type
            let env = self.environment.borrow();

            // Try to lookup the enum type by name using the same approach as execution
            // Drop the borrow before calling resolve_type_to_index to avoid borrow conflict
            drop(env);
            if let Ok(type_index) = self.resolve_type_to_index(enum_name) {
                let env = self.environment.borrow();
                if let Some(type_def) = env.get_type_by_index(type_index) {
                    if let TypeDefinition::Enum(enum_def) = type_def {
                        // Check if this variant exists in the enum
                        if let Some(variant_def) =
                            enum_def.variants.iter().find(|v| v.name == variant_name)
                        {
                            // Determine the correct variant construction kind
                            let construction_kind =
                                match (&variant_def.kind, tuple_struct.args.len()) {
                                    // Unit variant with no args
                                    (VariantKind::Unit, 0) => EnumVariantConstructionKind::Unit,

                                    // Tuple variant with args
                                    (VariantKind::Tuple(_), _) => {
                                        EnumVariantConstructionKind::Tuple(
                                            tuple_struct.args.clone(),
                                        )
                                    }

                                    // Named variant - this is more complex, we'd need to check field structure
                                    (VariantKind::Named(_), 0) => {
                                        // For now, assume unit if no args provided to named variant
                                        // This could be improved to handle named construction syntax
                                        return Ok(None);
                                    }

                                    // Mismatch between variant definition and usage
                                    _ => return Ok(None),
                                };

                            return Ok(Some(EnumVariantConstruction {
                                enum_name: InternedSymbol::from_text(enum_name),
                                variant_name: InternedSymbol::from_text(variant_name),
                                kind: construction_kind,
                            }));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Try to disambiguate a NamedStruct as an enum variant
    /// Returns Some(EnumVariant) if the NamedStruct should be an enum variant, None otherwise
    fn try_disambiguate_named_struct_as_enum_variant(
        &self,
        named_struct: &NamedStructConstruction,
    ) -> Result<Option<EnumVariantConstruction>, InterpreterError> {
        // Check if the name contains "::" which indicates potential enum variant
        if let Some((enum_name, variant_name)) = named_struct.name.rsplit_once("::") {
            // Try to resolve the enum type
            let env = self.environment.borrow();

            // Try to lookup the enum type by name using the same approach as execution
            // Drop the borrow before calling resolve_type_to_index to avoid borrow conflict
            drop(env);
            if let Ok(type_index) = self.resolve_type_to_index(enum_name) {
                let env = self.environment.borrow();
                if let Some(type_def) = env.get_type_by_index(type_index) {
                    if let TypeDefinition::Enum(enum_def) = type_def {
                        // Check if this variant exists in the enum and is a named variant
                        if let Some(variant_def) =
                            enum_def.variants.iter().find(|v| v.name == variant_name)
                        {
                            if let VariantKind::Named(_) = &variant_def.kind {
                                // Convert NamedStruct fields to EnumVariant named fields
                                let named_fields = named_struct
                                    .fields
                                    .iter()
                                    .map(|field| FieldInitializer {
                                        name: field.name.clone(),
                                        value: field.value.clone(),
                                    })
                                    .collect();

                                return Ok(Some(EnumVariantConstruction {
                                    enum_name: InternedSymbol::from_text(enum_name),
                                    variant_name: InternedSymbol::from_text(variant_name),
                                    kind: EnumVariantConstructionKind::Named(named_fields),
                                }));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Try to disambiguate a NamedStruct pattern as an enum variant pattern
    /// Returns Some(EnumVariantPattern) if the NamedStruct pattern should be an enum variant pattern, None otherwise
    fn try_disambiguate_named_struct_pattern_as_enum_variant(
        &self,
        named_struct_pattern: &NamedStructPattern,
    ) -> Result<Option<EnumVariantPattern>, InterpreterError> {
        // Check if the name contains "::" which indicates potential enum variant
        if let Some((enum_name, variant_name)) = named_struct_pattern.name.rsplit_once("::") {
            // Try to resolve the enum type
            let env = self.environment.borrow();

            // Drop the borrow before calling resolve_type_to_index to avoid borrow conflict
            drop(env);
            if let Ok(type_index) = self.resolve_type_to_index(enum_name) {
                let env = self.environment.borrow();
                if let Some(type_def) = env.get_type_by_index(type_index) {
                    if let TypeDefinition::Enum(enum_def) = type_def {
                        // Check if this variant exists in the enum and is a named variant
                        if let Some(variant_def) =
                            enum_def.variants.iter().find(|v| v.name == variant_name)
                        {
                            if let VariantKind::Named(_) = &variant_def.kind {
                                // Convert NamedStruct pattern fields to EnumVariant pattern named fields
                                let named_field_patterns = named_struct_pattern.fields.clone();

                                return Ok(Some(EnumVariantPattern {
                                    enum_name: InternedSymbol::from_text(enum_name),
                                    variant_name: InternedSymbol::from_text(variant_name),
                                    kind: EnumVariantPatternKind::Named(named_field_patterns),
                                }));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Try to disambiguate a TupleStruct pattern as an enum variant pattern
    /// Returns Some(EnumVariantPattern) if the TupleStruct pattern should be an enum variant pattern, None otherwise
    fn try_disambiguate_tuple_struct_pattern_as_enum_variant(
        &self,
        tuple_struct_pattern: &TupleStructPattern,
    ) -> Result<Option<EnumVariantPattern>, InterpreterError> {
        // Check if the name contains "::" which indicates potential enum variant
        if let Some((enum_name, variant_name)) = tuple_struct_pattern.name.rsplit_once("::") {
            // Try to resolve the enum type
            let env = self.environment.borrow();

            // Drop the borrow before calling resolve_type_to_index to avoid borrow conflict
            drop(env);
            if let Ok(type_index) = self.resolve_type_to_index(enum_name) {
                let env = self.environment.borrow();
                if let Some(type_def) = env.get_type_by_index(type_index) {
                    if let TypeDefinition::Enum(enum_def) = type_def {
                        // Check if this variant exists in the enum
                        if let Some(variant_def) =
                            enum_def.variants.iter().find(|v| v.name == variant_name)
                        {
                            // Determine the correct variant pattern kind
                            let pattern_kind =
                                match (&variant_def.kind, tuple_struct_pattern.args.len()) {
                                    // Unit variant with no args
                                    (VariantKind::Unit, 0) => EnumVariantPatternKind::Unit,

                                    // Tuple variant with args
                                    (VariantKind::Tuple(_), _) => EnumVariantPatternKind::Tuple(
                                        tuple_struct_pattern.args.clone(),
                                    ),

                                    // Named variant - shouldn't use tuple syntax, but handle gracefully
                                    (VariantKind::Named(_), 0) => {
                                        // For now, return None since named variants should use named syntax
                                        return Ok(None);
                                    }

                                    // Mismatch between variant definition and usage
                                    _ => return Ok(None),
                                };

                            return Ok(Some(EnumVariantPattern {
                                enum_name: InternedSymbol::from_text(enum_name),
                                variant_name: InternedSymbol::from_text(variant_name),
                                kind: pattern_kind,
                            }));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Resolve a type name to its registry index
    fn resolve_type_to_index<S: AsRef<str>>(&self, name: S) -> Result<usize, InterpreterError> {
        // Look up the type name as a symbol
        let name_str = name.as_ref();
        let runtime_value = self
            .environment
            .borrow()
            .lookup(name_str)
            .ok_or_else(|| InterpreterError::UnknownType(name_str.to_string()))?
            .clone();

        match runtime_value {
            RuntimeValue::Type(index) => Ok(index),
            _ => Err(InterpreterError::NotAType(name_str.to_string())),
        }
    }
}

/// Public function to perform semantic analysis on a program
pub fn analyze_program(
    program: &mut Program,
    environment: Rc<RefCell<Environment>>,
) -> Result<(), InterpreterError> {
    let mut analyzer = SemanticAnalyzer::new(environment);
    analyzer.analyze_program(program)
}

/// Public function to perform semantic analysis on a single term
pub fn analyze_term(
    term: &mut Term,
    environment: &Rc<RefCell<Environment>>,
) -> Result<(), InterpreterError> {
    let mut analyzer = SemanticAnalyzer::new(environment.clone());
    analyzer.analyze_term(term)
}

/// Public function to perform semantic analysis on a query goal
pub fn analyze_goal(
    goal: &mut Goal,
    environment: &Rc<RefCell<Environment>>,
) -> Result<(), InterpreterError> {
    let mut analyzer = SemanticAnalyzer::new(environment.clone());
    analyzer.analyze_goal(goal)
}
