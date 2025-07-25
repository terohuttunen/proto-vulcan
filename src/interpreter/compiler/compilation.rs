//! AST-to-IR compilation phase
//!
//! This module handles the third phase of compilation: compiling AST bodies to IR
//! with full symbol resolution using the resolved symbol maps.

use super::ir;
use super::*;

/// AST-to-IR compilation methods for the IR compiler
impl Compiler {
    /// Phase 3: Compile bodies with full symbol resolution
    pub(super) fn compile_bodies(
        &mut self,
        program: &ast::Program,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        for item in &program.items {
            self.compile_item_body(item, ir_program)?;
        }
        Ok(())
    }

    /// Compile the body of a single AST item
    fn compile_item_body(
        &mut self,
        item: &ast::Item,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        match item {
            ast::Item::Module(module) => {
                self.compile_module_body(module, ir_program)?;
            }
            ast::Item::Struct(struct_def) => {
                self.compile_struct_body(struct_def, ir_program)?;
            }
            ast::Item::Enum(enum_def) => {
                self.compile_enum_body(enum_def, ir_program)?;
            }
            ast::Item::Predicate(predicate) => {
                self.compile_predicate_body(predicate, ir_program)?;
            }
            ast::Item::Use(_) | ast::Item::ModuleDeclaration(_) => {
                // Skip these
            }
            ast::Item::Impl(impl_block) => {
                self.compile_impl_body(impl_block, ir_program)?;
            }
        }
        Ok(())
    }

    /// Compile module body (populate child items)
    fn compile_module_body(
        &mut self,
        module: &ast::ModuleDefinition,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Look up the module in the current module's symbol map
        let module_name = module.name.to_string();
        let current_module_map = self
            .module_symbol_maps
            .get(&self.symbol_context.current_module)
            .ok_or_else(|| CompileError::UnresolvedModule {
                attempted_item: self.symbol_context.current_module.clone(),
                symbol: module.name.clone(),
            })?;

        let module_item_id = current_module_map
            .local_symbols
            .get(&module_name)
            .or_else(|| current_module_map.imported_symbols.get(&module_name))
            .ok_or_else(|| CompileError::UnresolvedModule {
                attempted_item: ir::ModuleId::new(module_name.clone()).into(),
                symbol: module.name.clone(),
            })?;

        // Convert ItemId to ir::ModuleId (we know it's a module from context)
        let module_id = ir::ModuleId::new(module_item_id.path.clone());

        // Push module context
        self.module_path_stack.push(module.name.to_string());
        let old_current = self.symbol_context.current_module.clone();
        self.symbol_context.current_module = module_id.clone();

        // Compile child items
        for child_item in &module.items {
            self.compile_item_body(child_item, ir_program)?;
        }

        // Restore context
        self.module_path_stack.pop();
        self.symbol_context.current_module = old_current;

        Ok(())
    }

    /// Compile struct body (resolve field types)
    fn compile_struct_body(
        &mut self,
        struct_def: &ast::StructDefinition,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Look up the type in the current module's symbol map
        let type_name = struct_def.name.to_string();
        let current_module_map = self
            .module_symbol_maps
            .get(&self.symbol_context.current_module)
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: ir::TypeId::new(struct_def.name.to_string()),
                symbol: struct_def.name.clone(),
            })?;

        let type_item_id = current_module_map
            .local_symbols
            .get(&type_name)
            .or_else(|| current_module_map.imported_symbols.get(&type_name))
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: ir::TypeId::new(type_name.clone()).into(),
                symbol: struct_def.name.clone(),
            })?;

        // Convert ItemId to ir::TypeId (we know it's a type from context)
        let type_id = ir::TypeId::new(type_item_id.path.clone());

        // Resolve field types
        let fields = match &struct_def.kind {
            ast::StructKind::Named(named_fields) => {
                let mut ir_fields = vec![];
                for field in named_fields {
                    let field_type_ref = self.resolve_type_reference(&field.type_name)?;
                    ir_fields.push(ir::NamedField {
                        name: field.name.clone(),
                        type_ref: field_type_ref,
                        visibility: self.convert_visibility(&field.visibility)?,
                    });
                }
                ir::StructFields::Named(ir_fields)
            }
            ast::StructKind::Tuple(field_types) => {
                let mut ir_field_types = vec![];
                for field_type in field_types {
                    // Convert InternedSymbol to qualified path
                    let qualified_path = ast::QualifiedPath::Relative(vec![field_type.clone()]);
                    let field_type_ref = self.resolve_qualified_path_to_type(&qualified_path)?;
                    ir_field_types.push(field_type_ref);
                }
                ir::StructFields::Tuple(ir_field_types)
            }
        };

        // Update the type in the registry
        let registry = ir_program.registry_mut();
        let updated_type = ir::TypeDefinition {
            id: type_id,
            kind: ir::TypeKind::Struct(ir::StructDefinition { fields }),
            visibility: self.convert_visibility(&struct_def.visibility)?,
        };

        registry.replace_item(ir::Item::Type(updated_type));

        Ok(())
    }

    /// Compile enum body (resolve variant types)
    fn compile_enum_body(
        &mut self,
        enum_def: &ast::EnumDefinition,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Look up the type in the current module's symbol map
        let type_name = enum_def.name.to_string();
        let current_module_map = self
            .module_symbol_maps
            .get(&self.symbol_context.current_module)
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: ir::TypeId::new(enum_def.name.to_string()),
                symbol: enum_def.name.clone(),
            })?;

        let type_item_id = current_module_map
            .local_symbols
            .get(&type_name)
            .or_else(|| current_module_map.imported_symbols.get(&type_name))
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: ir::TypeId::new(type_name.clone()).into(),
                symbol: enum_def.name.clone(),
            })?;

        // Convert ItemId to ir::TypeId (we know it's a type from context)
        let type_id = ir::TypeId::new(type_item_id.path.clone());

        // Resolve variant types
        let mut variants = vec![];
        for variant in &enum_def.variants {
            let variant_kind = match &variant.kind {
                ast::VariantKind::Unit => ir::EnumVariantKind::Unit,
                ast::VariantKind::Tuple(field_types) => {
                    let mut ir_field_types = vec![];
                    for field_type in field_types {
                        // Convert InternedSymbol to qualified path
                        let qualified_path = ast::QualifiedPath::Relative(vec![field_type.clone()]);
                        let field_type_ref =
                            self.resolve_qualified_path_to_type(&qualified_path)?;
                        ir_field_types.push(field_type_ref);
                    }
                    ir::EnumVariantKind::Tuple(ir_field_types)
                }
                ast::VariantKind::Named(fields) => {
                    let mut ir_fields = vec![];
                    for field in fields {
                        let field_type_ref = self.resolve_type_reference(&field.type_name)?;
                        ir_fields.push(ir::NamedField {
                            name: field.name.clone(),
                            type_ref: field_type_ref,
                            visibility: self.convert_visibility(&field.visibility)?,
                        });
                    }
                    ir::EnumVariantKind::Named(ir_fields)
                }
            };

            variants.push(ir::EnumVariant {
                name: variant.name.clone(),
                kind: variant_kind,
            });
        }

        // Update the type in the registry
        let registry = ir_program.registry_mut();
        let updated_type = ir::TypeDefinition {
            id: type_id,
            kind: ir::TypeKind::Enum(ir::EnumDefinition { variants }),
            visibility: self.convert_visibility(&enum_def.visibility)?,
        };

        registry.replace_item(ir::Item::Type(updated_type));

        Ok(())
    }

    /// Resolve type reference from qualified path
    fn resolve_type_reference(
        &self,
        qualified_path: &ast::QualifiedPath,
    ) -> Result<ir::TypeId, CompileError> {
        self.resolve_qualified_path_to_type(qualified_path)
    }

    /// Resolve qualified path to ir::TypeId using proper scoped resolution
    fn resolve_qualified_path_to_type(
        &self,
        path: &ast::QualifiedPath,
    ) -> Result<ir::TypeId, CompileError> {
        // Start resolution from the current module context
        let segments: Vec<_> = path.segments().iter().map(|s| s.to_string()).collect();

        // Simple case: single segment - look in current module
        if segments.len() == 1 {
            let type_name = &segments[0];
            let current_module_map = self
                .module_symbol_maps
                .get(&self.symbol_context.current_module)
                .ok_or_else(|| CompileError::UnresolvedType {
                    attempted_item: ir::TypeId::new(type_name.to_string()),
                    symbol: InternedSymbol::from_text(type_name),
                })?;

            let type_item_id = current_module_map
                .local_symbols
                .get(type_name)
                .or_else(|| current_module_map.imported_symbols.get(type_name))
                .ok_or_else(|| CompileError::UnresolvedType {
                    attempted_item: ir::TypeId::new(type_name.clone()).into(),
                    symbol: InternedSymbol::from_text(type_name),
                })?;

            return Ok(ir::TypeId::new(type_item_id.path.clone()));
        }

        // Complex case: multi-segment path - walk through module hierarchy
        let mut current_module_id = self.symbol_context.current_module.clone();

        // Walk through all segments except the last one to find the target module
        for segment in &segments[..segments.len() - 1] {
            let module_map = self
                .module_symbol_maps
                .get(&current_module_id)
                .ok_or_else(|| CompileError::UnresolvedType {
                    attempted_item: ir::TypeId::new(segment.to_string()),
                    symbol: InternedSymbol::from_text(segment),
                })?;

            // Look for the segment as a local or imported module
            let module_item_id = module_map
                .get_local_module(segment)
                .or_else(|| {
                    module_map.get_imported_symbol(segment).and_then(|id| {
                        if id.kind == ir::ItemKind::Module {
                            Some(id)
                        } else {
                            None
                        }
                    })
                })
                .ok_or_else(|| CompileError::UnresolvedModule {
                    attempted_item: ir::ModuleId::new(format!(
                        "{}::{}",
                        current_module_id.id.path, segment
                    )),
                    symbol: InternedSymbol::from_text(segment),
                })?;

            current_module_id = ir::ModuleId::new(module_item_id.path.clone());
        }

        // Now look for the type in the final target module
        let type_name = &segments[segments.len() - 1];
        let target_module_map =
            self.module_symbol_maps
                .get(&current_module_id)
                .ok_or_else(|| CompileError::UnresolvedType {
                    attempted_item: ir::TypeId::new(type_name.to_string()),
                    symbol: InternedSymbol::from_text(type_name),
                })?;

        let type_item_id = target_module_map
            .local_symbols
            .get(type_name)
            .or_else(|| target_module_map.imported_symbols.get(type_name))
            .filter(|id| id.kind == ir::ItemKind::Type) // Ensure it's actually a type
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: ir::TypeId::new(type_name.clone()),
                symbol: InternedSymbol::from_text(type_name),
            })?;

        Ok(ir::TypeId::new(type_item_id.path.clone()))
    }

    /// Resolve qualified path to PredicateId using proper scoped resolution
    fn resolve_qualified_path_to_predicate(
        &self,
        path: &ast::QualifiedPath,
    ) -> Result<ir::PredicateId, CompileError> {
        // Start resolution from the current module context
        let segments: Vec<_> = path.segments().iter().map(|s| s.to_string()).collect();

        // Simple case: single segment - look in current module
        if segments.len() == 1 {
            let predicate_name = &segments[0];
            let current_module_map = self
                .module_symbol_maps
                .get(&self.symbol_context.current_module)
                .ok_or_else(|| CompileError::UnresolvedPredicate {
                    attempted_item: ir::PredicateId::new(predicate_name.to_string()),
                    symbol: InternedSymbol::from_text(predicate_name),
                })?;

            let predicate_item_id = current_module_map
                .local_symbols
                .get(predicate_name)
                .or_else(|| current_module_map.imported_symbols.get(predicate_name))
                .ok_or_else(|| CompileError::UnresolvedPredicate {
                    attempted_item: ir::PredicateId::new(predicate_name.clone()).into(),
                    symbol: InternedSymbol::from_text(predicate_name),
                })?;

            return Ok(ir::PredicateId::new(predicate_item_id.path.clone()));
        }

        // Complex case: multi-segment path - walk through module hierarchy
        let mut current_module_id = self.symbol_context.current_module.clone();

        // Walk through all segments except the last one to find the target module
        for segment in &segments[..segments.len() - 1] {
            let module_map = self
                .module_symbol_maps
                .get(&current_module_id)
                .ok_or_else(|| CompileError::UnresolvedPredicate {
                    attempted_item: ir::PredicateId::new(segment.to_string()),
                    symbol: InternedSymbol::from_text(segment),
                })?;

            let next_item_id = module_map
                .local_symbols
                .get(segment)
                .or_else(|| module_map.imported_symbols.get(segment))
                .ok_or_else(|| CompileError::UnresolvedPredicate {
                    attempted_item: ir::PredicateId::new(segment.clone()).into(),
                    symbol: InternedSymbol::from_text(segment),
                })?;

            // Update current module for next iteration
            current_module_id = ir::ModuleId::new(next_item_id.path.clone());
        }

        // Now resolve the final segment in the target module
        let final_segment = &segments[segments.len() - 1];
        let final_module_map =
            self.module_symbol_maps
                .get(&current_module_id)
                .ok_or_else(|| CompileError::UnresolvedPredicate {
                    attempted_item: ir::PredicateId::new(final_segment.to_string()),
                    symbol: InternedSymbol::from_text(final_segment),
                })?;

        let predicate_item_id = final_module_map
            .local_symbols
            .get(final_segment)
            .or_else(|| final_module_map.imported_symbols.get(final_segment))
            .ok_or_else(|| CompileError::UnresolvedPredicate {
                attempted_item: ir::PredicateId::new(final_segment.clone()).into(),
                symbol: InternedSymbol::from_text(final_segment),
            })?;

        Ok(ir::PredicateId::new(predicate_item_id.path.clone()))
    }

    // TODO: Add remaining compilation methods for predicates, goals, terms, patterns, etc.
    // This is a large amount of code (over 1000 lines) that would be moved from the original
    // compiler.rs file. For now, I'll include stub methods to complete the interface.

    /// Compile predicate body (goals, parameters, etc.)
    fn compile_predicate_body(
        &mut self,
        predicate: &ast::PredicateDefinition,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Look up the predicate in the current module's symbol map
        let predicate_name = predicate.name.to_string();
        let current_module_map = self
            .module_symbol_maps
            .get(&self.symbol_context.current_module)
            .ok_or_else(|| CompileError::UnresolvedPredicate {
                attempted_item: ir::PredicateId::new(predicate.name.to_string()),
                symbol: predicate.name.clone(),
            })?;

        let predicate_item_id = current_module_map
            .local_symbols
            .get(&predicate_name)
            .or_else(|| current_module_map.imported_symbols.get(&predicate_name))
            .ok_or_else(|| CompileError::UnresolvedPredicate {
                attempted_item: ir::PredicateId::new(predicate_name.clone()).into(),
                symbol: predicate.name.clone(),
            })?;

        // Convert ItemId to PredicateId (we know it's a predicate from context)
        let predicate_id = ir::PredicateId::new(predicate_item_id.path.clone());

        // Compile parameters
        let mut parameters = vec![];
        for param in &predicate.parameters {
            let type_annotation = if let Some(annotation) = &param.type_annotation {
                Some(self.compile_type_annotation(annotation)?)
            } else {
                None
            };

            parameters.push(ir::Parameter {
                name: param.name.clone(),
                type_annotation,
            });
        }

        // Compile body goals
        let mut body = vec![];
        for goal in predicate.body.iter() {
            body.push(self.compile_goal(goal)?);
        }

        // Update the predicate in the registry
        let registry = ir_program.registry_mut();
        let updated_predicate = ir::Predicate {
            id: predicate_id,
            parameters,
            body: ir::StructuralGoal::from_vec(body),
            kind: match predicate.predicate_kind {
                ast::PredicateKind::Relation => ir::PredicateKind::Relation,
                ast::PredicateKind::Macro => ir::PredicateKind::Macro,
            },
            visibility: self.convert_visibility(&predicate.visibility)?,
        };

        registry.replace_item(ir::Item::Predicate(updated_predicate));

        Ok(())
    }

    /// Compile impl block body
    fn compile_impl_body(
        &mut self,
        impl_block: &ast::ImplBlock,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Impl blocks are treated as modules containing predicates
        let impl_module_name = impl_block.type_name.to_string();

        // Push impl module context
        self.module_path_stack.push(impl_module_name.clone());
        let old_current = self.symbol_context.current_module.clone();

        // Find the impl module ID that was created during symbol collection
        let module_path = self.resolve_item_path(&impl_module_name);
        let module_id = ir::ModuleId::new(module_path);
        self.symbol_context.current_module = module_id;

        // Compile predicate bodies
        for predicate in &impl_block.predicates {
            self.compile_predicate_body(predicate, ir_program)?;
        }

        // Restore context
        self.module_path_stack.pop();
        self.symbol_context.current_module = old_current;

        Ok(())
    }

    /// Compile type annotation for parameters
    fn compile_type_annotation(
        &self,
        annotation: &crate::interpreter::metaprogramming::TypeAnnotation,
    ) -> Result<ir::TypeAnnotation, CompileError> {
        use crate::interpreter::metaprogramming::TypeAnnotation as AstTypeAnnotation;
        match annotation {
            AstTypeAnnotation::Int => Ok(ir::TypeAnnotation::Int),
            AstTypeAnnotation::String => Ok(ir::TypeAnnotation::String),
            AstTypeAnnotation::Bool => Ok(ir::TypeAnnotation::Bool),
            AstTypeAnnotation::Relation(arity) => Ok(ir::TypeAnnotation::Relation(*arity)),
            AstTypeAnnotation::Custom(qualified_path) => {
                let type_id = self.resolve_qualified_path_to_type(qualified_path)?;
                Ok(ir::TypeAnnotation::Custom(type_id))
            }
        }
    }

    /// Compile an AST goal to an IR goal
    fn compile_goal(&mut self, goal: &ast::Goal) -> Result<ir::Goal, CompileError> {
        match goal {
            ast::Goal::Equality(left, right, _span) => {
                let left_term = self.compile_term(left)?;
                let right_term = self.compile_term(right)?;
                Ok(ir::Goal::Equality(left_term, right_term))
            }

            ast::Goal::Disequality(left, right, _span) => {
                let left_term = self.compile_term(left)?;
                let right_term = self.compile_term(right)?;
                Ok(ir::Goal::Disequality(left_term, right_term))
            }

            ast::Goal::BooleanLiteral(value, _span) => Ok(ir::Goal::Boolean(*value)),

            ast::Goal::RelationCall(relation_call, _span) => {
                self.compile_relation_call(relation_call)
            }

            ast::Goal::Conjunction(conjunction, _span) => {
                let mut goals = Vec::new();
                for goal in conjunction.body.iter() {
                    goals.push(self.compile_goal(goal)?);
                }
                Ok(ir::Goal::Conjunction(ir::StructuralGoal::from_vec(goals)))
            }

            ast::Goal::Disjunction(disjunction, _span) => {
                let mut goals = Vec::new();
                for goal in disjunction.body.iter() {
                    goals.push(self.compile_goal(goal)?);
                }
                Ok(ir::Goal::Disjunction(ir::StructuralGoal::from_vec(goals)))
            }

            ast::Goal::Fresh(fresh_vars, _span) => self.compile_fresh_variables(fresh_vars),

            ast::Goal::Let(let_decl, _span) => self.compile_let_declaration(let_decl),

            ast::Goal::PatternMatch(pattern_match, _span) => {
                self.compile_pattern_match(pattern_match)
            }

            ast::Goal::Parenthesized(goal_body, _span) => {
                // For parenthesized goals, compile the body as a conjunction
                let mut goals = Vec::new();
                for goal in goal_body {
                    goals.push(self.compile_goal(goal)?);
                }
                // If there's only one goal, return it directly to avoid unnecessary nesting
                if goals.len() == 1 {
                    Ok(goals.into_iter().next().unwrap())
                } else {
                    Ok(ir::Goal::Conjunction(ir::StructuralGoal::from_vec(goals)))
                }
            }

            ast::Goal::ConstraintBlock(constraint_block, _span) => {
                self.compile_constraint_block(constraint_block)
            }

            ast::Goal::MethodCall(method_call, _span) => self.compile_method_call(method_call),

            ast::Goal::MetaStatement(meta_statement, _span) => {
                self.compile_meta_statement(meta_statement)
            }
        }
    }

    /// Compile an AST term to an IR term
    fn compile_term(&self, term: &ast::Term) -> Result<ir::Term, CompileError> {
        match term {
            ast::Term::Variable(name) => Ok(ir::Term::Variable(name.clone())),

            ast::Term::Wildcard(_span) => Ok(ir::Term::Wildcard),

            ast::Term::Literal(literal, _span) => {
                let ir_literal = match literal {
                    ast::Literal::Number(value) => {
                        // Try to parse as integer, default to 0 if parsing fails
                        let int_value = value.parse::<i64>().unwrap_or(0);
                        ir::Literal::Integer(int_value)
                    }
                    ast::Literal::String(value) => ir::Literal::String(value.clone().into()),
                    ast::Literal::Boolean(value) => ir::Literal::Boolean(*value),
                    ast::Literal::Char(value) => ir::Literal::Char(*value),
                };
                Ok(ir::Term::Literal(ir_literal))
            }

            ast::Term::List(list, _span) => {
                let mut elements = Vec::new();
                for element in &list.elements {
                    elements.push(self.compile_term(element)?);
                }

                let tail = if let Some(tail_term) = &list.tail {
                    Some(Box::new(self.compile_term(tail_term)?))
                } else {
                    None
                };

                Ok(ir::Term::List(ir::List { elements, tail }))
            }

            ast::Term::NamedStruct(struct_construction, _span) => {
                self.compile_named_struct_construction(struct_construction)
            }

            ast::Term::TupleStruct(struct_construction, _span) => {
                self.compile_tuple_struct_construction(struct_construction)
            }

            ast::Term::EnumVariant(enum_construction, _span) => {
                self.compile_enum_variant_construction(enum_construction)
            }

            ast::Term::Interpolation(meta_expr, _span) => {
                let compiled_meta = self.compile_meta_expression(meta_expr)?;
                Ok(ir::Term::MetaInterpolation(compiled_meta))
            }

            ast::Term::Parenthesized(inner_term, _span) => {
                // For parenthesized terms, just compile the inner term
                self.compile_term(inner_term)
            }
        }
    }

    // Stub implementations for remaining compilation methods
    // TODO: These need to be filled in with the actual implementations from compiler.rs

    fn compile_relation_call(
        &self,
        relation_call: &ast::RelationCall,
    ) -> Result<ir::Goal, CompileError> {
        // Convert RelationName to QualifiedPath
        let qualified_path = match &relation_call.name {
            ast::RelationName::Simple(name) => ast::QualifiedPath::Relative(vec![name.clone()]),
            ast::RelationName::Qualified(qualified_name) => {
                // Convert QualifiedName to QualifiedPath by combining path and name
                match &qualified_name.path {
                    ast::QualifiedPath::Global(segments) => {
                        let mut new_segments = segments.clone();
                        new_segments.push(qualified_name.name.clone());
                        ast::QualifiedPath::Global(new_segments)
                    }
                    ast::QualifiedPath::Absolute(segments) => {
                        let mut new_segments = segments.clone();
                        new_segments.push(qualified_name.name.clone());
                        ast::QualifiedPath::Absolute(new_segments)
                    }
                    ast::QualifiedPath::Relative(segments) => {
                        let mut new_segments = segments.clone();
                        new_segments.push(qualified_name.name.clone());
                        ast::QualifiedPath::Relative(new_segments)
                    }
                    ast::QualifiedPath::Super(levels, segments) => {
                        let mut new_segments = segments.clone();
                        new_segments.push(qualified_name.name.clone());
                        ast::QualifiedPath::Super(*levels, new_segments)
                    }
                    ast::QualifiedPath::Self_(segments) => {
                        let mut new_segments = segments.clone();
                        new_segments.push(qualified_name.name.clone());
                        ast::QualifiedPath::Self_(new_segments)
                    }
                    ast::QualifiedPath::External(crate_name, segments) => {
                        let mut new_segments = segments.clone();
                        new_segments.push(qualified_name.name.clone());
                        ast::QualifiedPath::External(crate_name.clone(), new_segments)
                    }
                }
            }
        };

        // Resolve the predicate reference
        let predicate_id = self.resolve_qualified_path_to_predicate(&qualified_path)?;

        // Compile arguments
        let mut arguments = Vec::new();
        for arg in &relation_call.args {
            match arg {
                ast::CallArgument::Term(term) => {
                    arguments.push(self.compile_term(term)?);
                }
                ast::CallArgument::MetaExpression(_meta_expr) => {
                    // Meta expressions are handled during preprocessing
                    return Err(CompileError::SemanticError {
                        message:
                            "Meta expressions in relation calls not supported in IR compilation"
                                .to_string(),
                        symbol: InternedSymbol::from_text("meta_expression"),
                    });
                }
            }
        }

        Ok(ir::Goal::PredicateCall(ir::PredicateCall {
            predicate: predicate_id,
            arguments,
        }))
    }

    fn compile_fresh_variables(
        &mut self,
        fresh_vars: &ast::FreshVariables,
    ) -> Result<ir::Goal, CompileError> {
        let variables = fresh_vars.vars.clone();

        // Compile the body goals
        let mut body = Vec::new();
        for goal in fresh_vars.body.iter() {
            body.push(self.compile_goal(goal)?);
        }

        Ok(ir::Goal::Fresh(ir::Fresh {
            variables,
            body: ir::StructuralGoal::from_vec(body),
        }))
    }

    fn compile_let_declaration(
        &self,
        let_decl: &ast::LetDeclaration,
    ) -> Result<ir::Goal, CompileError> {
        let variable = let_decl.var_name.clone();

        let value = if let Some(term) = &let_decl.value {
            Some(self.compile_term(term)?)
        } else {
            None
        };

        // Note: LetDeclaration in AST doesn't have a body - it's just a variable binding
        // The body would be handled by the parent construct (like Goal::Let with body)
        // For now, create an empty body
        let body = Vec::new();

        Ok(ir::Goal::Let(ir::Let {
            variable,
            value,
            body: ir::StructuralGoal::from_vec(body),
        }))
    }

    fn compile_pattern_match(
        &mut self,
        pattern_match: &ast::PatternMatching,
    ) -> Result<ir::Goal, CompileError> {
        let term = self.compile_term(&pattern_match.term)?;

        let mut arms = Vec::new();
        for arm in &pattern_match.arms {
            let pattern = self.compile_pattern(&arm.pattern)?;
            let mut body = Vec::new();
            for goal in arm.body.iter() {
                body.push(self.compile_goal(goal)?);
            }
            arms.push(ir::PatternArm {
                pattern,
                body: ir::StructuralGoal::from_vec(body),
            });
        }

        Ok(ir::Goal::PatternMatch(ir::PatternMatch { term, arms }))
    }

    fn compile_method_call(
        &self,
        _method_call: &ast::MethodCall,
    ) -> Result<ir::Goal, CompileError> {
        // Method calls are more complex and may need transformation
        // to regular predicate calls. For now, treat as unimplemented
        Err(CompileError::SemanticError {
            message: "Method call compilation not yet implemented".to_string(),
            symbol: InternedSymbol::from_text("method_call"),
        })
    }

    fn compile_named_struct_construction(
        &self,
        struct_construction: &ast::NamedStructConstruction,
    ) -> Result<ir::Term, CompileError> {
        // For named struct, the name is an InternedSymbol, need to convert to path
        let qualified_path = ast::QualifiedPath::Relative(vec![struct_construction.name.clone()]);
        let type_id = self.resolve_qualified_path_to_type(&qualified_path)?;

        let mut ir_fields = Vec::new();
        for field in &struct_construction.fields {
            let value = self.compile_term(&field.value)?;
            ir_fields.push(ir::NamedFieldConstruction {
                name: field.name.clone(),
                value,
            });
        }

        Ok(ir::Term::Struct(ir::StructConstruction {
            type_ref: type_id,
            fields: ir::StructConstructionFields::Named(ir_fields),
        }))
    }

    fn compile_tuple_struct_construction(
        &self,
        struct_construction: &ast::TupleStructConstruction,
    ) -> Result<ir::Term, CompileError> {
        let type_id = self.resolve_qualified_path_to_type(&struct_construction.name)?;

        let mut ir_fields = Vec::new();
        for arg in &struct_construction.args {
            ir_fields.push(self.compile_term(arg)?);
        }

        Ok(ir::Term::Struct(ir::StructConstruction {
            type_ref: type_id,
            fields: ir::StructConstructionFields::Tuple(ir_fields),
        }))
    }

    fn compile_enum_variant_construction(
        &self,
        enum_construction: &ast::EnumVariantConstruction,
    ) -> Result<ir::Term, CompileError> {
        // For enum construction, the enum_name is an InternedSymbol, need to convert to path
        let qualified_path =
            ast::QualifiedPath::Relative(vec![enum_construction.enum_name.clone()]);
        let enum_ref = self.resolve_qualified_path_to_type(&qualified_path)?;
        let variant_name = enum_construction.variant_name.clone();

        let kind = match &enum_construction.kind {
            ast::EnumVariantConstructionKind::Unit => ir::EnumVariantConstructionKind::Unit,

            ast::EnumVariantConstructionKind::Tuple(tuple_fields) => {
                let mut ir_fields = Vec::new();
                for field in tuple_fields {
                    ir_fields.push(self.compile_term(field)?);
                }
                ir::EnumVariantConstructionKind::Tuple(ir_fields)
            }

            ast::EnumVariantConstructionKind::Named(named_fields) => {
                let mut ir_fields = Vec::new();
                for field in named_fields {
                    let value = self.compile_term(&field.value)?;
                    ir_fields.push(ir::NamedFieldConstruction {
                        name: field.name.clone(),
                        value,
                    });
                }
                ir::EnumVariantConstructionKind::Named(ir_fields)
            }
        };

        Ok(ir::Term::EnumVariant(ir::EnumVariantConstruction {
            enum_ref,
            variant_name,
            kind,
        }))
    }

    /// Compile pattern from AST to IR
    fn compile_pattern(&self, pattern: &ast::Pattern) -> Result<ir::Pattern, CompileError> {
        match pattern {
            ast::Pattern::Variable(name) => Ok(ir::Pattern::Variable(name.clone())),

            ast::Pattern::Wildcard => Ok(ir::Pattern::Wildcard),

            ast::Pattern::Literal(literal) => {
                let ir_literal = match literal {
                    ast::Literal::Number(value) => {
                        // Try to parse as integer, default to 0 if parsing fails
                        let int_value = value.parse::<i64>().unwrap_or(0);
                        ir::Literal::Integer(int_value)
                    }
                    ast::Literal::String(value) => ir::Literal::String(value.clone().into()),
                    ast::Literal::Boolean(value) => ir::Literal::Boolean(*value),
                    ast::Literal::Char(value) => ir::Literal::Char(*value),
                };
                Ok(ir::Pattern::Literal(ir_literal))
            }

            ast::Pattern::List(list_pattern) => {
                let mut elements = Vec::new();
                for element in &list_pattern.elements {
                    elements.push(self.compile_pattern(element)?);
                }

                let tail = if let Some(tail_pattern) = &list_pattern.tail {
                    Some(Box::new(self.compile_pattern(tail_pattern)?))
                } else {
                    None
                };

                Ok(ir::Pattern::List(ir::ListPattern { elements, tail }))
            }

            ast::Pattern::NamedStruct(struct_pattern) => {
                self.compile_named_struct_pattern(struct_pattern)
            }

            ast::Pattern::TupleStruct(struct_pattern) => {
                self.compile_tuple_struct_pattern(struct_pattern)
            }

            ast::Pattern::EnumVariant(enum_pattern) => {
                self.compile_enum_variant_pattern(enum_pattern)
            }
        }
    }

    /// Compile named struct pattern
    fn compile_named_struct_pattern(
        &self,
        struct_pattern: &ast::NamedStructPattern,
    ) -> Result<ir::Pattern, CompileError> {
        // For named struct pattern, the name is an InternedSymbol, need to convert to path
        let qualified_path = ast::QualifiedPath::Relative(vec![struct_pattern.name.clone()]);
        let type_id = self.resolve_qualified_path_to_type(&qualified_path)?;

        let mut ir_patterns = Vec::new();
        for field_pattern in &struct_pattern.fields {
            let pattern = self.compile_pattern(&field_pattern.pattern)?;
            ir_patterns.push(ir::NamedFieldPattern {
                name: field_pattern.name.clone(),
                pattern,
            });
        }

        Ok(ir::Pattern::Struct(ir::StructPattern {
            type_ref: type_id,
            fields: ir::StructPatternFields::Named(ir_patterns),
        }))
    }

    /// Compile tuple struct pattern
    fn compile_tuple_struct_pattern(
        &self,
        struct_pattern: &ast::TupleStructPattern,
    ) -> Result<ir::Pattern, CompileError> {
        // For tuple struct pattern, the name is an InternedSymbol, need to convert to path
        let qualified_path = ast::QualifiedPath::Relative(vec![struct_pattern.name.clone()]);
        let type_id = self.resolve_qualified_path_to_type(&qualified_path)?;

        let mut ir_patterns = Vec::new();
        for pattern in &struct_pattern.args {
            ir_patterns.push(self.compile_pattern(pattern)?);
        }

        Ok(ir::Pattern::Struct(ir::StructPattern {
            type_ref: type_id,
            fields: ir::StructPatternFields::Tuple(ir_patterns),
        }))
    }

    /// Compile enum variant pattern
    fn compile_enum_variant_pattern(
        &self,
        enum_pattern: &ast::EnumVariantPattern,
    ) -> Result<ir::Pattern, CompileError> {
        // For enum variant pattern, the enum_name is an InternedSymbol, need to convert to path
        let qualified_path = ast::QualifiedPath::Relative(vec![enum_pattern.enum_name.clone()]);
        let enum_ref = self.resolve_qualified_path_to_type(&qualified_path)?;
        let variant_name = enum_pattern.variant_name.clone();

        let kind = match &enum_pattern.kind {
            ast::EnumVariantPatternKind::Unit => ir::EnumVariantPatternKind::Unit,

            ast::EnumVariantPatternKind::Tuple(tuple_patterns) => {
                let mut ir_patterns = Vec::new();
                for pattern in tuple_patterns {
                    ir_patterns.push(self.compile_pattern(pattern)?);
                }
                ir::EnumVariantPatternKind::Tuple(ir_patterns)
            }

            ast::EnumVariantPatternKind::Named(named_patterns) => {
                let mut ir_patterns = Vec::new();
                for field_pattern in named_patterns {
                    let pattern = self.compile_pattern(&field_pattern.pattern)?;
                    ir_patterns.push(ir::NamedFieldPattern {
                        name: field_pattern.name.clone(),
                        pattern,
                    });
                }
                ir::EnumVariantPatternKind::Named(ir_patterns)
            }
        };

        Ok(ir::Pattern::EnumVariant(ir::EnumVariantPattern {
            enum_ref,
            variant_name,
            kind,
        }))
    }

    /// Compile a meta expression from AST to IR
    fn compile_meta_expression(
        &self,
        expr: &crate::interpreter::metaprogramming::MetaExpression,
    ) -> Result<ir::MetaExpression, CompileError> {
        use crate::interpreter::metaprogramming::MetaExpression as AstMetaExpression;

        match expr {
            AstMetaExpression::Variable(name, _span) => {
                // Convert string to InternedSymbol
                let symbol = InternedSymbol::from_text(name);
                Ok(ir::MetaExpression::Variable(symbol))
            }
            AstMetaExpression::Literal(value, _span) => {
                let ir_value = self.compile_meta_value(value)?;
                Ok(ir::MetaExpression::Literal(ir_value))
            }
            AstMetaExpression::BinaryOp(op, left, right, _span) => {
                let ir_op = self.compile_meta_binary_op(op)?;
                let ir_left = Box::new(self.compile_meta_expression(left)?);
                let ir_right = Box::new(self.compile_meta_expression(right)?);
                Ok(ir::MetaExpression::BinaryOp(ir_op, ir_left, ir_right))
            }
        }
    }

    /// Compile a meta statement from AST to IR
    fn compile_meta_statement(
        &mut self,
        meta_statement: &crate::interpreter::metaprogramming::MetaStatement,
    ) -> Result<ir::Goal, CompileError> {
        use crate::interpreter::metaprogramming::MetaStatement;

        match meta_statement {
            MetaStatement::Let(let_stmt) => {
                let expression = self.compile_meta_expression(&let_stmt.expression)?;
                let variable_type = self.compile_type_annotation(&let_stmt.variable_type)?;
                Ok(ir::Goal::MetaLet(ir::MetaLet {
                    variable: let_stmt.variable.clone(),
                    variable_type,
                    expression,
                }))
            }
            MetaStatement::If {
                condition,
                then_body,
                else_ifs,
                else_body,
            } => {
                let ir_condition = self.compile_meta_expression(condition)?;

                // Compile then body
                let mut ir_then_body = Vec::new();
                for goal in then_body {
                    ir_then_body.push(self.compile_goal(goal)?);
                }

                // Compile else if branches
                let mut ir_else_ifs = Vec::new();
                for (else_if_condition, else_if_body) in else_ifs {
                    let ir_else_if_condition = self.compile_meta_expression(else_if_condition)?;
                    let mut ir_else_if_body = Vec::new();
                    for goal in else_if_body {
                        ir_else_if_body.push(self.compile_goal(goal)?);
                    }
                    ir_else_ifs.push((ir_else_if_condition, ir_else_if_body));
                }

                // Compile else body
                let ir_else_body = if let Some(else_goals) = else_body {
                    let mut ir_else_goals = Vec::new();
                    for goal in else_goals {
                        ir_else_goals.push(self.compile_goal(goal)?);
                    }
                    Some(ir_else_goals)
                } else {
                    None
                };

                Ok(ir::Goal::MetaIf(ir::MetaIf {
                    condition: ir_condition,
                    then_body: ir::StructuralGoal::from_vec(ir_then_body),
                    else_ifs: ir_else_ifs
                        .into_iter()
                        .map(|(cond, body)| (cond, ir::StructuralGoal::from_vec(body)))
                        .collect(),
                    else_body: ir_else_body.map(ir::StructuralGoal::from_vec),
                }))
            }
            MetaStatement::For {
                variable,
                variable_type,
                range,
                body,
            } => {
                let start_expr = self.compile_meta_expression(&range.start)?;
                let end_expr = self.compile_meta_expression(&range.end)?;
                let ir_variable_type = self.compile_type_annotation(variable_type)?;

                let mut ir_body = Vec::new();
                for goal in body {
                    ir_body.push(self.compile_goal(goal)?);
                }

                Ok(ir::Goal::MetaFor(ir::MetaFor {
                    variable: variable.clone(),
                    variable_type: ir_variable_type,
                    start: start_expr,
                    end: end_expr,
                    body: ir::StructuralGoal::from_vec(ir_body),
                }))
            }
        }
    }

    /// Compile a meta value from AST to IR
    fn compile_meta_value(
        &self,
        value: &crate::interpreter::metaprogramming::MetaValue,
    ) -> Result<ir::MetaValue, CompileError> {
        use crate::interpreter::metaprogramming::MetaValue as AstMetaValue;

        match value {
            AstMetaValue::Integer(i) => Ok(ir::MetaValue::Integer(*i)),
            AstMetaValue::String(s) => Ok(ir::MetaValue::String(s.as_str().into())),
            AstMetaValue::Boolean(b) => Ok(ir::MetaValue::Boolean(*b)),
        }
    }

    /// Compile a meta binary operator from AST to IR
    fn compile_meta_binary_op(
        &self,
        op: &crate::interpreter::metaprogramming::MetaBinaryOp,
    ) -> Result<ir::MetaBinaryOp, CompileError> {
        use crate::interpreter::metaprogramming::MetaBinaryOp as AstMetaBinaryOp;

        match op {
            AstMetaBinaryOp::Add => Ok(ir::MetaBinaryOp::Add),
            AstMetaBinaryOp::Subtract => Ok(ir::MetaBinaryOp::Subtract),
            AstMetaBinaryOp::Multiply => Ok(ir::MetaBinaryOp::Multiply),
            AstMetaBinaryOp::Divide => Ok(ir::MetaBinaryOp::Divide),
            AstMetaBinaryOp::LessThan => Ok(ir::MetaBinaryOp::LessThan),
            AstMetaBinaryOp::LessEqual => Ok(ir::MetaBinaryOp::LessEqual),
            AstMetaBinaryOp::GreaterThan => Ok(ir::MetaBinaryOp::GreaterThan),
            AstMetaBinaryOp::GreaterEqual => Ok(ir::MetaBinaryOp::GreaterEqual),
            AstMetaBinaryOp::Equal => Ok(ir::MetaBinaryOp::Equal),
            AstMetaBinaryOp::NotEqual => Ok(ir::MetaBinaryOp::NotEqual),
            AstMetaBinaryOp::And => Ok(ir::MetaBinaryOp::And),
            AstMetaBinaryOp::Or => Ok(ir::MetaBinaryOp::Or),
        }
    }

    /// Compile constraint block with template-based approach
    fn compile_constraint_block(
        &mut self,
        constraint_block: &ast::ConstraintBlock,
    ) -> Result<ir::Goal, CompileError> {
        use crate::interpreter::constraint_domains::{
            ConstraintDomain, VariableInfo, VariableType,
        };

        let domain_name = constraint_block.domain.as_str();

        // Get the constraint domain
        let domain = self
            .constraint_domains
            .get_domain(domain_name)
            .ok_or_else(|| CompileError::SemanticError {
                message: format!("Unknown constraint domain: {}", domain_name),
                symbol: InternedSymbol::from_text(domain_name),
            })?;

        // Get list of unbound variables that this constraint needs
        let unbound_variables = domain
            .get_unbound_variables(&constraint_block.body)
            .map_err(|err| CompileError::SemanticError {
                message: format!("Failed to get unbound variables for constraint: {}", err),
                symbol: InternedSymbol::from_text(domain_name),
            })?;

        // Look up variables in the current symbol context
        let mut resolved_variables = HashMap::new();
        for var_name in unbound_variables {
            // For now, assume all variables are relational
            // TODO: Add proper type inference/annotation to determine variable types
            let var_info = VariableInfo {
                name: var_name.clone(),
                var_type: VariableType::Relational,
            };
            resolved_variables.insert(var_name, var_info);
        }

        // Compile the constraint into an IR template
        let template = domain
            .compile_template(&constraint_block.body, resolved_variables)
            .map_err(|err| CompileError::SemanticError {
                message: format!("Failed to compile constraint template: {}", err),
                symbol: InternedSymbol::from_text(domain_name),
            })?;

        Ok(ir::Goal::Constraint(ir::ConstraintBlock {
            domain: domain_name.into(),
            template,
        }))
    }
}
