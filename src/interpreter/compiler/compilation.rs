//! AST-to-IR compilation phase
//!
//! This module handles the third phase of compilation: compiling AST bodies to IR
//! with full symbol resolution using the resolved symbol maps.

use super::ir;
use super::*;
use crate::interpreter::symbol_table;
use std::rc::Rc;

/// AST-to-IR compilation methods for the IR compiler
impl Compiler {
    /// Compile bodies of external modules (loaded from std lib etc.)
    pub(super) fn compile_external_module_bodies(
        &mut self,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Take the external module items (this empties the vec)
        let external_items = std::mem::take(&mut self.external_module_items);

        for (module_path, items) in external_items {
            // Push the module context
            self.module_path_stack.push(module_path.clone());

            // Compile each item's body
            for item in &items {
                self.compile_item_body(item, ir_program)?;
            }

            // Pop the module context
            self.module_path_stack.pop();
        }

        Ok(())
    }

    /// Phase 3: Compile bodies with full symbol resolution
    pub(super) fn compile_bodies(
        &mut self,
        program: &ast::Program,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Phase 3: Body compilation

        for item in &program.items {
            self.compile_item_body(item, ir_program)?;
        }
        Ok(())
    }

    /// Compile bodies from individual AST items (for incremental compilation)
    pub(super) fn compile_bodies_from_items(
        &mut self,
        items: &[ast::Item],
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        for item in items {
            self.compile_item_body(item, ir_program)?;
        }
        Ok(())
    }

    /// Compile the body of a single AST item
    pub(super) fn compile_item_body(
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
            ast::Item::Use(_) => {
                // Skip use statements - already processed
            }
            ast::Item::ExternCrate(_) => {
                // Skip extern crate statements - already processed
            }
            ast::Item::ModuleDeclaration(module_decl) => {
                self.compile_module_declaration_body(module_decl, ir_program)?;
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
        // Look up the module in the current module using IR registry
        let module_name = module.name.to_string();
        let module_item_id = self
            .resolve_local_symbol_with_kind(&module_name, ir::ItemKind::Module, ir_program)
            .ok_or_else(|| CompileError::UnresolvedModule {
                attempted_item: ir::ModuleId::with_parent(
                    Rc::new(ir::ModulePath::root()),
                    module_name.clone(),
                ),
                symbol: module.name.clone(),
            })?;

        // Convert ItemId to ir::ModuleId (we know it's a module from context)
        let module_id = ir::ModuleId::new(
            module_item_id.module_path.clone(),
            module_item_id.name.name.clone(),
        );

        // Push module context
        self.module_path_stack.push(module_id.full_path());

        // Compile child items
        for child_item in &module.items {
            self.compile_item_body(child_item, ir_program)?;
        }

        // Restore context
        self.module_path_stack.pop();

        Ok(())
    }

    /// Compile struct body (resolve field types)
    fn compile_struct_body(
        &mut self,
        struct_def: &ast::StructDefinition,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Look up the type in the current module using IR registry
        let type_name = struct_def.name.to_string();
        let type_item_id = self
            .resolve_local_symbol_with_kind(&type_name, ir::ItemKind::Type, ir_program)
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: ir::TypeId::with_parent(
                    self.current_module_path(),
                    type_name.clone(),
                )
                .into(),
                symbol: struct_def.name.clone(),
            })?;

        // Convert ItemId to ir::TypeId (we know it's a type from context)
        let type_id = ir::TypeId::new(
            type_item_id.module_path.clone(),
            type_item_id.name.name.clone(),
        );

        // Resolve field types
        let fields = match &struct_def.kind {
            ast::StructKind::Named(named_fields) => {
                let mut ir_fields = vec![];
                for field in named_fields {
                    let field_type_ref =
                        self.resolve_qualified_field_type(&field.type_name, ir_program)?;
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
                    let field_type_ref = self.resolve_field_type(field_type, ir_program)?;
                    ir_field_types.push(field_type_ref);
                }
                ir::StructFields::Tuple(ir_field_types)
            }
        };

        // Update the type in the registry
        let registry = ir_program.registry_mut();
        let updated_type = ir::TypeDefinition {
            id: type_id,
            kind: ir::TypeKind::Struct(ir::StructDefinition {
                name: struct_def.name.clone(),
                fields,
            }),
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
        // Look up the type in the current module using IR registry
        let type_name = enum_def.name.to_string();
        let type_item_id = self
            .resolve_local_symbol_with_kind(&type_name, ir::ItemKind::Type, ir_program)
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: ir::TypeId::with_parent(
                    self.current_module_path(),
                    type_name.clone(),
                )
                .into(),
                symbol: enum_def.name.clone(),
            })?;

        // Convert ItemId to ir::TypeId (we know it's a type from context)
        let type_id = ir::TypeId::new(
            type_item_id.module_path.clone(),
            type_item_id.name.name.clone(),
        );

        // Resolve variant types
        let mut variants = vec![];
        for variant in &enum_def.variants {
            let variant_kind = match &variant.kind {
                ast::VariantKind::Unit => ir::EnumVariantKind::Unit,
                ast::VariantKind::Tuple(field_types) => {
                    let mut ir_field_types = vec![];
                    for field_type in field_types {
                        let field_type_ref = self.resolve_field_type(field_type, ir_program)?;
                        ir_field_types.push(field_type_ref);
                    }
                    ir::EnumVariantKind::Tuple(ir_field_types)
                }
                ast::VariantKind::Named(fields) => {
                    let mut ir_fields = vec![];
                    for field in fields {
                        let field_type_ref =
                            self.resolve_qualified_field_type(&field.type_name, ir_program)?;
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
            kind: ir::TypeKind::Enum(ir::EnumDefinition {
                name: enum_def.name.clone(),
                variants,
            }),
            visibility: self.convert_visibility(&enum_def.visibility)?,
        };

        registry.replace_item(ir::Item::Type(updated_type));

        Ok(())
    }

    /// Resolve type reference from qualified path
    fn resolve_type_reference(
        &self,
        qualified_path: &ast::QualifiedPath,
        ir_program: &ir::Program,
    ) -> Result<ir::TypeId, CompileError> {
        self.resolve_qualified_path_to_type(qualified_path, ir_program)
    }

    /// Resolve qualified path to ir::TypeId using proper scoped resolution
    fn resolve_qualified_path_to_type(
        &self,
        path: &ast::QualifiedPath,
        ir_program: &ir::Program,
    ) -> Result<ir::TypeId, CompileError> {
        let segments: Vec<_> = path.segments().iter().map(|s| s.to_string()).collect();

        // Simple case: single segment - look in current module first, then check builtins
        if segments.len() == 1 {
            let type_name = &segments[0];

            // First try to find in current module using IR registry
            if let Some(type_item_id) =
                self.resolve_local_symbol_with_kind(type_name, ir::ItemKind::Type, ir_program)
            {
                return Ok(ir::TypeId::new(
                    type_item_id.module_path.clone(),
                    type_item_id.name.name.clone(),
                ));
            }

            // If not found locally, check if it's a builtin type
            if Self::is_builtin_type(type_name) {
                return Ok(ir::TypeId::with_parent(
                    Rc::new(ir::ModulePath::root()),
                    type_name.to_string(),
                ));
            }

            // Not found and not a builtin type - return error
            return Err(CompileError::UnresolvedType {
                attempted_item: ir::TypeId::with_parent(
                    self.current_module_path(),
                    type_name.clone(),
                )
                .into(),
                symbol: InternedSymbol::from_text(type_name),
            });
        }

        // Complex case: multi-segment path - use type-specific resolution
        let type_id = self.resolve_qualified_path_as_type(path, ir_program)?;

        // Check if this type exists in the IR registry
        if ir_program.registry.get_type(&type_id).is_some() {
            return Ok(type_id);
        }

        // Not found - return error with the last segment as the symbol name
        let empty_string = String::new();
        let type_name = segments.last().unwrap_or(&empty_string);
        return Err(CompileError::UnresolvedType {
            attempted_item: type_id,
            symbol: InternedSymbol::from_text(type_name),
        });
    }

    /// Resolve qualified path to either a PredicateId or a predicate variable
    fn resolve_qualified_path_to_predicate(
        &self,
        path: &ast::QualifiedPath,
        ir_program: &ir::Program,
    ) -> Result<ir::PredicateCallTarget, CompileError> {
        let segments: Vec<_> = path.segments().iter().map(|s| s.to_string()).collect();

        // Simple case: single segment - look in current module first
        if segments.len() == 1 {
            let predicate_name = &segments[0];

            // Check local scope first - parameters with rel(n) type annotations
            let predicate_symbol = InternedSymbol::from_text(predicate_name);
            if let Some(type_annotation) = self.lookup_local_symbol(&predicate_symbol) {
                // Check if this is a relational type that can be called as a predicate
                if let ir::TypeAnnotation::Relation(_arity) = type_annotation {
                    // This is a local variable holding a predicate
                    return Ok(ir::PredicateCallTarget::Variable(predicate_symbol));
                }
            }

            // Check if this might be a builtin predicate - allow to pass through for runtime resolution
            if predicate_name.starts_with("__builtin_") || predicate_name.starts_with("assert_") {
                // Builtin predicates are resolved at runtime
                return Ok(ir::PredicateCallTarget::Builtin(predicate_name.clone()));
            }

            // Try enhanced symbol resolution for simple names (includes glob imports and global items)
            if let Some(item_id) = self.resolve_local_symbol_with_kind(
                predicate_name,
                ir::ItemKind::Predicate,
                ir_program,
            ) {
                // Since resolve_local_symbol_with_kind returns ItemId for predicates,
                // we can wrap it back into PredicateId since it came from a predicate search
                return Ok(ir::PredicateCallTarget::Predicate(ir::PredicateId {
                    id: item_id,
                }));
            }
        }

        // Complex case: multi-segment path - use predicate-specific resolution
        let predicate_id = self.resolve_qualified_path_as_predicate(path, ir_program)?;
        Ok(ir::PredicateCallTarget::Predicate(predicate_id))
    }

    // TODO: Add remaining compilation methods for predicates, goals, terms, patterns, etc.
    // This is a large amount of code (over 1000 lines) that would be moved from the original
    // compiler.rs file. For now, I'll include stub methods to complete the interface.

    /// Compile module declaration body (phase 3)
    fn compile_module_declaration_body(
        &mut self,
        module_decl: &ast::ModuleDeclaration,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Get the module name and check if there's a corresponding file using generic crate resolution
        let module_name = module_decl.name.to_string();
        let path = self.resolve_module_file_path(&module_name)?;

        if path.exists() {
            // Read and parse the module file (same as in symbol collection)
            let module_contents =
                std::fs::read_to_string(&path).map_err(|e| CompileError::SemanticError {
                    message: format!("Failed to read module {}: {}", path.display(), e),
                    symbol: module_decl.name.clone(),
                })?;

            let module_ast =
                crate::interpreter::parser::parse_str(&module_contents).map_err(|e| {
                    CompileError::SemanticError {
                        message: format!("Failed to parse module {}: {}", path.display(), e),
                        symbol: module_decl.name.clone(),
                    }
                })?;

            // Create module ID directly from current context - no parsing needed
            let module_name_typed =
                ir::ItemName::new_unchecked(module_name.clone(), ir::ItemKind::Module);
            let module_id = ir::ModuleId::with_parent(
                self.current_module_path(),
                module_name_typed.name.clone(),
            );

            // Push child module context for compiling its contents
            self.module_path_stack.push(module_id.full_path());

            // Recursively compile bodies from the module file
            for item in &module_ast.items {
                self.compile_item_body(item, ir_program)?;
            }

            // Restore original module context
            self.module_path_stack.pop();
        }
        // If file doesn't exist, nothing to compile

        Ok(())
    }

    /// Compile predicate body (goals, parameters, etc.)
    fn compile_predicate_body(
        &mut self,
        predicate: &ast::PredicateDefinition,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Look up the predicate using IR registry
        let predicate_name = predicate.name.to_string();

        // Find predicate using IR registry
        let predicate_item_id = self
            .resolve_local_symbol_with_kind(&predicate_name, ir::ItemKind::Predicate, ir_program)
            .ok_or_else(|| CompileError::UnresolvedPredicate {
                attempted_item: ir::PredicateId::with_parent(
                    self.current_module_path(),
                    predicate_name.clone(),
                ),
                symbol: predicate.name.clone(),
            })?;

        // Convert ItemId to PredicateId (we know it's a predicate from context)
        let predicate_id = ir::PredicateId::new(
            predicate_item_id.module_path.clone(),
            predicate_item_id.name.name.clone(),
        );

        // Compile parameters
        let mut parameters = vec![];
        for param in &predicate.parameters {
            let type_annotation = if let Some(annotation) = &param.type_annotation {
                Some(self.compile_type_annotation(annotation, ir_program)?)
            } else {
                None
            };

            parameters.push(ir::Parameter {
                name: param.name.clone(),
                type_annotation,
            });
        }

        // For DCGs, add difference list parameters (Input, Output)
        if predicate.predicate_kind == ast::PredicateKind::Grammar {
            let input_param = ir::Parameter {
                name: InternedSymbol::from_text("Input"),
                type_annotation: None,
            };
            let output_param = ir::Parameter {
                name: InternedSymbol::from_text("Output"),
                type_annotation: None,
            };
            parameters.push(input_param);
            parameters.push(output_param);
        }

        // Push new scope for predicate parameters
        self.push_local_scope();

        // Add parameters to local scope
        for param in &parameters {
            if let Some(type_annotation) = &param.type_annotation {
                self.add_local_symbol(param.name.clone(), type_annotation.clone());
            }
        }

        // Compile body goals
        let body = if predicate.predicate_kind == ast::PredicateKind::Grammar {
            // For DCGs, transform the body to thread difference lists
            let dcg_goals = self.compile_dcg_body(&predicate.body, ir_program)?;
            dcg_goals
        } else {
            // For regular relations, compile goals normally
            let mut goals = vec![];
            for goal in predicate.body.iter() {
                goals.push(self.compile_goal(goal, ir_program)?);
            }
            goals
        };

        // Pop the predicate parameter scope
        self.pop_local_scope();

        // Update the predicate in the registry
        let registry = ir_program.registry_mut();
        let updated_predicate = ir::Predicate {
            id: predicate_id,
            parameters,
            body: ir::StructuralGoal::from_vec(body),
            kind: match predicate.predicate_kind {
                ast::PredicateKind::Relation => ir::PredicateKind::Relation,
                ast::PredicateKind::Macro => ir::PredicateKind::Macro,
                ast::PredicateKind::Grammar => ir::PredicateKind::Grammar,
            },
            visibility: self.convert_visibility(&predicate.visibility)?,
        };

        registry.replace_item(ir::Item::Predicate(updated_predicate));

        Ok(())
    }

    /// Compile DCG body goals, transforming them to thread difference lists
    fn compile_dcg_body(
        &mut self,
        body: &[ast::Goal],
        ir_program: &mut ir::Program,
    ) -> Result<Vec<ir::Goal>, CompileError> {
        let mut compiled_goals = vec![];
        let mut current_list_var = InternedSymbol::from_text("Input");
        let mut intermediate_vars = vec![];

        // Process each goal in sequence, threading the difference lists
        for (i, goal) in body.iter().enumerate() {
            let next_list_var = if i == body.len() - 1 {
                // Last goal uses Output
                InternedSymbol::from_text("Output")
            } else {
                // Intermediate goal uses a fresh variable with compiler prefix
                let var_name = InternedSymbol::from_text(&format!("__dcg_rest_{}", i));
                intermediate_vars.push(var_name.clone());
                var_name
            };

            let compiled_goal =
                self.compile_dcg_goal(goal, current_list_var, next_list_var.clone(), ir_program)?;
            compiled_goals.push(compiled_goal);

            current_list_var = next_list_var;
        }

        // If we have intermediate variables, wrap the goals in a Fresh goal
        if !intermediate_vars.is_empty() {
            let fresh_goal = ir::Goal::Fresh(ir::Fresh {
                variables: intermediate_vars,
                body: ir::StructuralGoal::from_vec(compiled_goals),
            });
            Ok(vec![fresh_goal])
        } else {
            Ok(compiled_goals)
        }
    }

    /// Compile a single DCG goal, adding difference list parameters
    fn compile_dcg_goal(
        &mut self,
        goal: &ast::Goal,
        input_var: InternedSymbol,
        output_var: InternedSymbol,
        ir_program: &mut ir::Program,
    ) -> Result<ir::Goal, CompileError> {
        match goal {
            ast::Goal::RelationCall(call, _span) => {
                // Transform DCG relation call to include difference lists
                let mut args = vec![];

                // Add original arguments
                for arg in &call.args {
                    // Convert ast::CallArgument to ir::Term
                    match arg {
                        ast::CallArgument::Term(term) => {
                            let compiled_term = self.compile_term(term, ir_program)?;
                            args.push(compiled_term);
                        }
                        ast::CallArgument::MetaExpression(_meta_expr) => {
                            // For now, DCG doesn't support meta expressions in arguments
                            // This could be extended in the future
                            return Err(CompileError::SemanticError {
                                message: "Meta expressions are not supported in DCG arguments"
                                    .to_string(),
                                symbol: InternedSymbol::from_text("dcg_meta"),
                            });
                        }
                    }
                }

                // Add difference list arguments
                args.push(ir::Term::Variable(input_var));
                args.push(ir::Term::Variable(output_var));

                // Use the same structure as regular relation calls
                // Convert RelationName to QualifiedPath
                let qualified_path = match &call.name {
                    ast::RelationName::Simple(name) => {
                        ast::QualifiedPath::Relative(vec![name.clone()])
                    }
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
                let predicate_target =
                    self.resolve_qualified_path_to_predicate(&qualified_path, ir_program)?;

                Ok(ir::Goal::PredicateCall(ir::PredicateCall {
                    target: predicate_target,
                    arguments: args,
                }))
            }
            ast::Goal::PatternMatch(pattern_match, _span) => {
                // Handle pattern matching in DCG context by adding Input/Output variables to scope
                // and compiling pattern match with DCG goal compilation for arms
                self.compile_dcg_pattern_match(pattern_match, input_var, output_var, ir_program)
            }
            ast::Goal::Conjunction(conjunction, _span) => {
                // For conjunctions in DCG, we need to thread the difference lists through
                // each goal in the conjunction
                let mut compiled_goals = vec![];
                let mut current_input = input_var;
                let mut intermediate_vars = vec![];

                for (i, goal) in conjunction.body.iter().enumerate() {
                    let next_output = if i == conjunction.body.len() - 1 {
                        // Last goal uses the final output
                        output_var.clone()
                    } else {
                        // Intermediate goal uses a fresh variable
                        let var_name = InternedSymbol::from_text(&format!("__dcg_conj_{}", i));
                        intermediate_vars.push(var_name.clone());
                        var_name
                    };

                    let compiled_goal = self.compile_dcg_goal(
                        goal,
                        current_input,
                        next_output.clone(),
                        ir_program,
                    )?;
                    compiled_goals.push(compiled_goal);
                    current_input = next_output;
                }

                // Wrap in fresh variables if needed
                if !intermediate_vars.is_empty() {
                    Ok(ir::Goal::Fresh(ir::Fresh {
                        variables: intermediate_vars,
                        body: ir::StructuralGoal::from_vec(compiled_goals),
                    }))
                } else {
                    Ok(ir::Goal::Conjunction(ir::StructuralGoal::from_vec(
                        compiled_goals,
                    )))
                }
            }
            ast::Goal::Disjunction(disjunction, _span) => {
                // For disjunctions in DCG, each alternative should use the same Input and Output vars
                // Each branch gets the same input and should produce the same output
                let mut compiled_goals = vec![];

                for goal in disjunction.body.iter() {
                    let compiled_goal = self.compile_dcg_goal(
                        goal,
                        input_var.clone(),
                        output_var.clone(),
                        ir_program,
                    )?;
                    compiled_goals.push(compiled_goal);
                }

                Ok(ir::Goal::Disjunction(ir::StructuralGoal::from_vec(
                    compiled_goals,
                )))
            }
            _ => {
                // For non-DCG goals like unification, compile normally
                // These goals don't need difference list threading but should
                // have access to Input/Output variables in their scope
                self.compile_goal(goal, ir_program)
            }
        }
    }

    /// Compile pattern matching in DCG context with difference list variables in scope
    fn compile_dcg_pattern_match(
        &mut self,
        pattern_match: &ast::PatternMatching,
        input_var: InternedSymbol,
        output_var: InternedSymbol,
        ir_program: &mut ir::Program,
    ) -> Result<ir::Goal, CompileError> {
        // Compile the term being matched
        let compiled_term = self.compile_term(&pattern_match.term, ir_program)?;

        // Compile each arm with DCG goal compilation
        let mut compiled_arms = vec![];
        for arm in &pattern_match.arms {
            let compiled_pattern = self.compile_pattern(&arm.pattern, ir_program)?;

            // Compile guard if present
            let compiled_guard = if let Some(guard_goal) = &arm.guard {
                Some(self.compile_goal(guard_goal, ir_program)?)
            } else {
                None
            };

            // Compile body with DCG goal compilation to handle difference list threading
            let mut compiled_body_goals = vec![];
            // arm.body is a Vec<Goal> (GoalBody)
            if arm.body.is_empty() {
                // Empty body - just unify Input with Output
                compiled_body_goals.push(ir::Goal::Equality(
                    ir::Term::Variable(input_var.clone()),
                    ir::Term::Variable(output_var.clone()),
                ));
            } else if arm.body.len() == 1 {
                // Single goal
                let compiled_goal = self.compile_dcg_goal(
                    &arm.body[0],
                    input_var.clone(),
                    output_var.clone(),
                    ir_program,
                )?;
                compiled_body_goals.push(compiled_goal);
            } else {
                // Multiple goals - thread difference lists through them
                let mut current_input = input_var.clone();
                let mut intermediate_vars = vec![];

                for (i, goal) in arm.body.iter().enumerate() {
                    let next_output = if i == arm.body.len() - 1 {
                        output_var.clone()
                    } else {
                        let var_name = InternedSymbol::from_text(&format!("__dcg_arm_{}", i));
                        intermediate_vars.push(var_name.clone());
                        var_name
                    };

                    let compiled_goal = self.compile_dcg_goal(
                        goal,
                        current_input,
                        next_output.clone(),
                        ir_program,
                    )?;
                    compiled_body_goals.push(compiled_goal);
                    current_input = next_output;
                }

                // Wrap in fresh if needed
                if !intermediate_vars.is_empty() {
                    let fresh_goal = ir::Goal::Fresh(ir::Fresh {
                        variables: intermediate_vars,
                        body: ir::StructuralGoal::from_vec(compiled_body_goals),
                    });
                    compiled_body_goals = vec![fresh_goal];
                }
            }

            compiled_arms.push(ir::PatternArm {
                pattern: compiled_pattern,
                guard: compiled_guard,
                body: ir::StructuralGoal::from_vec(compiled_body_goals),
            });
        }

        Ok(ir::Goal::PatternMatch(ir::PatternMatch {
            term: compiled_term,
            arms: compiled_arms,
        }))
    }

    /// Compile impl block body
    fn compile_impl_body(
        &mut self,
        impl_block: &ast::ImplBlock,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Impl blocks are treated as modules containing predicates
        let impl_module_name = impl_block.type_name.to_string();

        // Create impl module ID directly from current context - no parsing needed
        let module_name_typed =
            ir::ItemName::new_unchecked(impl_module_name.clone(), ir::ItemKind::Module);
        let module_id =
            ir::ModuleId::with_parent(self.current_module_path(), module_name_typed.name.clone());

        // Push impl module context
        self.module_path_stack.push(module_id.full_path());

        // Compile predicate bodies
        for predicate in &impl_block.predicates {
            self.compile_predicate_body(predicate, ir_program)?;
        }

        // Restore context
        self.module_path_stack.pop();

        Ok(())
    }

    /// Compile type annotation for parameters
    fn compile_type_annotation(
        &self,
        annotation: &crate::interpreter::metaprogramming::TypeAnnotation,
        ir_program: &ir::Program,
    ) -> Result<ir::TypeAnnotation, CompileError> {
        use crate::interpreter::metaprogramming::TypeAnnotation as AstTypeAnnotation;
        match annotation {
            // Non-relational types (for meta expressions)
            AstTypeAnnotation::Int => Ok(ir::TypeAnnotation::Int),
            AstTypeAnnotation::String => Ok(ir::TypeAnnotation::String),
            AstTypeAnnotation::Bool => Ok(ir::TypeAnnotation::Bool),
            // Relational built-in types (for logic terms)
            AstTypeAnnotation::RelInt => Ok(ir::TypeAnnotation::RelInt),
            AstTypeAnnotation::RelString => Ok(ir::TypeAnnotation::RelString),
            AstTypeAnnotation::RelBool => Ok(ir::TypeAnnotation::RelBool),
            AstTypeAnnotation::RelChar => Ok(ir::TypeAnnotation::RelChar),
            AstTypeAnnotation::LTerm => Ok(ir::TypeAnnotation::LTerm),
            AstTypeAnnotation::Relation(arity) => Ok(ir::TypeAnnotation::Relation(*arity)),
            AstTypeAnnotation::Custom(qualified_path) => {
                let type_id = self.resolve_qualified_path_to_type(qualified_path, ir_program)?;
                Ok(ir::TypeAnnotation::Custom(type_id))
            }
        }
    }

    /// Compile an AST goal to an IR goal
    pub(super) fn compile_goal(
        &mut self,
        goal: &ast::Goal,
        ir_program: &ir::Program,
    ) -> Result<ir::Goal, CompileError> {
        match goal {
            ast::Goal::Equality(left, right, _span) => {
                let left_term = self.compile_term(left, ir_program)?;
                let right_term = self.compile_term(right, ir_program)?;
                Ok(ir::Goal::Equality(left_term, right_term))
            }

            ast::Goal::Disequality(left, right, _span) => {
                let left_term = self.compile_term(left, ir_program)?;
                let right_term = self.compile_term(right, ir_program)?;
                Ok(ir::Goal::Disequality(left_term, right_term))
            }

            ast::Goal::BooleanLiteral(value, _span) => Ok(ir::Goal::Boolean(*value)),

            ast::Goal::RelationCall(relation_call, _span) => {
                self.compile_relation_call(relation_call, ir_program)
            }

            ast::Goal::Conjunction(conjunction, _span) => {
                let mut goals = Vec::new();
                for goal in conjunction.body.iter() {
                    goals.push(self.compile_goal(goal, ir_program)?);
                }
                Ok(ir::Goal::Conjunction(ir::StructuralGoal::from_vec(goals)))
            }

            ast::Goal::Disjunction(disjunction, _span) => {
                let mut goals = Vec::new();
                for goal in disjunction.body.iter() {
                    goals.push(self.compile_goal(goal, ir_program)?);
                }
                Ok(ir::Goal::Disjunction(ir::StructuralGoal::from_vec(goals)))
            }

            ast::Goal::Fresh(fresh_vars, _span) => {
                self.compile_fresh_variables(fresh_vars, ir_program)
            }

            ast::Goal::Let(let_decl, _span) => self.compile_let_declaration(let_decl, ir_program),

            ast::Goal::PatternMatch(pattern_match, _span) => {
                self.compile_pattern_match(pattern_match, ir_program)
            }

            ast::Goal::Parenthesized(goal_body, _span) => {
                // For parenthesized goals, compile the body as a conjunction
                let mut goals = Vec::new();
                for goal in goal_body {
                    goals.push(self.compile_goal(goal, ir_program)?);
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

            ast::Goal::MethodCall(method_call, _span) => {
                self.compile_method_call(method_call, ir_program)
            }

            ast::Goal::MetaStatement(meta_statement, _span) => {
                self.compile_meta_statement(meta_statement, ir_program)
            }
        }
    }

    /// Compile an AST term to an IR term
    fn compile_term(
        &self,
        term: &ast::Term,
        ir_program: &ir::Program,
    ) -> Result<ir::Term, CompileError> {
        match term {
            ast::Term::Variable(name) => {
                // Check if this Variable is actually a unit enum variant
                // This serves as a fallback for cases where semantic analysis wasn't performed
                if let Some(enum_variant) =
                    self.try_disambiguate_variable_as_enum_variant(name, ir_program)?
                {
                    return self.compile_enum_variant_construction(&enum_variant, ir_program);
                }

                // TODO: Check if this variable has a rel(n) type annotation and should be converted to predicate reference
                // For now, we'll handle this through the parameter type system in predicate calls

                Ok(ir::Term::Variable(name.clone()))
            }

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
                    elements.push(self.compile_term(element, ir_program)?);
                }

                let tail = if let Some(tail_term) = &list.tail {
                    Some(Box::new(self.compile_term(tail_term, ir_program)?))
                } else {
                    None
                };

                Ok(ir::Term::List(ir::List { elements, tail }))
            }

            ast::Term::NamedStruct(struct_construction, _span) => {
                self.compile_named_struct_construction(struct_construction, ir_program)
            }

            ast::Term::TupleStruct(struct_construction, _span) => {
                self.compile_tuple_struct_construction(struct_construction, ir_program)
            }

            ast::Term::EnumVariant(enum_construction, _span) => {
                self.compile_enum_variant_construction(enum_construction, ir_program)
            }

            ast::Term::Interpolation(meta_expr, _span) => {
                let compiled_meta = self.compile_meta_expression(meta_expr)?;
                Ok(ir::Term::MetaInterpolation(compiled_meta))
            }

            ast::Term::Parenthesized(inner_term, _span) => {
                // For parenthesized terms, just compile the inner term
                self.compile_term(inner_term, ir_program)
            }
        }
    }

    // Stub implementations for remaining compilation methods
    // TODO: These need to be filled in with the actual implementations from compiler.rs

    fn compile_relation_call(
        &self,
        relation_call: &ast::RelationCall,
        ir_program: &ir::Program,
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
        let predicate_target =
            self.resolve_qualified_path_to_predicate(&qualified_path, ir_program)?;

        // Validate builtin predicates exist and have correct arity
        if let ir::PredicateCallTarget::Builtin(builtin_name) = &predicate_target {
            // We need access to the environment, but it's not directly available here
            // For now, we'll validate using the builtin registry
            let registry = crate::interpreter::builtins::get_builtin_registry();

            match registry.get(builtin_name) {
                Some(&expected_arity) => {
                    if expected_arity != relation_call.args.len() {
                        return Err(CompileError::SemanticError {
                            message: format!(
                                "Builtin predicate '{}' expects {} arguments, but {} were provided",
                                builtin_name,
                                expected_arity,
                                relation_call.args.len()
                            ),
                            symbol: InternedSymbol::from_text(builtin_name),
                        });
                    }
                }
                None => {
                    return Err(CompileError::SemanticError {
                        message: format!("Unknown builtin predicate '{}'", builtin_name),
                        symbol: InternedSymbol::from_text(builtin_name),
                    });
                }
            }
        }

        // Look up predicate to check parameter types for macro calls
        let predicate_def = match &predicate_target {
            ir::PredicateCallTarget::Predicate(predicate_id) => {
                ir_program.registry.get_predicate(predicate_id)
            }
            _ => None, // Variables and builtins don't have definitions in the registry
        };

        // Compile arguments
        let mut arguments = Vec::new();
        for (i, arg) in relation_call.args.iter().enumerate() {
            match arg {
                ast::CallArgument::Term(term) => {
                    // Check if this should be a meta value for macro parameters
                    let should_be_meta = if let Some(predicate) = predicate_def {
                        if predicate.kind == ir::PredicateKind::Macro
                            && i < predicate.parameters.len()
                        {
                            matches!(
                                predicate.parameters[i].type_annotation,
                                Some(ir::TypeAnnotation::Int)
                                    | Some(ir::TypeAnnotation::String)
                                    | Some(ir::TypeAnnotation::Bool)
                            )
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if should_be_meta {
                        // Convert compatible literals to meta expressions for macro parameters
                        match term {
                            ast::Term::Literal(literal, _location) => {
                                let meta_value = match literal {
                                    ast::Literal::Number(num_str) => {
                                        // Parse string to integer for meta parameter
                                        match num_str.parse::<i64>() {
                                            Ok(i) => ir::MetaValue::Integer(i),
                                            Err(_) => {
                                                return Err(CompileError::SemanticError {
                                                    message: format!("Invalid integer literal for meta parameter: {}", num_str),
                                                    symbol: InternedSymbol::from_text("literal"),
                                                });
                                            }
                                        }
                                    }
                                    ast::Literal::String(s) => {
                                        ir::MetaValue::String(s.as_str().into())
                                    }
                                    ast::Literal::Boolean(b) => ir::MetaValue::Boolean(*b),
                                    ast::Literal::Char(_) => {
                                        return Err(CompileError::SemanticError {
                                            message: "Character literals cannot be used for meta parameters".to_string(),
                                            symbol: InternedSymbol::from_text("char_literal"),
                                        });
                                    }
                                };
                                arguments.push(ir::Term::MetaInterpolation(
                                    ir::MetaExpression::Literal(meta_value),
                                ));
                            }
                            _ => {
                                let param_name = if let Some(predicate) = predicate_def {
                                    predicate.parameters[i].name.to_string()
                                } else {
                                    format!("parameter_{}", i)
                                };
                                return Err(CompileError::SemanticError {
                                    message: format!(
                                        "Meta parameter '{}' expects a literal value",
                                        param_name
                                    ),
                                    symbol: InternedSymbol::from_text(&param_name),
                                });
                            }
                        }
                    } else {
                        // Regular relational parameter - check if it should be a predicate reference
                        let compiled_arg = if self.should_convert_arg_to_predicate_ref(
                            &predicate_target,
                            i,
                            term,
                            predicate_def,
                            ir_program,
                        )? {
                            // Convert variable to predicate reference for higher-order predicates
                            self.compile_term_as_predicate_ref(term, ir_program)?
                        } else {
                            self.compile_term(term, ir_program)?
                        };
                        arguments.push(compiled_arg);
                    }
                }
                ast::CallArgument::MetaExpression(meta_expr) => {
                    // Compile meta expressions for runtime evaluation in macro calls
                    let compiled_meta = self.compile_meta_expression(meta_expr)?;
                    arguments.push(ir::Term::MetaInterpolation(compiled_meta));
                }
            }
        }

        Ok(ir::Goal::PredicateCall(ir::PredicateCall {
            target: predicate_target,
            arguments,
        }))
    }

    fn compile_fresh_variables(
        &mut self,
        fresh_vars: &ast::FreshVariables,
        ir_program: &ir::Program,
    ) -> Result<ir::Goal, CompileError> {
        // Push new scope for fresh variables
        self.push_local_scope();

        // Extract variable names and add them to local scope
        let mut variables = Vec::new();
        for param in &fresh_vars.vars {
            variables.push(param.name.clone());

            // Add the fresh variable to local scope with its type annotation
            if let Some(type_annotation) = &param.type_annotation {
                let ir_type_annotation =
                    self.compile_type_annotation(type_annotation, ir_program)?;
                self.add_local_symbol(param.name.clone(), ir_type_annotation);
            }
        }

        // Compile the body goals within the fresh variable scope
        let mut body = Vec::new();
        for goal in fresh_vars.body.iter() {
            body.push(self.compile_goal(goal, ir_program)?);
        }

        // Pop the fresh variable scope
        self.pop_local_scope();

        Ok(ir::Goal::Fresh(ir::Fresh {
            variables,
            body: ir::StructuralGoal::from_vec(body),
        }))
    }

    fn compile_let_declaration(
        &self,
        let_decl: &ast::LetDeclaration,
        ir_program: &ir::Program,
    ) -> Result<ir::Goal, CompileError> {
        let variable = let_decl.var_name.clone();

        let value = if let Some(term) = &let_decl.value {
            Some(self.compile_term(term, ir_program)?)
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
        ir_program: &ir::Program,
    ) -> Result<ir::Goal, CompileError> {
        let term = self.compile_term(&pattern_match.term, ir_program)?;

        let mut arms = Vec::new();
        for arm in &pattern_match.arms {
            // Push new scope for pattern variables in this match arm
            self.push_local_scope();

            let pattern = self.compile_pattern(&arm.pattern, ir_program)?;

            // Bind compiled pattern variables to local scope
            self.bind_compiled_pattern_variables(&pattern)?;

            // Compile guard if present (pattern variables are available in scope)
            let guard = match &arm.guard {
                Some(guard_goal) => Some(self.compile_goal(guard_goal, ir_program)?),
                None => None,
            };

            let mut body = Vec::new();
            for goal in arm.body.iter() {
                body.push(self.compile_goal(goal, ir_program)?);
            }

            // Pop the pattern variable scope
            self.pop_local_scope();

            arms.push(ir::PatternArm {
                pattern,
                guard,
                body: ir::StructuralGoal::from_vec(body),
            });
        }

        Ok(ir::Goal::PatternMatch(ir::PatternMatch { term, arms }))
    }

    /// Bind compiled pattern variables to the current local scope
    fn bind_compiled_pattern_variables(
        &mut self,
        pattern: &ir::Pattern,
    ) -> Result<(), CompileError> {
        match pattern {
            ir::Pattern::Variable(var_name) => {
                // Add the pattern variable to the local scope with LTerm type annotation
                self.add_local_symbol(var_name.clone(), ir::TypeAnnotation::LTerm);
            }
            ir::Pattern::List(list_pattern) => {
                // Bind variables in list elements
                for element in &list_pattern.elements {
                    self.bind_compiled_pattern_variables(element)?;
                }
                // Bind variables in tail if present
                if let Some(tail) = &list_pattern.tail {
                    self.bind_compiled_pattern_variables(tail)?;
                }
            }
            ir::Pattern::Struct(struct_pattern) => {
                // Bind variables in struct fields
                match &struct_pattern.fields {
                    ir::StructPatternFields::Named(named_fields) => {
                        for field in named_fields {
                            self.bind_compiled_pattern_variables(&field.pattern)?;
                        }
                    }
                    ir::StructPatternFields::Tuple(tuple_fields) => {
                        for field_pattern in tuple_fields {
                            self.bind_compiled_pattern_variables(field_pattern)?;
                        }
                    }
                }
            }
            ir::Pattern::EnumVariant(enum_pattern) => {
                // Bind variables in enum variant fields
                match &enum_pattern.kind {
                    ir::EnumVariantPatternKind::Tuple(tuple_fields) => {
                        for field_pattern in tuple_fields {
                            self.bind_compiled_pattern_variables(field_pattern)?;
                        }
                    }
                    ir::EnumVariantPatternKind::Named(named_fields) => {
                        for field in named_fields {
                            self.bind_compiled_pattern_variables(&field.pattern)?;
                        }
                    }
                    ir::EnumVariantPatternKind::Unit => {
                        // No variables to bind in unit variants
                    }
                }
            }
            ir::Pattern::Literal(_) | ir::Pattern::Wildcard => {
                // No variables to bind in literals or wildcards
            }
        }
        Ok(())
    }

    fn compile_method_call(
        &self,
        method_call: &ast::MethodCall,
        ir_program: &ir::Program,
    ) -> Result<ir::Goal, CompileError> {
        // Transform method call into a regular predicate call
        // receiver.method(args...) becomes TypeName::method(receiver, args...)

        // First, we need to determine the type of the receiver
        let receiver_type_name = self.extract_receiver_type(&method_call.receiver)?;

        // Create a qualified path for the method: TypeName::method_name
        let method_path = ast::QualifiedPath::Relative(vec![
            receiver_type_name.clone(),
            method_call.method.clone(),
        ]);

        // Resolve the method as a predicate
        let predicate_target =
            self.resolve_qualified_path_to_predicate(&method_path, ir_program)?;

        // Compile the receiver as the first argument
        let receiver_term = self.compile_term(&method_call.receiver, ir_program)?;
        let mut arguments = vec![receiver_term];

        // Compile the rest of the arguments
        for arg in &method_call.args {
            arguments.push(self.compile_term(arg, ir_program)?);
        }

        // Create the predicate call
        Ok(ir::Goal::PredicateCall(ir::PredicateCall {
            target: predicate_target,
            arguments,
        }))
    }

    /// Extract the type name from a receiver term for method calls
    fn extract_receiver_type(&self, receiver: &ast::Term) -> Result<InternedSymbol, CompileError> {
        match receiver {
            ast::Term::NamedStruct(named_struct, _) => Ok(named_struct.name.clone()),
            ast::Term::TupleStruct(tuple_struct, _) => {
                // For tuple structs, extract the final segment from the qualified path
                let segments = tuple_struct.name.segments();
                if let Some(type_name) = segments.last() {
                    Ok(type_name.clone())
                } else {
                    Err(CompileError::SemanticError {
                        message: "Cannot extract type name from empty qualified path".to_string(),
                        symbol: InternedSymbol::from_text("tuple_struct"),
                    })
                }
            }
            ast::Term::Variable(var_name) => {
                // For variables, we'll need type inference in the future
                // For now, return an error asking for explicit type annotation
                Err(CompileError::SemanticError {
                    message: format!("Cannot determine type of variable '{}' for method call. Consider using explicit struct construction or type annotation", var_name),
                    symbol: var_name.clone(),
                })
            }
            _ => Err(CompileError::SemanticError {
                message: "Method calls are only supported on struct instances".to_string(),
                symbol: InternedSymbol::from_text("method_receiver"),
            }),
        }
    }

    fn compile_named_struct_construction(
        &self,
        struct_construction: &ast::NamedStructConstruction,
        ir_program: &ir::Program,
    ) -> Result<ir::Term, CompileError> {
        // Check if this might be an enum variant with named fields (semantic disambiguation)
        // For names like "Shape::Rectangle", check if this is parsed as a single name containing "::"
        let struct_name_str = struct_construction.name.to_string();
        if let Some(colon_pos) = struct_name_str.find("::") {
            // This looks like "Shape::Rectangle" - could be an enum variant
            let potential_enum_name = &struct_name_str[..colon_pos];
            let potential_variant_name = &struct_name_str[colon_pos + 2..];

            // Try to resolve the enum part as a type using IR registry
            if let Some(type_item_id) = self.resolve_local_symbol_with_kind(
                potential_enum_name,
                ir::ItemKind::Type,
                ir_program,
            ) {
                let enum_ref = ir::TypeId::new(
                    type_item_id.module_path.clone(),
                    type_item_id.name.name.clone(),
                );

                // STRICT VALIDATION: Look up the actual enum definition to validate syntax
                let type_def = ir_program.registry.get_type(&enum_ref).ok_or_else(|| {
                    CompileError::UnresolvedType {
                        attempted_item: enum_ref.clone(),
                        symbol: InternedSymbol::from_text(potential_enum_name),
                    }
                })?;

                // Extract enum definition and find the specific variant
                let enum_def = match &type_def.kind {
                    ir::TypeKind::Enum(enum_def) => enum_def,
                    _ => {
                        return Err(CompileError::SemanticError {
                            message: format!("'{}' is not an enum type", potential_enum_name),
                            symbol: InternedSymbol::from_text(potential_enum_name),
                        })
                    }
                };

                let variant_def = enum_def
                    .variants
                    .iter()
                    .find(|v| v.name.to_string() == potential_variant_name)
                    .ok_or_else(|| CompileError::UnresolvedType {
                        attempted_item: ir::TypeId::with_parent(
                            self.current_module_path(),
                            format!("{}::{}", potential_enum_name, potential_variant_name),
                        ),
                        symbol: InternedSymbol::from_text(potential_variant_name),
                    })?;

                // STRICT VALIDATION: Ensure named syntax matches variant definition
                match &variant_def.kind {
                    // Unit variant: wrong syntax! Should use unit syntax
                    ir::EnumVariantKind::Unit => {
                        return Err(CompileError::SemanticError {
                            message: format!(
                                "Enum variant '{}::{}' is a unit variant and cannot use named field syntax. Correct syntax: '{}::{}'",
                                potential_enum_name, potential_variant_name, potential_enum_name, potential_variant_name
                            ),
                            symbol: InternedSymbol::from_text(potential_variant_name),
                        });
                    }

                    // Tuple variant: wrong syntax! Should use tuple syntax
                    ir::EnumVariantKind::Tuple(expected_types) => {
                        return Err(CompileError::SemanticError {
                            message: format!(
                                "Enum variant '{}::{}' is a tuple variant and requires tuple syntax with {} arguments. Correct syntax: '{}::{}(...)'",
                                potential_enum_name, potential_variant_name, expected_types.len(), potential_enum_name, potential_variant_name
                            ),
                            symbol: InternedSymbol::from_text(potential_variant_name),
                        });
                    }

                    // Named variant: correct syntax, validate fields
                    ir::EnumVariantKind::Named(expected_fields) => {
                        // Validate field count
                        if struct_construction.fields.len() != expected_fields.len() {
                            return Err(CompileError::SemanticError {
                                message: format!(
                                    "Enum variant '{}::{}' expects {} fields, found {}",
                                    potential_enum_name,
                                    potential_variant_name,
                                    expected_fields.len(),
                                    struct_construction.fields.len()
                                ),
                                symbol: InternedSymbol::from_text(potential_variant_name),
                            });
                        }

                        // Validate field names exist
                        for field in &struct_construction.fields {
                            if !expected_fields
                                .iter()
                                .any(|ef| ef.name.to_string() == field.name.to_string())
                            {
                                return Err(CompileError::SemanticError {
                                    message: format!(
                                        "Enum variant '{}::{}' has no field named '{}'",
                                        potential_enum_name,
                                        potential_variant_name,
                                        field.name.to_string()
                                    ),
                                    symbol: field.name.clone(),
                                });
                            }
                        }

                        // Compile the fields as enum variant named fields
                        let mut ir_fields = Vec::new();
                        for field in &struct_construction.fields {
                            let value = self.compile_term(&field.value, ir_program)?;
                            ir_fields.push(ir::NamedFieldConstruction {
                                name: field.name.clone(),
                                value,
                            });
                        }

                        return Ok(ir::Term::EnumVariant(ir::EnumVariantConstruction {
                            enum_ref,
                            variant_name: InternedSymbol::from_text(potential_variant_name),
                            kind: ir::EnumVariantConstructionKind::Named(ir_fields),
                        }));
                    }
                }
            }
        }

        // If not an enum variant, treat as regular named struct
        // Look up the struct type using IR registry
        let struct_name = struct_construction.name.to_string();
        let type_item_id = self
            .resolve_local_symbol_with_kind(&struct_name, ir::ItemKind::Type, ir_program)
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: ir::TypeId::with_parent(
                    self.current_module_path(),
                    struct_name.clone(),
                ),
                symbol: struct_construction.name.clone(),
            })?;

        let type_id = ir::TypeId::new(
            type_item_id.module_path.clone(),
            type_item_id.name.name.clone(),
        );

        let mut ir_fields = Vec::new();
        for field in &struct_construction.fields {
            let value = self.compile_term(&field.value, ir_program)?;
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
        ir_program: &ir::Program,
    ) -> Result<ir::Term, CompileError> {
        // Check if this is actually an enum variant (semantic disambiguation)
        // For paths like Color::Red, try to resolve Color as a type first
        if let ast::QualifiedPath::Relative(segments) = &struct_construction.name {
            if segments.len() == 2 {
                // This could be an enum variant like Color::Red
                let potential_enum_name = &segments[0];
                let potential_variant_name = &segments[1];

                // Use IR registry as source of truth instead of fragile symbol maps
                // Try to find the enum type directly in the current program's registry
                let enum_name_str = potential_enum_name.to_string();
                // Create enum type ID directly from current context - no parsing needed
                let potential_enum_type_id =
                    ir::TypeId::with_parent(self.current_module_path(), enum_name_str.clone());

                // Try semantic disambiguation: look up the enum type in the IR registry
                if let Some(enum_type_item) = ir_program.registry.get_type(&potential_enum_type_id)
                {
                    // Found the enum type! Check if it's actually an enum
                    if let ir::TypeKind::Enum(enum_def) = &enum_type_item.kind {
                        // This is an enum type like Color - validate that the variant exists
                        let variant_name_str = potential_variant_name.to_string();

                        // Check if this variant exists in the enum
                        if let Some(variant_def) = enum_def
                            .variants
                            .iter()
                            .find(|v| v.name.to_string() == variant_name_str)
                        {
                            // STRICT VALIDATION: Ensure tuple syntax matches variant definition
                            match &variant_def.kind {
                                // Unit variant: must not have arguments
                                ir::EnumVariantKind::Unit => {
                                    if !struct_construction.args.is_empty() {
                                        return Err(CompileError::SemanticError {
                                            message: format!(
                                                "Enum variant '{}::{}' is a unit variant and cannot take arguments. Found {} arguments, expected 0. Correct syntax: '{}::{}'",
                                                enum_name_str, variant_name_str, struct_construction.args.len(), enum_name_str, variant_name_str
                                            ),
                                            symbol: potential_variant_name.clone(),
                                        });
                                    }
                                    return Ok(ir::Term::EnumVariant(
                                        ir::EnumVariantConstruction {
                                            enum_ref: potential_enum_type_id,
                                            variant_name: potential_variant_name.clone(),
                                            kind: ir::EnumVariantConstructionKind::Unit,
                                        },
                                    ));
                                }

                                // Tuple variant: correct syntax, validate arity
                                ir::EnumVariantKind::Tuple(expected_types) => {
                                    if struct_construction.args.len() != expected_types.len() {
                                        return Err(CompileError::SemanticError {
                                            message: format!(
                                                "Enum variant '{}::{}' expects {} arguments, found {}",
                                                enum_name_str, variant_name_str, expected_types.len(), struct_construction.args.len()
                                            ),
                                            symbol: potential_variant_name.clone(),
                                        });
                                    }

                                    // Compile the arguments
                                    let mut ir_fields = Vec::new();
                                    for arg in &struct_construction.args {
                                        ir_fields.push(self.compile_term(arg, ir_program)?);
                                    }

                                    return Ok(ir::Term::EnumVariant(
                                        ir::EnumVariantConstruction {
                                            enum_ref: potential_enum_type_id,
                                            variant_name: potential_variant_name.clone(),
                                            kind: ir::EnumVariantConstructionKind::Tuple(ir_fields),
                                        },
                                    ));
                                }

                                // Named variant: wrong syntax! Should use named field syntax
                                ir::EnumVariantKind::Named(expected_fields) => {
                                    let field_names: Vec<String> = expected_fields
                                        .iter()
                                        .map(|f| f.name.to_string())
                                        .collect();
                                    return Err(CompileError::SemanticError {
                                        message: format!(
                                            "Enum variant '{}::{}' is a named variant and requires named field syntax, not tuple syntax. Correct syntax: '{}::{} {{ {} }}'",
                                            enum_name_str, variant_name_str, enum_name_str, variant_name_str,
                                            field_names.iter().map(|name| format!("{}: ...", name)).collect::<Vec<_>>().join(", ")
                                        ),
                                        symbol: potential_variant_name.clone(),
                                    });
                                }
                            }
                        } else {
                            // Found the enum but variant doesn't exist - this is an error
                            let variant_name_typed = ir::ItemName::new_unchecked(
                                variant_name_str.clone(),
                                ir::ItemKind::Type,
                            );
                            return Err(CompileError::UnresolvedType {
                                attempted_item: ir::TypeId::with_parent(
                                    self.current_module_path(),
                                    variant_name_typed.name.clone(),
                                ),
                                symbol: potential_variant_name.clone(),
                            });
                        }
                    } else {
                        // Found type but it's not an enum, continue to regular struct compilation
                    }
                }
                // Not found or not an enum, continue to regular struct compilation
            }
        }

        // If not an enum variant, treat as regular tuple struct
        let type_id = self.resolve_qualified_path_to_type(&struct_construction.name, ir_program)?;

        let mut ir_fields = Vec::new();
        for arg in &struct_construction.args {
            ir_fields.push(self.compile_term(arg, ir_program)?);
        }

        Ok(ir::Term::Struct(ir::StructConstruction {
            type_ref: type_id,
            fields: ir::StructConstructionFields::Tuple(ir_fields),
        }))
    }

    fn compile_enum_variant_construction(
        &self,
        enum_construction: &ast::EnumVariantConstruction,
        ir_program: &ir::Program,
    ) -> Result<ir::Term, CompileError> {
        // Look up the enum type using IR registry
        let enum_name = enum_construction.enum_name.to_string();
        let type_item_id = self
            .resolve_local_symbol_with_kind(&enum_name, ir::ItemKind::Type, ir_program)
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: ir::TypeId::with_parent(
                    self.current_module_path(),
                    enum_name.clone(),
                ),
                symbol: enum_construction.enum_name.clone(),
            })?;

        let enum_ref = ir::TypeId::new(
            type_item_id.module_path.clone(),
            type_item_id.name.name.clone(),
        );
        let variant_name = enum_construction.variant_name.clone();

        // STRICT SEMANTIC VALIDATION: Look up the actual enum definition to validate syntax
        let type_def = ir_program.registry.get_type(&enum_ref).ok_or_else(|| {
            CompileError::UnresolvedType {
                attempted_item: enum_ref.clone(),
                symbol: enum_construction.enum_name.clone(),
            }
        })?;

        // Extract enum definition and find the specific variant
        let enum_def = match &type_def.kind {
            ir::TypeKind::Enum(enum_def) => enum_def,
            _ => {
                return Err(CompileError::SemanticError {
                    message: format!("'{enum_name}' is not an enum type"),
                    symbol: enum_construction.enum_name.clone(),
                })
            }
        };

        let variant_def = enum_def
            .variants
            .iter()
            .find(|v| v.name.to_string() == variant_name.to_string())
            .ok_or_else(|| {
                let variant_name_typed =
                    ir::ItemName::new_unchecked(variant_name.to_string(), ir::ItemKind::Type);
                CompileError::UnresolvedType {
                    attempted_item: ir::TypeId::with_parent(
                        self.current_module_path(),
                        variant_name_typed.name.clone(),
                    ),
                    symbol: variant_name.clone(),
                }
            })?;

        // STRICT VALIDATION: Ensure construction syntax matches variant definition
        let kind = match (&enum_construction.kind, &variant_def.kind) {
            // Unit variant: must use unit syntax
            (ast::EnumVariantConstructionKind::Unit, ir::EnumVariantKind::Unit) => {
                ir::EnumVariantConstructionKind::Unit
            }

            // Unit variant with arguments: ERROR
            (ast::EnumVariantConstructionKind::Tuple(_), ir::EnumVariantKind::Unit)
            | (ast::EnumVariantConstructionKind::Named(_), ir::EnumVariantKind::Unit) => {
                let args_count = match &enum_construction.kind {
                    ast::EnumVariantConstructionKind::Tuple(args) => args.len(),
                    ast::EnumVariantConstructionKind::Named(args) => args.len(),
                    _ => 0,
                };
                return Err(CompileError::SemanticError {
                    message: format!(
                        "Enum variant '{}::{}' is a unit variant and cannot take arguments. Found {} arguments, expected 0. Correct syntax: '{}::{}'",
                        enum_name, variant_name.to_string(), args_count, enum_name, variant_name.to_string()
                    ),
                    symbol: variant_name.clone(),
                });
            }

            // Tuple variant: must use tuple syntax
            (
                ast::EnumVariantConstructionKind::Tuple(tuple_fields),
                ir::EnumVariantKind::Tuple(expected_types),
            ) => {
                // Validate arity
                if tuple_fields.len() != expected_types.len() {
                    return Err(CompileError::SemanticError {
                        message: format!(
                            "Enum variant '{}::{}' expects {} arguments, found {}",
                            enum_name,
                            variant_name.to_string(),
                            expected_types.len(),
                            tuple_fields.len()
                        ),
                        symbol: variant_name.clone(),
                    });
                }

                let mut ir_fields = Vec::new();
                for field in tuple_fields {
                    ir_fields.push(self.compile_term(field, ir_program)?);
                }
                ir::EnumVariantConstructionKind::Tuple(ir_fields)
            }

            // Tuple variant with wrong syntax: ERROR
            (
                ast::EnumVariantConstructionKind::Unit,
                ir::EnumVariantKind::Tuple(expected_types),
            ) => {
                return Err(CompileError::SemanticError {
                    message: format!(
                        "Enum variant '{}::{}' is a tuple variant and requires {} arguments. Correct syntax: '{}::{}(...)'",
                        enum_name, variant_name.to_string(), expected_types.len(), enum_name, variant_name.to_string()
                    ),
                    symbol: variant_name.clone(),
                });
            }

            (
                ast::EnumVariantConstructionKind::Named(_),
                ir::EnumVariantKind::Tuple(expected_types),
            ) => {
                return Err(CompileError::SemanticError {
                    message: format!(
                        "Enum variant '{}::{}' is a tuple variant and requires tuple syntax with {} arguments. Correct syntax: '{}::{}(...)'",
                        enum_name, variant_name.to_string(), expected_types.len(), enum_name, variant_name.to_string()
                    ),
                    symbol: variant_name.clone(),
                });
            }

            // Named variant: must use named syntax
            (
                ast::EnumVariantConstructionKind::Named(named_fields),
                ir::EnumVariantKind::Named(expected_fields),
            ) => {
                // Validate field count
                if named_fields.len() != expected_fields.len() {
                    return Err(CompileError::SemanticError {
                        message: format!(
                            "Enum variant '{}::{}' expects {} fields, found {}",
                            enum_name,
                            variant_name.to_string(),
                            expected_fields.len(),
                            named_fields.len()
                        ),
                        symbol: variant_name.clone(),
                    });
                }

                // Validate field names exist
                for field in named_fields {
                    if !expected_fields
                        .iter()
                        .any(|ef| ef.name.to_string() == field.name.to_string())
                    {
                        return Err(CompileError::SemanticError {
                            message: format!(
                                "Enum variant '{}::{}' has no field named '{}'",
                                enum_name,
                                variant_name.to_string(),
                                field.name.to_string()
                            ),
                            symbol: field.name.clone(),
                        });
                    }
                }

                let mut ir_fields = Vec::new();
                for field in named_fields {
                    let value = self.compile_term(&field.value, ir_program)?;
                    ir_fields.push(ir::NamedFieldConstruction {
                        name: field.name.clone(),
                        value,
                    });
                }
                ir::EnumVariantConstructionKind::Named(ir_fields)
            }

            // Named variant with wrong syntax: ERROR
            (
                ast::EnumVariantConstructionKind::Unit,
                ir::EnumVariantKind::Named(expected_fields),
            ) => {
                let field_names: Vec<String> =
                    expected_fields.iter().map(|f| f.name.to_string()).collect();
                return Err(CompileError::SemanticError {
                    message: format!(
                        "Enum variant '{}::{}' is a named variant and requires named field syntax. Correct syntax: '{}::{} {{ {} }}'",
                        enum_name, variant_name.to_string(), enum_name, variant_name.to_string(),
                        field_names.iter().map(|name| format!("{}: ...", name)).collect::<Vec<_>>().join(", ")
                    ),
                    symbol: variant_name.clone(),
                });
            }

            (
                ast::EnumVariantConstructionKind::Tuple(_),
                ir::EnumVariantKind::Named(expected_fields),
            ) => {
                let field_names: Vec<String> =
                    expected_fields.iter().map(|f| f.name.to_string()).collect();
                return Err(CompileError::SemanticError {
                    message: format!(
                        "Enum variant '{}::{}' is a named variant and requires named field syntax, not tuple syntax. Correct syntax: '{}::{} {{ {} }}'",
                        enum_name, variant_name.to_string(), enum_name, variant_name.to_string(),
                        field_names.iter().map(|name| format!("{}: ...", name)).collect::<Vec<_>>().join(", ")
                    ),
                    symbol: variant_name.clone(),
                });
            }
        };

        Ok(ir::Term::EnumVariant(ir::EnumVariantConstruction {
            enum_ref,
            variant_name,
            kind,
        }))
    }

    /// Compile pattern from AST to IR
    fn compile_pattern(
        &self,
        pattern: &ast::Pattern,
        ir_program: &ir::Program,
    ) -> Result<ir::Pattern, CompileError> {
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
                    elements.push(self.compile_pattern(element, ir_program)?);
                }

                let tail = if let Some(tail_pattern) = &list_pattern.tail {
                    Some(Box::new(self.compile_pattern(tail_pattern, ir_program)?))
                } else {
                    None
                };

                Ok(ir::Pattern::List(ir::ListPattern { elements, tail }))
            }

            ast::Pattern::NamedStruct(struct_pattern) => {
                self.compile_named_struct_pattern(struct_pattern, ir_program)
            }

            ast::Pattern::TupleStruct(struct_pattern) => {
                self.compile_tuple_struct_pattern(struct_pattern, ir_program)
            }

            ast::Pattern::EnumVariant(enum_pattern) => {
                self.compile_enum_variant_pattern(enum_pattern, ir_program)
            }
        }
    }

    /// Compile named struct pattern
    fn compile_named_struct_pattern(
        &self,
        struct_pattern: &ast::NamedStructPattern,
        ir_program: &ir::Program,
    ) -> Result<ir::Pattern, CompileError> {
        // Check if this might be an enum variant pattern with named fields (semantic disambiguation)
        let struct_name_str = struct_pattern.name.to_string();
        if let Some(colon_pos) = struct_name_str.find("::") {
            // This looks like "Shape::Rectangle" - could be an enum variant pattern
            let potential_enum_name = &struct_name_str[..colon_pos];
            let potential_variant_name = &struct_name_str[colon_pos + 2..];

            // Try to resolve the enum part as a type using IR registry
            if let Some(type_item_id) = self.resolve_local_symbol_with_kind(
                potential_enum_name,
                ir::ItemKind::Type,
                ir_program,
            ) {
                // This is a type! Treat as enum variant pattern with named fields
                let enum_ref = ir::TypeId::new(
                    type_item_id.module_path.clone(),
                    type_item_id.name.name.clone(),
                );

                // Compile the field patterns as enum variant named field patterns
                let mut ir_patterns = Vec::new();
                for field_pattern in &struct_pattern.fields {
                    let pattern = self.compile_pattern(&field_pattern.pattern, ir_program)?;
                    ir_patterns.push(ir::NamedFieldPattern {
                        name: field_pattern.name.clone(),
                        pattern,
                    });
                }

                return Ok(ir::Pattern::EnumVariant(ir::EnumVariantPattern {
                    enum_ref: ir::TypeReference::UserDefined(enum_ref),
                    variant_name: InternedSymbol::from_text(potential_variant_name),
                    kind: ir::EnumVariantPatternKind::Named(ir_patterns),
                }));
            }
        }

        // If not an enum variant, treat as regular named struct pattern
        // Look up the struct type using IR registry
        let struct_name = struct_pattern.name.to_string();
        let type_item_id = self
            .resolve_local_symbol_with_kind(&struct_name, ir::ItemKind::Type, ir_program)
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: ir::TypeId::with_parent(
                    self.current_module_path(),
                    struct_name.clone(),
                ),
                symbol: struct_pattern.name.clone(),
            })?;

        let type_id = ir::TypeId::new(
            type_item_id.module_path.clone(),
            type_item_id.name.name.clone(),
        );

        let mut ir_patterns = Vec::new();
        for field_pattern in &struct_pattern.fields {
            let pattern = self.compile_pattern(&field_pattern.pattern, ir_program)?;
            ir_patterns.push(ir::NamedFieldPattern {
                name: field_pattern.name.clone(),
                pattern,
            });
        }

        Ok(ir::Pattern::Struct(ir::StructPattern {
            type_ref: ir::TypeReference::UserDefined(type_id),
            fields: ir::StructPatternFields::Named(ir_patterns),
        }))
    }

    /// Compile tuple struct pattern
    fn compile_tuple_struct_pattern(
        &self,
        struct_pattern: &ast::TupleStructPattern,
        ir_program: &ir::Program,
    ) -> Result<ir::Pattern, CompileError> {
        // Check if this might be an enum variant pattern (semantic disambiguation)
        let struct_name_str = struct_pattern.name.to_string();
        if let Some(colon_pos) = struct_name_str.find("::") {
            // This looks like "Color::Red" - could be an enum variant pattern
            let potential_enum_name = &struct_name_str[..colon_pos];
            let potential_variant_name = &struct_name_str[colon_pos + 2..];

            // Try to resolve the enum part as a type using IR registry
            if let Some(type_item_id) = self.resolve_local_symbol_with_kind(
                potential_enum_name,
                ir::ItemKind::Type,
                ir_program,
            ) {
                // This is a type! Treat as enum variant pattern with tuple fields
                let enum_ref = ir::TypeId::new(
                    type_item_id.module_path.clone(),
                    type_item_id.name.name.clone(),
                );

                // Compile the argument patterns as enum variant tuple field patterns
                let mut ir_patterns = Vec::new();
                for pattern in &struct_pattern.args {
                    ir_patterns.push(self.compile_pattern(pattern, ir_program)?);
                }

                return Ok(ir::Pattern::EnumVariant(ir::EnumVariantPattern {
                    enum_ref: ir::TypeReference::UserDefined(enum_ref),
                    variant_name: InternedSymbol::from_text(potential_variant_name),
                    kind: ir::EnumVariantPatternKind::Tuple(ir_patterns),
                }));
            }
        }

        // If not an enum variant, treat as regular tuple struct pattern
        // Look up the struct type using IR registry
        let struct_name = struct_pattern.name.to_string();
        let type_item_id = self
            .resolve_local_symbol_with_kind(&struct_name, ir::ItemKind::Type, ir_program)
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: ir::TypeId::with_parent(
                    self.current_module_path(),
                    struct_name.clone(),
                ),
                symbol: struct_pattern.name.clone(),
            })?;

        let type_id = ir::TypeId::new(
            type_item_id.module_path.clone(),
            type_item_id.name.name.clone(),
        );

        let mut ir_patterns = Vec::new();
        for pattern in &struct_pattern.args {
            ir_patterns.push(self.compile_pattern(pattern, ir_program)?);
        }

        Ok(ir::Pattern::Struct(ir::StructPattern {
            type_ref: ir::TypeReference::UserDefined(type_id),
            fields: ir::StructPatternFields::Tuple(ir_patterns),
        }))
    }

    /// Compile enum variant pattern
    fn compile_enum_variant_pattern(
        &self,
        enum_pattern: &ast::EnumVariantPattern,
        ir_program: &ir::Program,
    ) -> Result<ir::Pattern, CompileError> {
        // Look up the enum type using IR registry
        let enum_name = enum_pattern.enum_name.to_string();
        let type_item_id = self
            .resolve_local_symbol_with_kind(&enum_name, ir::ItemKind::Type, ir_program)
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: ir::TypeId::with_parent(
                    self.current_module_path(),
                    enum_name.clone(),
                ),
                symbol: enum_pattern.enum_name.clone(),
            })?;

        let enum_ref = ir::TypeId::new(
            type_item_id.module_path.clone(),
            type_item_id.name.name.clone(),
        );
        let variant_name = enum_pattern.variant_name.clone();

        // STRICT SEMANTIC VALIDATION: Look up the actual enum definition to validate pattern syntax
        let type_def = ir_program.registry.get_type(&enum_ref).ok_or_else(|| {
            CompileError::UnresolvedType {
                attempted_item: enum_ref.clone(),
                symbol: enum_pattern.enum_name.clone(),
            }
        })?;

        // Extract enum definition and find the specific variant
        let enum_def = match &type_def.kind {
            ir::TypeKind::Enum(enum_def) => enum_def,
            _ => {
                return Err(CompileError::SemanticError {
                    message: format!("'{enum_name}' is not an enum type"),
                    symbol: enum_pattern.enum_name.clone(),
                })
            }
        };

        let variant_def = enum_def
            .variants
            .iter()
            .find(|v| v.name.to_string() == variant_name.to_string())
            .ok_or_else(|| {
                let variant_name_typed =
                    ir::ItemName::new_unchecked(variant_name.to_string(), ir::ItemKind::Type);
                CompileError::UnresolvedType {
                    attempted_item: ir::TypeId::with_parent(
                        self.current_module_path(),
                        variant_name_typed.name.clone(),
                    ),
                    symbol: variant_name.clone(),
                }
            })?;

        // STRICT VALIDATION: Ensure pattern syntax matches variant definition
        let kind = match (&enum_pattern.kind, &variant_def.kind) {
            // Unit variant: must use unit pattern syntax
            (ast::EnumVariantPatternKind::Unit, ir::EnumVariantKind::Unit) => {
                ir::EnumVariantPatternKind::Unit
            }

            // Unit variant with pattern arguments: ERROR
            (ast::EnumVariantPatternKind::Tuple(_), ir::EnumVariantKind::Unit)
            | (ast::EnumVariantPatternKind::Named(_), ir::EnumVariantKind::Unit) => {
                let patterns_count = match &enum_pattern.kind {
                    ast::EnumVariantPatternKind::Tuple(patterns) => patterns.len(),
                    ast::EnumVariantPatternKind::Named(patterns) => patterns.len(),
                    _ => 0,
                };
                return Err(CompileError::SemanticError {
                    message: format!(
                        "Enum variant '{}::{}' is a unit variant and cannot take pattern arguments. Found {} patterns, expected 0. Correct syntax: '{}::{}'",
                        enum_name, variant_name.to_string(), patterns_count, enum_name, variant_name.to_string()
                    ),
                    symbol: variant_name.clone(),
                });
            }

            // Tuple variant: must use tuple pattern syntax
            (
                ast::EnumVariantPatternKind::Tuple(tuple_patterns),
                ir::EnumVariantKind::Tuple(expected_types),
            ) => {
                // Validate arity
                if tuple_patterns.len() != expected_types.len() {
                    return Err(CompileError::SemanticError {
                        message: format!(
                            "Enum variant '{}::{}' expects {} pattern arguments, found {}",
                            enum_name,
                            variant_name.to_string(),
                            expected_types.len(),
                            tuple_patterns.len()
                        ),
                        symbol: variant_name.clone(),
                    });
                }

                let mut ir_patterns = Vec::new();
                for pattern in tuple_patterns {
                    ir_patterns.push(self.compile_pattern(pattern, ir_program)?);
                }
                ir::EnumVariantPatternKind::Tuple(ir_patterns)
            }

            // Tuple variant with wrong pattern syntax: ERROR
            (ast::EnumVariantPatternKind::Unit, ir::EnumVariantKind::Tuple(expected_types)) => {
                return Err(CompileError::SemanticError {
                    message: format!(
                        "Enum variant '{}::{}' is a tuple variant and requires {} pattern arguments. Correct syntax: '{}::{}(...)'",
                        enum_name, variant_name.to_string(), expected_types.len(), enum_name, variant_name.to_string()
                    ),
                    symbol: variant_name.clone(),
                });
            }

            (ast::EnumVariantPatternKind::Named(_), ir::EnumVariantKind::Tuple(expected_types)) => {
                return Err(CompileError::SemanticError {
                    message: format!(
                        "Enum variant '{}::{}' is a tuple variant and requires tuple pattern syntax with {} arguments. Correct syntax: '{}::{}(...)'",
                        enum_name, variant_name.to_string(), expected_types.len(), enum_name, variant_name.to_string()
                    ),
                    symbol: variant_name.clone(),
                });
            }

            // Named variant: must use named pattern syntax
            (
                ast::EnumVariantPatternKind::Named(named_patterns),
                ir::EnumVariantKind::Named(expected_fields),
            ) => {
                // Validate field count
                if named_patterns.len() != expected_fields.len() {
                    return Err(CompileError::SemanticError {
                        message: format!(
                            "Enum variant '{}::{}' expects {} field patterns, found {}",
                            enum_name,
                            variant_name.to_string(),
                            expected_fields.len(),
                            named_patterns.len()
                        ),
                        symbol: variant_name.clone(),
                    });
                }

                // Validate field names exist
                for field_pattern in named_patterns {
                    if !expected_fields
                        .iter()
                        .any(|ef| ef.name.to_string() == field_pattern.name.to_string())
                    {
                        return Err(CompileError::SemanticError {
                            message: format!(
                                "Enum variant '{}::{}' has no field named '{}'",
                                enum_name,
                                variant_name.to_string(),
                                field_pattern.name.to_string()
                            ),
                            symbol: field_pattern.name.clone(),
                        });
                    }
                }

                let mut ir_patterns = Vec::new();
                for field_pattern in named_patterns {
                    let pattern = self.compile_pattern(&field_pattern.pattern, ir_program)?;
                    ir_patterns.push(ir::NamedFieldPattern {
                        name: field_pattern.name.clone(),
                        pattern,
                    });
                }
                ir::EnumVariantPatternKind::Named(ir_patterns)
            }

            // Named variant with wrong pattern syntax: ERROR
            (ast::EnumVariantPatternKind::Unit, ir::EnumVariantKind::Named(expected_fields)) => {
                let field_names: Vec<String> =
                    expected_fields.iter().map(|f| f.name.to_string()).collect();
                return Err(CompileError::SemanticError {
                    message: format!(
                        "Enum variant '{}::{}' is a named variant and requires named field pattern syntax. Correct syntax: '{}::{} {{ {} }}'",
                        enum_name, variant_name.to_string(), enum_name, variant_name.to_string(),
                        field_names.iter().map(|name| format!("{}: ...", name)).collect::<Vec<_>>().join(", ")
                    ),
                    symbol: variant_name.clone(),
                });
            }

            (
                ast::EnumVariantPatternKind::Tuple(_),
                ir::EnumVariantKind::Named(expected_fields),
            ) => {
                let field_names: Vec<String> =
                    expected_fields.iter().map(|f| f.name.to_string()).collect();
                return Err(CompileError::SemanticError {
                    message: format!(
                        "Enum variant '{}::{}' is a named variant and requires named field pattern syntax, not tuple syntax. Correct syntax: '{}::{} {{ {} }}'",
                        enum_name, variant_name.to_string(), enum_name, variant_name.to_string(),
                        field_names.iter().map(|name| format!("{}: ...", name)).collect::<Vec<_>>().join(", ")
                    ),
                    symbol: variant_name.clone(),
                });
            }
        };

        Ok(ir::Pattern::EnumVariant(ir::EnumVariantPattern {
            enum_ref: ir::TypeReference::UserDefined(enum_ref),
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
        ir_program: &ir::Program,
    ) -> Result<ir::Goal, CompileError> {
        use crate::interpreter::metaprogramming::MetaStatement;

        match meta_statement {
            MetaStatement::Let(let_stmt) => {
                let expression = self.compile_meta_expression(&let_stmt.expression)?;
                let variable_type =
                    self.compile_type_annotation(&let_stmt.variable_type, ir_program)?;
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
                    ir_then_body.push(self.compile_goal(goal, ir_program)?);
                }

                // Compile else if branches
                let mut ir_else_ifs = Vec::new();
                for (else_if_condition, else_if_body) in else_ifs {
                    let ir_else_if_condition = self.compile_meta_expression(else_if_condition)?;
                    let mut ir_else_if_body = Vec::new();
                    for goal in else_if_body {
                        ir_else_if_body.push(self.compile_goal(goal, ir_program)?);
                    }
                    ir_else_ifs.push((ir_else_if_condition, ir_else_if_body));
                }

                // Compile else body
                let ir_else_body = if let Some(else_goals) = else_body {
                    let mut ir_else_goals = Vec::new();
                    for goal in else_goals {
                        ir_else_goals.push(self.compile_goal(goal, ir_program)?);
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
                let ir_variable_type = self.compile_type_annotation(variable_type, ir_program)?;

                let mut ir_body = Vec::new();
                for goal in body {
                    ir_body.push(self.compile_goal(goal, ir_program)?);
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
        use crate::interpreter::constraint_domains::VariableType;

        let domain_name = constraint_block.domain.as_str();

        // Get the constraint domain
        let domain = self
            .constraint_domains
            .get_domain(domain_name)
            .ok_or_else(|| CompileError::SemanticError {
                message: format!("Unknown constraint domain: {}", domain_name),
                symbol: InternedSymbol::from_text(domain_name),
            })?;

        // Create a binder closure that validates variables in the current context
        let binder = |_var_name: &str| -> Option<VariableType> {
            // For now, assume all variables are relational
            // TODO: Add proper type inference/annotation to determine variable types
            Some(VariableType::Relational)
        };

        // Compile the constraint into an IR template using the new API
        let template = domain
            .compile(&constraint_block.body, &binder)
            .map_err(|err| CompileError::SemanticError {
                message: format!("Failed to compile constraint template: {}", err),
                symbol: InternedSymbol::from_text(domain_name),
            })?;

        Ok(ir::Goal::Constraint(ir::ConstraintBlock {
            domain: domain_name.into(),
            template,
        }))
    }

    /// Check if a type name refers to a builtin primitive type
    /// These types are handled internally by the compiler and not registered in the IR registry
    fn is_builtin_type(type_name: &str) -> bool {
        matches!(type_name, "Bool" | "Number" | "Char" | "String" | "LTerm")
            || (type_name.starts_with("rel(") && type_name.ends_with(")"))
    }

    /// Convert a field type (InternedSymbol) to TypeReference, handling built-in types properly
    fn resolve_field_type(
        &self,
        field_type: &symbol_table::InternedSymbol,
        ir_program: &mut ir::Program,
    ) -> Result<ir::TypeReference, CompileError> {
        // Check if this is a built-in type first to avoid qualified path processing
        if let Some(builtin_type) = ir::BuiltinType::from_str(&field_type.to_string()) {
            Ok(ir::TypeReference::Builtin(builtin_type))
        } else {
            // Convert InternedSymbol to qualified path for non-builtin types
            let qualified_path = ast::QualifiedPath::Relative(vec![field_type.clone()]);
            let type_id = self.resolve_qualified_path_to_type(&qualified_path, ir_program)?;
            Ok(ir::TypeReference::UserDefined(type_id))
        }
    }

    /// Convert a qualified path field type to TypeReference, handling built-in types properly
    fn resolve_qualified_field_type(
        &self,
        qualified_path: &ast::QualifiedPath,
        ir_program: &mut ir::Program,
    ) -> Result<ir::TypeReference, CompileError> {
        // For simple relative paths, check if it's a built-in type first
        if let ast::QualifiedPath::Relative(segments) = qualified_path {
            if segments.len() == 1 {
                let type_name = &segments[0];
                if let Some(builtin_type) = ir::BuiltinType::from_str(&type_name.to_string()) {
                    return Ok(ir::TypeReference::Builtin(builtin_type));
                }
            }
        }

        // For non-builtin types or complex paths, resolve to TypeId
        let type_id = self.resolve_qualified_path_to_type(qualified_path, ir_program)?;
        Ok(ir::TypeReference::UserDefined(type_id))
    }

    /// Try to disambiguate a Variable as a unit enum variant (IR compiler fallback)
    /// Returns Some(EnumVariantConstruction) if the Variable should be a unit enum variant, None otherwise
    fn try_disambiguate_variable_as_enum_variant(
        &self,
        variable_name: &InternedSymbol,
        ir_program: &ir::Program,
    ) -> Result<Option<ast::EnumVariantConstruction>, CompileError> {
        // Check if the variable name contains "::" which indicates potential enum variant
        if let Some((enum_name, variant_name)) = variable_name.to_string().rsplit_once("::") {
            // Try to resolve the enum type using IR registry
            if let Some(type_item_id) =
                self.resolve_local_symbol_with_kind(enum_name, ir::ItemKind::Type, ir_program)
            {
                let type_id = ir::TypeId::new(
                    type_item_id.module_path.clone(),
                    type_item_id.name.name.clone(),
                );

                // Check if this type exists and is an enum
                if let Some(type_def) = ir_program.registry.get_type(&type_id) {
                    if let ir::TypeKind::Enum(enum_def) = &type_def.kind {
                        // Check if this variant exists in the enum and is a unit variant
                        if let Some(variant_def) = enum_def
                            .variants
                            .iter()
                            .find(|v| v.name.to_string() == variant_name)
                        {
                            // Only disambiguate unit variants (no arguments)
                            if let ir::EnumVariantKind::Unit = &variant_def.kind {
                                return Ok(Some(ast::EnumVariantConstruction {
                                    enum_name: InternedSymbol::from_text(enum_name),
                                    variant_name: InternedSymbol::from_text(variant_name),
                                    kind: ast::EnumVariantConstructionKind::Unit,
                                }));
                            }
                        } else {
                            // Found the enum but variant doesn't exist - this is an error
                            let variant_name_typed = ir::ItemName::new_unchecked(
                                variant_name.to_string(),
                                ir::ItemKind::Type,
                            );
                            return Err(CompileError::UnresolvedType {
                                attempted_item: ir::TypeId::with_parent(
                                    self.current_module_path(),
                                    variant_name_typed.name.clone(),
                                ),
                                symbol: InternedSymbol::from_text(variant_name),
                            });
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Check if an argument should be converted to a predicate reference for higher-order predicates
    fn should_convert_arg_to_predicate_ref(
        &self,
        predicate_target: &ir::PredicateCallTarget,
        param_index: usize,
        term: &ast::Term,
        predicate_def: Option<&ir::Predicate>,
        ir_program: &ir::Program,
    ) -> Result<bool, CompileError> {
        // Check if the predicate parameter has a rel(n) type annotation
        if let Some(predicate) = predicate_def {
            if param_index < predicate.parameters.len() {
                if let Some(ir::TypeAnnotation::Relation(_arity)) =
                    &predicate.parameters[param_index].type_annotation
                {
                    // Parameter expects a relation - check if the term is a variable that resolves to a predicate
                    if let ast::Term::Variable(var_name) = term {
                        let simple_path = ast::QualifiedPath::simple(vec![var_name.clone()]);
                        if self
                            .resolve_qualified_path_as_predicate(&simple_path, ir_program)
                            .is_ok()
                        {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        Ok(false)
    }

    /// Compile a term as a predicate reference for higher-order predicates
    fn compile_term_as_predicate_ref(
        &self,
        term: &ast::Term,
        ir_program: &ir::Program,
    ) -> Result<ir::Term, CompileError> {
        match term {
            ast::Term::Variable(var_name) => {
                let simple_path = ast::QualifiedPath::simple(vec![var_name.clone()]);
                let predicate_id =
                    self.resolve_qualified_path_as_predicate(&simple_path, ir_program)?;
                Ok(ir::Term::Predicate(predicate_id))
            }
            _ => {
                // For non-variable terms, just compile normally
                self.compile_term(term, ir_program)
            }
        }
    }
}
