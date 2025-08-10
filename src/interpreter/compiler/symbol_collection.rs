//! Symbol collection phase for IR compilation
//!
//! This module handles the first phase of compilation: collecting all type, predicate,
//! and module declarations into symbol tables and processing use statements.

use super::errors::CompileError;
use super::ir;
use super::Compiler;
use crate::interpreter::ast;
use crate::interpreter::symbol_table::InternedSymbol;

/// Compilation phase tracking
#[derive(Debug, Clone, PartialEq)]
pub enum CompilationPhase {
    SymbolAndUseClauseCollection,
    ImportResolution,
    BodyCompilation,
    Validation,
}

/// A pending import during compilation
#[derive(Debug, Clone)]
pub struct PendingImport {
    /// The AST use statement
    pub use_statement: ast::UseStatement,
    /// Module path where this import is located
    pub importing_module: ir::ModuleId,
    /// Optional alias
    pub alias: Option<String>,
}

/// A resolved import during compilation
#[derive(Debug, Clone)]
pub struct ResolvedImport {
    pub importing_module: ir::ModuleId,
    pub imported_item: ir::ItemId,
    pub import_name: ir::ItemName,
}

/// Symbol collection methods for the IR compiler
impl Compiler {
    /// Phase 1: Collect all symbols and use clauses
    pub(super) fn collect_symbols_and_use_clauses(
        &mut self,
        program: &ast::Program,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        for item in &program.items {
            self.collect_item_symbols(item, ir_program)?;
        }
        Ok(())
    }

    /// Collect symbols from individual AST items (for incremental compilation)
    pub(super) fn collect_symbols_from_items(
        &mut self,
        items: &[ast::Item],
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        for item in items {
            self.collect_item_symbols(item, ir_program)?;
        }
        Ok(())
    }

    /// Collect symbols from a single AST item
    fn collect_item_symbols(
        &mut self,
        item: &ast::Item,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        match item {
            ast::Item::Module(module) => {
                self.collect_module_symbols(module, ir_program)?;
            }
            ast::Item::Struct(struct_def) => {
                self.collect_struct_symbols(struct_def, ir_program)?;
            }
            ast::Item::Enum(enum_def) => {
                self.collect_enum_symbols(enum_def, ir_program)?;
            }
            ast::Item::Predicate(predicate) => {
                self.collect_predicate_symbols(predicate, ir_program)?;
            }
            ast::Item::Use(use_statement) => {
                // Process use statements to populate pending imports
                self.collect_use_statement(use_statement)?;
            }
            ast::Item::ExternCrate(extern_crate) => {
                // Process extern crate declarations by loading entire crate tree
                self.collect_extern_crate_symbols(extern_crate, ir_program)?;
            }
            ast::Item::ModuleDeclaration(module_decl) => {
                self.collect_module_declaration_symbols(module_decl, ir_program)?;
            }
            ast::Item::Impl(impl_block) => {
                self.collect_impl_symbols(impl_block, ir_program)?;
            }
        }
        Ok(())
    }

    /// Collect module symbols
    fn collect_module_symbols(
        &mut self,
        module: &ast::ModuleDefinition,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Create module ID directly from current context - no parsing needed
        let module_id = ir::ModuleId::with_parent(self.current_module_path(), module.name.to_string());

        // Create module item
        let ir_module = ir::Module {
            id: module_id.clone(),
            //items: im_rc::HashMap::new(), // Will be populated during compilation
            //imports: im_rc::HashMap::new(),
            visibility: self.convert_visibility(&module.visibility)?,
        };

        // Add to registry
        let registry = ir_program.registry_mut();
        if !registry.add_module(ir_module) {
            return Err(CompileError::DuplicateItem {
                item: module_id.id.clone(),
                symbol: module.name.clone(),
            });
        }

        // Add to module map
        self.module_map.add_module(module_id.clone());
        self.module_map.add_item(
            self.current_module_id(),
            module_id.id.name.clone(),
            module_id.id.clone(),
        );

        // Push module onto path stack and process children
        self.module_path_stack.push(module_id.full_path());

        for child_item in &module.items {
            self.collect_item_symbols(child_item, ir_program)?;
        }

        // Restore context
        self.module_path_stack.pop();

        Ok(())
    }

    /// Collect struct symbols
    fn collect_struct_symbols(
        &mut self,
        struct_def: &ast::StructDefinition,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Create type ID directly from current context - no parsing needed
        let type_id = ir::TypeId::with_parent(self.current_module_path(), struct_def.name.to_string());

        // Create type definition (fields will be resolved in phase 2)
        let ir_type = ir::TypeDefinition {
            id: type_id.clone(),
            kind: ir::TypeKind::Struct(ir::StructDefinition {
                name: struct_def.name.clone(),
                fields: ir::StructFields::Tuple(vec![]), // Placeholder
            }),
            visibility: self.convert_visibility(&struct_def.visibility)?,
        };

        // Add to registry
        let registry = ir_program.registry_mut();
        if !registry.add_type(ir_type) {
            return Err(CompileError::DuplicateItem {
                item: type_id.id.clone(),
                symbol: struct_def.name.clone(),
            });
        }

        // Add to module map
        self.module_map.add_item(
            self.current_module_id(),
            type_id.id.name.clone(),
            type_id.id.clone(),
        );

        Ok(())
    }

    /// Collect enum symbols
    fn collect_enum_symbols(
        &mut self,
        enum_def: &ast::EnumDefinition,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Create type ID directly from current context - no parsing needed
        let type_id = ir::TypeId::with_parent(self.current_module_path(), enum_def.name.to_string());

        // Create type definition (variants will be resolved in phase 2)
        let ir_type = ir::TypeDefinition {
            id: type_id.clone(),
            kind: ir::TypeKind::Enum(ir::EnumDefinition {
                name: enum_def.name.clone(),
                variants: vec![], // Placeholder
            }),
            visibility: self.convert_visibility(&enum_def.visibility)?,
        };

        // Add to registry
        let registry = ir_program.registry_mut();
        if !registry.add_type(ir_type.clone()) {
            return Err(CompileError::DuplicateItem {
                item: type_id.id.clone(),
                symbol: enum_def.name.clone(),
            });
        }

        // Add to module map
        self.module_map.add_item(
            self.current_module_id(),
            type_id.id.name.clone(),
            type_id.id.clone(),
        );

        Ok(())
    }

    /// Collect predicate symbols
    fn collect_predicate_symbols(
        &mut self,
        predicate: &ast::PredicateDefinition,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Skip @test items when not in test mode
        if !self.compilation_context.options.include_test_items {
            let has_test_attr = predicate.attributes.iter().any(|attr| attr.name.to_string() == "test");
            if has_test_attr {
                return Ok(()); // Skip this predicate
            }
        }
        // Create predicate ID directly from current context - no parsing needed
        let predicate_id =
            ir::PredicateId::with_parent(self.current_module_path(), predicate.name.to_string());

        // Create predicate definition (body will be compiled in phase 2)
        let ir_predicate = ir::Predicate {
            id: predicate_id.clone(),
            parameters: vec![],                          // Placeholder
            body: ir::StructuralGoal::empty_container(), // Placeholder
            kind: match predicate.predicate_kind {
                ast::PredicateKind::Relation => ir::PredicateKind::Relation,
                ast::PredicateKind::Macro => ir::PredicateKind::Macro,
                ast::PredicateKind::Grammar => ir::PredicateKind::Grammar,
            },
            visibility: self.convert_visibility(&predicate.visibility)?,
        };

        // Add to registry
        let registry = ir_program.registry_mut();
        if !registry.add_predicate(ir_predicate) {
            return Err(CompileError::DuplicateItem {
                item: predicate_id.id.clone(),
                symbol: predicate.name.clone(),
            });
        }

        // Add to module map
        self.module_map.add_item(
            self.current_module_id(),
            predicate_id.id.name.clone(),
            predicate_id.id.clone(),
        );

        Ok(())
    }

    /// Collect impl block symbols
    fn collect_impl_symbols(
        &mut self,
        impl_block: &ast::ImplBlock,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Impl blocks are treated as modules containing predicates
        // Create module ID directly from current context - no parsing needed
        let module_id =
            ir::ModuleId::with_parent(self.current_module_path(), impl_block.type_name.to_string());
        
        // Create module item
        let ir_module = ir::Module {
            id: module_id.clone(),
            //items: im_rc::HashMap::new(), // Will be populated during compilation
            //imports: im_rc::HashMap::new(),
            visibility: ir::Visibility::Public, // Impl blocks are typically public
        };

        // Add to registry
        let registry = ir_program.registry_mut();
        if !registry.add_module(ir_module) {
            return Err(CompileError::DuplicateItem {
                item: module_id.id.clone(),
                symbol: impl_block.type_name.clone(),
            });
        }

        // Add to module map
        self.module_map.add_module(module_id.clone());
        self.module_map.add_item(
            self.current_module_id(),
            ir::ItemName::new_unchecked(impl_block.type_name.to_string(), ir::ItemKind::Module),
            module_id.id.clone(),
        );

        // Push impl module context and collect predicates
        self.module_path_stack.push(module_id.full_path());

        for predicate in &impl_block.predicates {
            self.collect_predicate_symbols(predicate, ir_program)?;
        }

        // Restore context
        self.module_path_stack.pop();
        // Module context managed by stack - old_current;

        Ok(())
    }

    /// Collect module declaration symbols and load the corresponding module file
    fn collect_module_declaration_symbols(
        &mut self,
        module_decl: &ast::ModuleDeclaration,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Create module ID directly from current context - no parsing needed
        let module_id = ir::ModuleId::with_parent(self.current_module_path(), module_decl.name.to_string());
        let ir_module = ir::Module {
            id: module_id.clone(),
            //items: im_rc::HashMap::new(), // Will be populated when we load the module file
            //imports: im_rc::HashMap::new(),
            visibility: self.convert_visibility(&module_decl.visibility)?,
        };

        // Add to registry
        let registry = ir_program.registry_mut();
        if !registry.add_module(ir_module) {
            return Err(CompileError::DuplicateItem {
                item: module_id.id.clone(),
                symbol: module_decl.name.clone(),
            });
        }

        // Add to module map
        self.module_map.add_module(module_id.clone());
        self.module_map.add_item(
            self.current_module_id(),
            module_id.id.name.clone(),
            module_id.id.clone(),
        );

        // Now load the module file and compile it in the context of this module
        // Use generic crate-based resolution for external crates, relative paths for local modules
        let path = self.resolve_module_file_path(&module_decl.name.to_string())?;

        if path.exists() {
            // Read and parse the module file
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

            // Push module context before processing items
            self.module_path_stack.push(module_id.full_path());

            // Recursively collect symbols from the module file
            for item in &module_ast.items {
                self.collect_item_symbols(item, ir_program)?;
            }

            // Restore context
            self.module_path_stack.pop();
        }
        // If file doesn't exist, that's okay - the module declaration is still valid
        // but the module is empty

        Ok(())
    }

    /// Collect extern crate symbols by loading the entire crate tree
    pub(super) fn collect_extern_crate_symbols(
        &mut self,
        extern_crate: &ast::ExternCrateStatement,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        let crate_name = extern_crate.crate_name.to_string();
        let crate_alias = extern_crate
            .alias
            .as_ref()
            .map(|alias| alias.to_string())
            .unwrap_or_else(|| crate_name.clone());

        // Find the crate root directory
        let crate_root =
            self.find_crate_root(&crate_name)
                .ok_or_else(|| CompileError::SemanticError {
                    message: format!("External crate '{}' not found in search paths", crate_name),
                    symbol: extern_crate.crate_name.clone(),
                })?;

        // Create crate root module as a global module (no parent)
        let crate_module_id = ir::ModuleId::root_with_name(crate_alias);

        // Create the crate root module if it doesn't exist
        let registry = ir_program.registry_mut();
        if !registry.contains_item(&crate_module_id) {
            let crate_module = ir::Module {
                id: crate_module_id.clone(),
                //items: im_rc::HashMap::new(),
                //imports: im_rc::HashMap::new(),
                visibility: ir::Visibility::Public,
            };
            registry.add_module(crate_module);

            // Add to module map
            self.module_map.add_module(crate_module_id.clone());
            self.module_map.add_item(
                self.current_module_id(),
                crate_module_id.id.name.clone(),
                crate_module_id.id.clone(),
            );
        }

        // Load the entire crate tree starting from mod.pv or crate_name.pv
        let entry_file = crate_root
            .join("mod.pv")
            .exists()
            .then(|| crate_root.join("mod.pv"))
            .or_else(|| {
                let crate_file = crate_root.join(format!("{}.pv", crate_name));
                crate_file.exists().then_some(crate_file)
            })
            .ok_or_else(|| CompileError::SemanticError {
                message: format!(
                    "Crate '{}' has no entry point (mod.pv or {}.pv)",
                    crate_name, crate_name
                ),
                symbol: extern_crate.crate_name.clone(),
            })?;

        // Load the crate tree recursively
        self.load_crate_tree(&entry_file, &crate_module_id, &crate_root, ir_program)?;

        Ok(())
    }

    /// Recursively load a crate tree by following mod declarations
    fn load_crate_tree(
        &mut self,
        module_file: &std::path::Path,
        parent_module_id: &ir::ModuleId,  // Actually the module ID for this file, not its parent
        crate_root: &std::path::Path,
        ir_program: &mut ir::Program,
    ) -> Result<(), CompileError> {
        // Read and parse the module file
        let module_contents =
            std::fs::read_to_string(module_file).map_err(|e| CompileError::SemanticError {
                message: format!("Failed to read module {}: {}", module_file.display(), e),
                symbol: InternedSymbol::from_text(&module_file.to_string_lossy()),
            })?;

        let module_ast = crate::interpreter::parser::parse_str(&module_contents).map_err(|e| {
            CompileError::SemanticError {
                message: format!("Failed to parse module {}: {}", module_file.display(), e),
                symbol: InternedSymbol::from_text(&module_file.to_string_lossy()),
            }
        })?;

        // Push the current module context
        self.module_path_stack.push(parent_module_id.full_path());

        // Two-pass processing: create all modules first, then load their contents
        // This ensures sibling modules exist before processing imports

        // First pass: Create all child modules (empty shells)
        let mut child_modules = Vec::new();
        for item in &module_ast.items {
            if let ast::Item::ModuleDeclaration(mod_decl) = item {
                let child_module_id =
                    ir::ModuleId::with_parent(parent_module_id.full_path(), mod_decl.name.to_string());

                // Create child module
                let registry = ir_program.registry_mut();
                if !registry.contains_item(&child_module_id) {
                    let child_module = ir::Module {
                        id: child_module_id.clone(),
                        //items: im_rc::HashMap::new(),
                        //imports: im_rc::HashMap::new(),
                        visibility: self.convert_visibility(&mod_decl.visibility)?,
                    };
                    registry.add_module(child_module);

                    // Add to module map
                    self.module_map.add_module(child_module_id.clone());
                    self.module_map.add_item(
                        self.current_module_id(),
                        child_module_id.id.name.clone(),
                        child_module_id.id.clone(),
                    );
                }

                // Store for second pass
                let child_file = crate_root.join(format!("{}.pv", mod_decl.name.to_string()));
                if child_file.exists() {
                    child_modules.push((child_file, child_module_id));
                }
            }
        }

        // Second pass: Recursively load child module contents
        for (child_file, child_module_id) in child_modules {
            self.load_crate_tree(&child_file, &child_module_id, crate_root, ir_program)?;
        }

        // Third pass: Process all other items (types, predicates, use statements, etc.)
        for item in &module_ast.items {
            match item {
                ast::Item::ModuleDeclaration(_) => {
                    // Already processed in first pass
                }
                _ => {
                    // Process other items normally
                    self.collect_item_symbols(item, ir_program)?;
                }
            }
        }

        // Store module items for later body compilation after import resolution
        self.external_module_items.push((parent_module_id.full_path(), module_ast.items));

        // Restore context
        self.module_path_stack.pop();

        Ok(())
    }

    /// Collect a use statement and add it to pending imports
    fn collect_use_statement(
        &mut self,
        use_statement: &ast::UseStatement,
    ) -> Result<(), CompileError> {
        let current_module = &self.current_module_id();

        match &use_statement.path {
            ast::UsePath::Simple(_qualified_path, _symbol) => {
                let _symbol_name = _symbol.to_string();
                let pending_import = PendingImport {
                    use_statement: use_statement.clone(),
                    importing_module: current_module.clone(),
                    alias: None,
                };

                self.pending_imports.push(pending_import);
            }
            ast::UsePath::Glob(_qualified_path) => {
                let pending_import = PendingImport {
                    use_statement: use_statement.clone(),
                    importing_module: current_module.clone(),
                    alias: None,
                };

                self.pending_imports.push(pending_import);
            }
            ast::UsePath::List(_qualified_path, items) => {
                for (symbol, alias) in items {
                    let _symbol_name = symbol.to_string();
                    let alias_name = alias.as_ref().map(|a| a.to_string());
                    let pending_import = PendingImport {
                        use_statement: use_statement.clone(),
                        importing_module: current_module.clone(),
                        alias: alias_name,
                    };

                    self.pending_imports.push(pending_import);
                }
            }
        }

        Ok(())
    }
}
