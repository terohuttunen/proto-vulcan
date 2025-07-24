//! Symbol collection phase for IR compilation
//!
//! This module handles the first phase of compilation: collecting all type, predicate,
//! and module declarations into symbol tables and processing use statements.

use super::errors::CompileWarning;
use super::*;

/// Symbol resolution context during compilation
#[derive(Debug, Clone)]
pub struct SymbolContext {
    /// Current module being compiled
    pub current_module: ModuleId,
}

/// Compilation phase tracking
#[derive(Debug, Clone, PartialEq)]
pub enum CompilationPhase {
    SymbolAndUseClauseCollection,
    ImportResolution,
    BodyCompilation,
    Validation,
}

/// Fast symbol lookup map for a single module during compilation
#[derive(Debug, Clone)]
pub struct ModuleSymbolMap {
    /// Local symbols defined in this module (types, predicates, submodules)
    pub local_symbols: HashMap<String, ItemId>,
    /// Imports symbols in this module
    pub imported_symbols: HashMap<String, ItemId>,
}

/// A pending import during compilation
#[derive(Debug, Clone)]
pub struct PendingImport {
    /// The AST use statement
    pub use_statement: ast::UseStatement,
    /// Module path where this import is located
    pub importing_module: ModuleId,
    /// Target module path being imported from
    pub target_module: String,
    /// Symbol name being imported
    pub symbol_name: String,
    /// Optional alias
    pub alias: Option<String>,
}

impl ModuleSymbolMap {
    pub fn new() -> Self {
        Self {
            local_symbols: HashMap::new(),
            imported_symbols: HashMap::new(),
        }
    }

    /// Check if this module has a symbol (either local or imported)
    pub fn has_symbol(&self, name: &str) -> Option<&ItemId> {
        self.local_symbols
            .get(name)
            .or_else(|| self.imported_symbols.get(name))
    }

    /// Add a local symbol to this module
    pub fn add_local_symbol(&mut self, name: String, item: impl Into<ItemId>) {
        self.local_symbols.insert(name, item.into());
    }

    /// Get a local module (not imported) - enforces no-shadowing rule
    pub fn get_local_module(&self, name: &str) -> Option<&ItemId> {
        // Only return if it's a local module (local symbol)
        self.local_symbols
            .get(name)
            .filter(|item| item.kind == ItemKind::Module)
    }

    /// Get a imported symbol item
    pub fn get_imported_symbol(&self, name: &str) -> Option<&ItemId> {
        self.imported_symbols.get(name)
    }

    /// Get all local symbols in this module (for glob imports)
    pub fn get_all_local_symbols(&self) -> impl Iterator<Item = (&String, &ItemId)> {
        self.local_symbols.iter()
    }

    /// Get all imported symbols in this module (for glob imports)
    pub fn get_all_imported_symbols(&self) -> impl Iterator<Item = (&String, &ItemId)> {
        self.imported_symbols.iter()
    }

    /// Get all symbols (both local and imported) in this module (for glob imports)
    pub fn get_all_symbols(&self) -> impl Iterator<Item = (&String, &ItemId)> {
        self.local_symbols
            .iter()
            .chain(self.imported_symbols.iter())
    }

    /// Try to add a symbol from a glob import - silently skips if symbol already exists
    /// Glob imports are shadowed by both local symbols and explicit imports
    pub fn try_add_glob_imported_symbol(
        &mut self,
        local_name: String,
        source_item: impl Into<ItemId>,
        import_source: &str,
    ) -> Result<bool, CompileError> {
        let source_item_id = source_item.into();

        // Check if this symbol already exists (either local or imported)
        if self.has_symbol(&local_name).is_some() {
            // Symbol already exists - glob import is shadowed, skip silently
            return Ok(false);
        }

        // No existing symbol, add the glob import
        self.imported_symbols.insert(local_name, source_item_id);
        Ok(true)
    }

    /// Try to add an imported symbol with module shadowing prevention and other shadowing warnings
    pub fn try_add_imported_symbol(
        &mut self,
        local_name: String,
        source_item: impl Into<ItemId>,
        import_source: &str,
        import_symbol: &InternedSymbol,
    ) -> Result<Option<CompileWarning>, CompileError> {
        let source_item_id = source_item.into();
        let mut warning = None;

        // Check if this import would shadow a local symbol
        if let Some(local_item_id) = self.local_symbols.get(&local_name) {
            // HARD ERROR: Never allow shadowing of local modules (breaks path resolution)
            if local_item_id.kind == super::ItemKind::Module {
                let local_symbol = InternedSymbol::from_text(&local_item_id.path);
                return Err(CompileError::ShadowingError {
                    symbol_name: local_name,
                    import_source: import_source.to_string(),
                    local_symbol,
                    import_symbol: import_symbol.clone(),
                });
            }

            // WARNING: Allow shadowing of non-module symbols but warn
            let local_symbol = InternedSymbol::from_text(&local_item_id.path);
            warning = Some(CompileWarning::ShadowingWarning {
                symbol_name: local_name.clone(),
                import_source: import_source.to_string(),
                local_symbol,
                import_symbol: import_symbol.clone(),
            });
        }

        // Check if this import conflicts with an existing import
        if let Some(existing_import) = self.imported_symbols.get(&local_name) {
            if existing_import.path != source_item_id.path {
                // WARNING: Allow ambiguous imports but warn
                let ambiguous_warning = CompileWarning::AmbiguousImportWarning {
                    symbol_name: local_name.clone(),
                    sources: vec![
                        existing_import.path.to_string(),
                        source_item_id.path.to_string(),
                    ],
                    symbol: import_symbol.clone(),
                };

                // If we already have a shadowing warning, prefer the ambiguous import warning
                warning = Some(ambiguous_warning);
            } else {
                // Same import source - this is OK (idempotent)
                return Ok(None);
            }
        }

        // Add the import (even if it causes warnings)
        self.imported_symbols.insert(local_name, source_item_id);
        Ok(warning)
    }
}

/// Symbol collection methods for the IR compiler
impl Compiler {
    /// Phase 1: Collect all symbols and use clauses
    pub(super) fn collect_symbols_and_use_clauses(
        &mut self,
        program: &ast::Program,
        ir_program: &mut Program,
    ) -> Result<(), CompileError> {
        for item in &program.items {
            self.collect_item_symbols(item, ir_program)?;
        }
        Ok(())
    }

    /// Collect symbols from a single AST item
    fn collect_item_symbols(
        &mut self,
        item: &ast::Item,
        ir_program: &mut Program,
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
            ast::Item::ModuleDeclaration(_) => {
                // These are processed during import resolution
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
        ir_program: &mut Program,
    ) -> Result<(), CompileError> {
        let module_path = self.resolve_item_path(&module.name.to_string());

        // Register module in symbol context
        let module_id = ModuleId::new(module_path);

        // Determine parent module (None only for root "::" itself, otherwise current_module)
        let parent = if module_id.id.path.as_ref() == "::" {
            None // Only root module has no parent
        } else {
            Some(self.symbol_context.current_module.clone())
        };

        // Create module item
        let ir_module = Module {
            id: module_id.clone(),
            parent,
            items: vec![], // Will be populated in phase 2
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

        // Initialize symbol map for this module
        self.module_symbol_maps
            .insert(module_id.clone(), ModuleSymbolMap::new());

        // Add this module as a symbol in its parent module's symbol map
        if let Some(parent_map) = self
            .module_symbol_maps
            .get_mut(&self.symbol_context.current_module)
        {
            parent_map.add_local_symbol(module.name.to_string(), &module_id);
        }

        // Push module onto path stack and process children
        self.module_path_stack.push(module.name.to_string());
        let old_current = self.symbol_context.current_module.clone();
        self.symbol_context.current_module = module_id.clone();

        for child_item in &module.items {
            self.collect_item_symbols(child_item, ir_program)?;
        }

        // Restore context
        self.module_path_stack.pop();
        self.symbol_context.current_module = old_current;

        Ok(())
    }

    /// Collect struct symbols
    fn collect_struct_symbols(
        &mut self,
        struct_def: &ast::StructDefinition,
        ir_program: &mut Program,
    ) -> Result<(), CompileError> {
        let type_path = self.resolve_item_path(&struct_def.name.to_string());

        // Register type in symbol context
        let type_id = TypeId::new(type_path);

        // Create type definition (fields will be resolved in phase 2)
        let ir_type = TypeDefinition {
            id: type_id.clone(),
            kind: TypeKind::Struct(StructDefinition {
                fields: StructFields::Tuple(vec![]), // Placeholder
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

        // Add to current module's symbol map
        if let Some(module_map) = self
            .module_symbol_maps
            .get_mut(&self.symbol_context.current_module)
        {
            module_map.add_local_symbol(struct_def.name.to_string(), &type_id);
        }

        Ok(())
    }

    /// Collect enum symbols
    fn collect_enum_symbols(
        &mut self,
        enum_def: &ast::EnumDefinition,
        ir_program: &mut Program,
    ) -> Result<(), CompileError> {
        let type_path = self.resolve_item_path(&enum_def.name.to_string());

        // Register type in symbol context
        let type_id = TypeId::new(type_path);

        // Create type definition (variants will be resolved in phase 2)
        let ir_type = TypeDefinition {
            id: type_id.clone(),
            kind: TypeKind::Enum(EnumDefinition {
                variants: vec![], // Placeholder
            }),
            visibility: self.convert_visibility(&enum_def.visibility)?,
        };

        // Add to registry
        let registry = ir_program.registry_mut();
        if !registry.add_type(ir_type) {
            return Err(CompileError::DuplicateItem {
                item: type_id.id.clone(),
                symbol: enum_def.name.clone(),
            });
        }

        // Add to current module's symbol map
        if let Some(module_map) = self
            .module_symbol_maps
            .get_mut(&self.symbol_context.current_module)
        {
            module_map.add_local_symbol(enum_def.name.to_string(), &type_id);
        }

        Ok(())
    }

    /// Collect predicate symbols
    fn collect_predicate_symbols(
        &mut self,
        predicate: &ast::PredicateDefinition,
        ir_program: &mut Program,
    ) -> Result<(), CompileError> {
        let predicate_path = self.resolve_item_path(&predicate.name.to_string());

        // Register predicate in symbol context
        let predicate_id = PredicateId::new(predicate_path);

        // Create predicate definition (body will be compiled in phase 2)
        let ir_predicate = Predicate {
            id: predicate_id.clone(),
            parameters: vec![],                      // Placeholder
            body: StructuralGoal::empty_container(), // Placeholder
            kind: match predicate.predicate_kind {
                ast::PredicateKind::Relation => PredicateKind::Relation,
                ast::PredicateKind::Macro => PredicateKind::Macro,
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

        // Add to current module's symbol map
        if let Some(module_map) = self
            .module_symbol_maps
            .get_mut(&self.symbol_context.current_module)
        {
            module_map.add_local_symbol(predicate.name.to_string(), &predicate_id);
        }

        Ok(())
    }

    /// Collect impl block symbols
    fn collect_impl_symbols(
        &mut self,
        impl_block: &ast::ImplBlock,
        ir_program: &mut Program,
    ) -> Result<(), CompileError> {
        // Impl blocks are treated as modules containing predicates
        // Modules are in their own namespace, we can use the type name directly
        let impl_module_name = impl_block.type_name.to_string();
        let module_path = self.resolve_item_path(&impl_module_name);

        // Register module in symbol context
        let module_id = ModuleId::new(module_path.clone());

        // Determine parent module (None only for root "::" itself, otherwise current_module)
        let parent = if module_id.id.path.as_ref() == "::" {
            None // Only root module has no parent
        } else {
            Some(self.symbol_context.current_module.clone())
        };

        // Create module item
        let ir_module = Module {
            id: module_id.clone(),
            parent,
            items: vec![],                  // Will be populated in phase 2
            visibility: Visibility::Public, // Impl blocks are typically public
        };

        // Add to registry
        let registry = ir_program.registry_mut();
        if !registry.add_module(ir_module) {
            return Err(CompileError::DuplicateItem {
                item: module_id.id.clone(),
                symbol: impl_block.type_name.clone(),
            });
        }

        // Push impl module context and collect predicates
        self.module_path_stack.push(impl_module_name);
        let old_current = self.symbol_context.current_module.clone();
        self.symbol_context.current_module = module_id.clone();

        for predicate in &impl_block.predicates {
            self.collect_predicate_symbols(predicate, ir_program)?;
        }

        // Restore context
        self.module_path_stack.pop();
        self.symbol_context.current_module = old_current;

        Ok(())
    }

    /// Collect a use statement and add it to pending imports
    fn collect_use_statement(
        &mut self,
        use_statement: &ast::UseStatement,
    ) -> Result<(), CompileError> {
        let current_module = &self.symbol_context.current_module;

        match &use_statement.path {
            ast::UsePath::Simple(qualified_path, symbol) => {
                // For simple imports, we need to determine if the qualified_path represents:
                // 1. The full path to the item (need to extract parent module)
                // 2. The module path (use as-is)
                // Check by seeing if the path has multiple segments - if so, extract module
                let target_module = match qualified_path {
                    ast::QualifiedPath::Global(segments)
                    | ast::QualifiedPath::Absolute(segments) => {
                        if segments.len() > 1 {
                            // Multiple segments: use all but last as module path
                            format!(
                                "::{}",
                                segments[..segments.len() - 1]
                                    .iter()
                                    .map(|s| s.to_string())
                                    .collect::<Vec<_>>()
                                    .join("::")
                            )
                        } else {
                            // Single segment: this is the module path
                            self.qualified_path_to_string(qualified_path)
                        }
                    }
                    _ => self.qualified_path_to_string(qualified_path),
                };
                let symbol_name = symbol.to_string();
                let pending_import = PendingImport {
                    use_statement: use_statement.clone(),
                    importing_module: current_module.clone(),
                    target_module,
                    symbol_name,
                    alias: None,
                };

                self.pending_imports.push(pending_import);
            }
            ast::UsePath::Glob(qualified_path) => {
                let target_module = self.qualified_path_to_string(qualified_path);
                let pending_import = PendingImport {
                    use_statement: use_statement.clone(),
                    importing_module: current_module.clone(),
                    target_module,
                    symbol_name: "*".to_string(), // Special marker for glob imports
                    alias: None,
                };

                self.pending_imports.push(pending_import);
            }
            ast::UsePath::List(qualified_path, items) => {
                let target_module = self.qualified_path_to_string(qualified_path);

                for (symbol, alias) in items {
                    let symbol_name = symbol.to_string();
                    let alias_name = alias.as_ref().map(|a| a.to_string());
                    let pending_import = PendingImport {
                        use_statement: use_statement.clone(),
                        importing_module: current_module.clone(),
                        target_module: target_module.clone(),
                        symbol_name,
                        alias: alias_name,
                    };

                    self.pending_imports.push(pending_import);
                }
            }
        }

        Ok(())
    }
}
