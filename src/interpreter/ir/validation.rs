//! Semantic validation and type checking for IR

use super::*;
use crate::interpreter::symbol_table::InternedSymbol;
use std::collections::{HashMap, HashSet};

/// Validates IR for semantic correctness
pub struct Validator {
    /// Cache for validated items to avoid redundant checks
    validated_items: HashMap<ItemId, bool>,
    /// Stack to detect circular dependencies
    validation_stack: Vec<ItemId>,
}

impl Validator {
    pub fn new() -> Self {
        Self {
            validated_items: HashMap::new(),
            validation_stack: Vec::new(),
        }
    }

    /// Validate an IR program
    pub fn validate_program(&self, program: &Program) -> Result<(), super::compiler::CompileError> {
        let mut validator = Validator::new();

        // Validate all items in the registry
        let item_ids: Vec<_> = program
            .registry
            .all_items()
            .map(|item| item.id().clone())
            .collect();
        for item_id in &item_ids {
            validator.validate_item(item_id, program)?;
        }

        Ok(())
    }

    /// Validate a specific item and its dependencies
    fn validate_item<T: AsRef<ItemId>>(
        &mut self,
        item_id: T,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        let item_id = item_id.as_ref();
        // Skip if already validated
        if self.validated_items.contains_key(item_id) {
            return Ok(());
        }

        // Check for circular dependency
        if self.validation_stack.contains(item_id) {
            return Err(super::compiler::CompileError::CircularDependency {
                cycle: self.format_dependency_cycle(item_id),
                symbol: InternedSymbol::from_text(&item_id.path),
            });
        }

        // Add to validation stack
        self.validation_stack.push(item_id.clone());

        // Get the item from registry
        let item = program.registry.get_item(item_id).ok_or_else(|| {
            super::compiler::CompileError::UnresolvedReference {
                attempted_item: item_id.clone(),
                symbol: InternedSymbol::from_text(&item_id.path),
            }
        })?;

        // Validate the item based on its type
        match item {
            Item::Module(module) => self.validate_module(&module, program)?,
            Item::Type(type_def) => self.validate_type_definition(&type_def, program)?,
            Item::Predicate(predicate) => self.validate_predicate(&predicate, program)?,
            Item::Import(import) => self.validate_import(&import, program)?,
        }

        // Remove from validation stack
        self.validation_stack.pop();

        // Mark as validated
        self.validated_items.insert(item_id.clone(), true);

        Ok(())
    }

    /// Validate a module
    fn validate_module(
        &mut self,
        module: &Module,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        // Validate all child items exist and are accessible
        for child_id in &module.items {
            // Check that child item exists
            if !program.registry.contains_item(child_id) {
                return Err(super::compiler::CompileError::UnresolvedReference {
                    attempted_item: child_id.clone(),
                    symbol: InternedSymbol::from_text(&child_id.path),
                });
            }

            // Recursively validate child item
            self.validate_item(child_id, program)?;
        }

        Ok(())
    }

    /// Validate a type definition
    fn validate_type_definition(
        &mut self,
        type_def: &TypeDefinition,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        match &type_def.kind {
            TypeKind::Struct(struct_def) => self.validate_struct_definition(struct_def, program)?,
            TypeKind::Enum(enum_def) => self.validate_enum_definition(enum_def, program)?,
        }
        Ok(())
    }

    /// Validate a struct definition
    fn validate_struct_definition(
        &mut self,
        struct_def: &StructDefinition,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        match &struct_def.fields {
            StructFields::Named(named_fields) => {
                let mut field_names = HashSet::new();
                for field in named_fields {
                    // Check for duplicate field names
                    if !field_names.insert(&field.name) {
                        return Err(super::compiler::CompileError::DuplicateField {
                            field_name: field.name.to_string(),
                            symbol: field.name.clone(),
                        });
                    }

                    // Validate field type reference
                    self.validate_type_reference(&field.type_ref, program)?;
                }
            }
            StructFields::Tuple(tuple_fields) => {
                for type_ref in tuple_fields {
                    // Validate tuple field type reference
                    self.validate_type_reference(type_ref, program)?;
                }
            }
        }
        Ok(())
    }

    /// Validate an enum definition
    fn validate_enum_definition(
        &mut self,
        enum_def: &EnumDefinition,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        let mut variant_names = HashSet::new();

        for variant in &enum_def.variants {
            // Check for duplicate variant names
            if !variant_names.insert(&variant.name) {
                return Err(super::compiler::CompileError::DuplicateEnumVariant {
                    variant_name: variant.name.to_string(),
                    symbol: variant.name.clone(),
                });
            }

            // Validate variant fields
            match &variant.kind {
                EnumVariantKind::Unit => {
                    // Unit variants have no fields to validate
                }
                EnumVariantKind::Tuple(tuple_fields) => {
                    for type_ref in tuple_fields {
                        self.validate_type_reference(type_ref, program)?;
                    }
                }
                EnumVariantKind::Named(named_fields) => {
                    let mut field_names = HashSet::new();
                    for field in named_fields {
                        // Check for duplicate field names within variant
                        if !field_names.insert(&field.name) {
                            return Err(super::compiler::CompileError::DuplicateField {
                                field_name: field.name.to_string(),
                                symbol: field.name.clone(),
                            });
                        }

                        // Validate field type reference
                        self.validate_type_reference(&field.type_ref, program)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate a predicate definition
    fn validate_predicate(
        &mut self,
        predicate: &Predicate,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        // Check for duplicate parameter names
        let mut param_names = HashSet::new();
        for param in &predicate.parameters {
            if !param_names.insert(&param.name) {
                return Err(super::compiler::CompileError::DuplicateParameter {
                    parameter_name: param.name.to_string(),
                    symbol: param.name.clone(),
                });
            }

            // Validate type annotations if present
            if let Some(type_annotation) = &param.type_annotation {
                self.validate_type_annotation(type_annotation, program)?;
            }
        }

        // Validate all goals in the body
        for goal in predicate.body.iter() {
            self.validate_goal(goal, program)?;
        }

        Ok(())
    }

    /// Validate an import
    fn validate_import(
        &mut self,
        import: &Import,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        // Check that the source reference exists and is accessible
        if !program.registry.contains_item(&import.source_ref) {
            return Err(super::compiler::CompileError::UnresolvedReference {
                attempted_item: import.source_ref.clone(),
                symbol: InternedSymbol::from_text(&import.source_ref.path),
            });
        }

        // Recursively validate the source item
        self.validate_item(&import.source_ref, program)?;

        Ok(())
    }

    /// Validate a type reference
    fn validate_type_reference<T: AsRef<ItemId>>(
        &mut self,
        type_ref: T,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        let type_ref = type_ref.as_ref();
        // Check that the type exists
        let type_item = program.registry.get_type(type_ref).ok_or_else(|| {
            super::compiler::CompileError::UnresolvedType {
                attempted_item: TypeId::new(type_ref.path.clone()),
                symbol: InternedSymbol::from_text(&type_ref.path),
            }
        })?;

        // Recursively validate the type definition
        self.validate_item(&type_ref, program)?;

        Ok(())
    }

    /// Validate a type annotation
    fn validate_type_annotation(
        &mut self,
        type_annotation: &TypeAnnotation,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        match type_annotation {
            TypeAnnotation::Int | TypeAnnotation::String | TypeAnnotation::Bool => {
                // Built-in types are always valid
                Ok(())
            }
            TypeAnnotation::Relation(arity) => {
                // Validate arity is reasonable (not negative, not too large)
                if *arity > 1000 {
                    return Err(super::compiler::CompileError::SemanticError {
                        message: format!("Relation arity {} is unreasonably large", arity),
                        symbol: InternedSymbol::from_text("relation_arity"),
                    });
                }
                Ok(())
            }
            TypeAnnotation::Custom(type_ref) => {
                // Validate the custom type reference
                self.validate_type_reference(type_ref, program)
            }
        }
    }

    /// Validate a goal in IR
    fn validate_goal(
        &mut self,
        goal: &Goal,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        match goal {
            Goal::Equality(left, right) | Goal::Disequality(left, right) => {
                self.validate_term(left, program)?;
                self.validate_term(right, program)?;
            }
            Goal::PredicateCall(predicate_call) => {
                self.validate_predicate_call(predicate_call, program)?;
            }
            Goal::Conjunction(goals) | Goal::Disjunction(goals) => {
                for goal in goals.iter() {
                    self.validate_goal(goal, program)?;
                }
            }
            Goal::PatternMatch(pattern_match) => {
                self.validate_pattern_match(pattern_match, program)?;
            }
            Goal::Fresh(fresh) => {
                for goal in fresh.body.iter() {
                    self.validate_goal(goal, program)?;
                }
            }
            Goal::Let(let_binding) => {
                if let Some(value) = &let_binding.value {
                    self.validate_term(value, program)?;
                }
                for goal in let_binding.body.iter() {
                    self.validate_goal(goal, program)?;
                }
            }
            Goal::Boolean(_) => {
                // Boolean goals are always valid
            }
            Goal::Constraint(_constraint_block) => {
                // TODO: Validate constraint blocks when constraint system is implemented
            }
            Goal::MetaLet(meta_let) => {
                self.validate_meta_expression(&meta_let.expression, program)?;
            }
            Goal::MetaIf(meta_if) => {
                self.validate_meta_expression(&meta_if.condition, program)?;
                for goal in meta_if.then_body.iter() {
                    self.validate_goal(goal, program)?;
                }
                for (condition, body) in &meta_if.else_ifs {
                    self.validate_meta_expression(condition, program)?;
                    for goal in body.iter() {
                        self.validate_goal(goal, program)?;
                    }
                }
                if let Some(else_body) = &meta_if.else_body {
                    for goal in else_body.iter() {
                        self.validate_goal(goal, program)?;
                    }
                }
            }
            Goal::MetaFor(meta_for) => {
                self.validate_meta_expression(&meta_for.start, program)?;
                self.validate_meta_expression(&meta_for.end, program)?;
                for goal in meta_for.body.iter() {
                    self.validate_goal(goal, program)?;
                }
            }
        }
        Ok(())
    }

    /// Validate a predicate call
    fn validate_predicate_call(
        &mut self,
        predicate_call: &PredicateCall,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        // Check arity match using registry convenience method
        let expected_arity = program
            .registry
            .get_predicate_arity(&predicate_call.predicate)
            .ok_or_else(|| super::compiler::CompileError::UnresolvedPredicate {
                attempted_item: predicate_call.predicate.clone(),
                symbol: InternedSymbol::from_text(&predicate_call.predicate.as_ref().path),
            })?;

        if predicate_call.arguments.len() != expected_arity {
            return Err(super::compiler::CompileError::ArityMismatch {
                predicate_item: predicate_call.predicate.clone(),
                expected_arity,
                actual_arity: predicate_call.arguments.len(),
                symbol: InternedSymbol::from_text(&predicate_call.predicate.as_ref().path),
            });
        }

        // Validate all arguments
        for arg in &predicate_call.arguments {
            self.validate_term(arg, program)?;
        }

        // Recursively validate the predicate definition
        self.validate_item(&predicate_call.predicate, program)?;

        Ok(())
    }

    /// Validate a pattern match
    fn validate_pattern_match(
        &mut self,
        pattern_match: &PatternMatch,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        // Validate the matched term
        self.validate_term(&pattern_match.term, program)?;

        // Validate each pattern arm
        for arm in &pattern_match.arms {
            self.validate_pattern(&arm.pattern, program)?;
            for goal in arm.body.iter() {
                self.validate_goal(goal, program)?;
            }
        }

        Ok(())
    }

    /// Validate a pattern
    fn validate_pattern(
        &mut self,
        pattern: &Pattern,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        match pattern {
            Pattern::Variable(_) | Pattern::Wildcard => {
                // Variables and wildcards are always valid
                Ok(())
            }
            Pattern::Literal(_) => {
                // Literals are always valid
                Ok(())
            }
            Pattern::List(list_pattern) => {
                for element_pattern in &list_pattern.elements {
                    self.validate_pattern(element_pattern, program)?;
                }
                if let Some(tail_pattern) = &list_pattern.tail {
                    self.validate_pattern(tail_pattern, program)?;
                }
                Ok(())
            }
            Pattern::Struct(struct_pattern) => {
                self.validate_type_reference(&struct_pattern.type_ref, program)?;
                match &struct_pattern.fields {
                    StructPatternFields::Named(named_patterns) => {
                        for field_pattern in named_patterns {
                            self.validate_pattern(&field_pattern.pattern, program)?;
                        }
                    }
                    StructPatternFields::Tuple(tuple_patterns) => {
                        for pattern in tuple_patterns {
                            self.validate_pattern(pattern, program)?;
                        }
                    }
                }
                Ok(())
            }
            Pattern::EnumVariant(enum_pattern) => {
                self.validate_type_reference(&enum_pattern.enum_ref, program)?;
                match &enum_pattern.kind {
                    EnumVariantPatternKind::Unit => Ok(()),
                    EnumVariantPatternKind::Tuple(tuple_patterns) => {
                        for pattern in tuple_patterns {
                            self.validate_pattern(pattern, program)?;
                        }
                        Ok(())
                    }
                    EnumVariantPatternKind::Named(named_patterns) => {
                        for field_pattern in named_patterns {
                            self.validate_pattern(&field_pattern.pattern, program)?;
                        }
                        Ok(())
                    }
                }
            }
        }
    }

    /// Validate a term
    fn validate_term(
        &mut self,
        term: &Term,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        match term {
            Term::Variable(_) | Term::Wildcard | Term::Literal(_) => {
                // Variables, wildcards, and literals are always valid
                Ok(())
            }
            Term::List(list) => {
                for element in &list.elements {
                    self.validate_term(element, program)?;
                }
                if let Some(tail) = &list.tail {
                    self.validate_term(tail, program)?;
                }
                Ok(())
            }
            Term::Struct(struct_construction) => {
                self.validate_type_reference(&struct_construction.type_ref, program)?;
                match &struct_construction.fields {
                    StructConstructionFields::Named(named_fields) => {
                        for field in named_fields {
                            self.validate_term(&field.value, program)?;
                        }
                    }
                    StructConstructionFields::Tuple(tuple_fields) => {
                        for field in tuple_fields {
                            self.validate_term(field, program)?;
                        }
                    }
                }
                Ok(())
            }
            Term::EnumVariant(enum_construction) => {
                self.validate_type_reference(&enum_construction.enum_ref, program)?;
                match &enum_construction.kind {
                    EnumVariantConstructionKind::Unit => Ok(()),
                    EnumVariantConstructionKind::Tuple(tuple_fields) => {
                        for field in tuple_fields {
                            self.validate_term(field, program)?;
                        }
                        Ok(())
                    }
                    EnumVariantConstructionKind::Named(named_fields) => {
                        for field in named_fields {
                            self.validate_term(&field.value, program)?;
                        }
                        Ok(())
                    }
                }
            }
            Term::MetaInterpolation(meta_expr) => self.validate_meta_expression(meta_expr, program),
        }
    }

    /// Validate a meta expression in IR
    fn validate_meta_expression(
        &mut self,
        meta_expr: &MetaExpression,
        program: &Program,
    ) -> Result<(), super::compiler::CompileError> {
        match meta_expr {
            MetaExpression::Variable(_) => {
                // Meta variables are validated at expansion time
                Ok(())
            }
            MetaExpression::Literal(_) => {
                // Meta literals are always valid
                Ok(())
            }
            MetaExpression::BinaryOp(_, left, right) => {
                // Recursively validate operands
                self.validate_meta_expression(left, program)?;
                self.validate_meta_expression(right, program)
            }
        }
    }

    /// Format a dependency cycle for error reporting
    fn format_dependency_cycle<T: AsRef<ItemId>>(&self, current_item: T) -> String {
        let current_item = current_item.as_ref();
        let cycle_start = self
            .validation_stack
            .iter()
            .position(|id| id == current_item)
            .unwrap_or(0);

        let cycle_items: Vec<_> = self.validation_stack[cycle_start..]
            .iter()
            .map(|id| id.path.as_ref())
            .collect();

        format!("{} -> {}", cycle_items.join(" -> "), current_item.path)
    }
}
