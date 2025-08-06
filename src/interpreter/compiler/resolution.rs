//! Import resolution phase for IR compilation
//!
//! This module handles the second phase of compilation: resolving use clauses
//! and building final symbol maps using an iterative fixpoint algorithm.

use super::ir::*;
use super::*;
use crate::interpreter::symbol_table::InternedSymbol;

/// Import resolution methods for the IR compiler
impl Compiler {
    /// Phase 2: Resolve imports using simplified fixpoint algorithm with global pending list
    pub(super) fn resolve_imports(&mut self, ir_program: &mut Program) -> Result<(), CompileError> {
        loop {
            // Use std::mem::replace to take current pending imports, leaving empty list
            let current_pending = std::mem::replace(&mut self.pending_imports, Vec::new());

            if current_pending.is_empty() {
                break; // No more imports to resolve
            }

            let mut resolved_count = 0;

            // Try to resolve each import
            for pending_import in current_pending {
                match self.try_resolve_import(&pending_import, ir_program)? {
                    Some(resolved_imports) => {
                        for resolved_import in &resolved_imports {
                            ir_program.registry_mut().add_alias(ir::Alias {
                                id: ItemId::with_parent(
                                    resolved_import.importing_module.full_path(),
                                    resolved_import.import_name.clone(),
                                ),
                                target: resolved_import.imported_item.clone(),
                            });
                        }
                        resolved_count += resolved_imports.len();
                    }
                    None => {
                        // Import not yet resolved - add back to pending list
                        self.pending_imports.push(pending_import);
                    }
                }
            }

            // If no imports were resolved in this iteration, we're done (fixpoint reached)
            if resolved_count == 0 {
                break;
            }
        }

        // Check for any remaining unresolved imports
        if !self.pending_imports.is_empty() {
            return Err(CompileError::UnresolvedImports {
                imports: std::mem::replace(&mut self.pending_imports, Vec::new()),
            });
        }

        //println!("Import resolution complete: {}", ir_program);

        Ok(())
    }

    /// Extract the module path from an import statement
    fn extract_module_path_from_import<'a>(
        &self,
        import: &'a PendingImport,
    ) -> Option<&'a ast::QualifiedPath> {
        match &import.use_statement.path {
            ast::UsePath::Simple(qualified_path, _) => {
                // For simple imports, we would need to extract the module part, but for now
                // we can return the full path and let the resolver handle it
                Some(qualified_path)
            }
            ast::UsePath::Glob(qualified_path) => {
                // For glob imports, the entire qualified path is the module
                Some(qualified_path)
            }
            ast::UsePath::List(qualified_path, _) => {
                // For list imports, the qualified path is the module
                Some(qualified_path)
            }
        }
    }

    /// Check if an import references a specific crate
    fn import_references_crate(&self, import: &PendingImport, crate_name: &str) -> bool {
        match &import.use_statement.path {
            ast::UsePath::Simple(qualified_path, _)
            | ast::UsePath::Glob(qualified_path)
            | ast::UsePath::List(qualified_path, _) => {
                self.qualified_path_references_crate(qualified_path, crate_name)
            }
        }
    }

    /// Check if a qualified path references a specific crate
    fn qualified_path_references_crate(
        &self,
        qualified_path: &ast::QualifiedPath,
        crate_name: &str,
    ) -> bool {
        match qualified_path {
            ast::QualifiedPath::Global(segments) | ast::QualifiedPath::Absolute(segments) => {
                segments.first().map(|s| s.as_ref()) == Some(crate_name)
            }
            ast::QualifiedPath::External(external_crate, _) => {
                external_crate.as_ref() == crate_name
            }
            _ => false, // Relative paths don't reference external crates
        }
    }

    /// Try to resolve a single import, returning the resolved ItemId if successful
    fn try_resolve_import(
        &mut self,
        pending_import: &PendingImport,
        ir_program: &mut Program,
    ) -> Result<Option<Vec<ResolvedImport>>, CompileError> {
        self.module_path_stack
            .push(pending_import.importing_module.full_path());

        let result = match &pending_import.use_statement.path {
            ast::UsePath::Simple(qualified_path, name) => {
                self.try_resolve_simple_import(qualified_path, name, ir_program)
            }
            ast::UsePath::Glob(_) => self.try_resolve_glob_import(pending_import, ir_program),
            ast::UsePath::List(_, _) => self.try_resolve_list_import(pending_import, ir_program),
        };

        self.module_path_stack.pop();

        result
    }

    /// Resolve a qualified path from a use statement to resolved path
    fn resolve_use_clause_path(
        &self,
        qualified_path: &ast::QualifiedPath,
        _context_module: &ModuleId,
        _symbol_name: &str,
        ir_program: &Program,
    ) -> Result<ResolvedPath, ResolutionError> {
        // Use the modern resolution function that returns ResolvedPath
        self.resolve_qualified_path_all(qualified_path, ir_program)
            .map_err(ResolutionError::Failed)
    }

    /// Try to resolve a simple import (use path::item)
    fn try_resolve_simple_import(
        &self,
        qualified_path: &ast::QualifiedPath,
        name: &InternedSymbol,
        ir_program: &mut Program,
    ) -> Result<Option<Vec<ResolvedImport>>, CompileError> {
        // Use new path resolution
        let resolved_path =
            match self.resolve_qualified_path_and_item(qualified_path, name, ir_program) {
                Ok(path) => path,
                Err(_e) => return Ok(None),
            };

        let mut resolved_imports = vec![];
        // Add all item kinds that match with the given name. Imports do not
        // specify the namespace of the imported item, so we must import from
        // all namespaces.
        if let Some(type_id) = resolved_path.as_type {
            resolved_imports.push(ResolvedImport {
                importing_module: self.current_module_id(),
                imported_item: type_id.clone().into(),
                import_name: type_id.id.name.clone(),
            });
        }

        if let Some(predicate_id) = resolved_path.as_predicate {
            resolved_imports.push(ResolvedImport {
                importing_module: self.current_module_id(),
                imported_item: predicate_id.clone().into(),
                import_name: predicate_id.id.name.clone(),
            });
        }

        if let Some(module_id) = resolved_path.as_module {
            resolved_imports.push(ResolvedImport {
                importing_module: self.current_module_id(),
                imported_item: module_id.clone().into(),
                import_name: module_id.id.name.clone(),
            });
        }

        if resolved_imports.is_empty() {
            Ok(None) // Symbol not found
        } else {
            Ok(Some(resolved_imports))
        }
    }

    /// Try to resolve a glob import (use path::*) with incremental compilation support
    ///
    /// This validates the glob import at compile time but defers actual symbol resolution
    /// to runtime, allowing incremental changes to be picked up automatically.
    fn try_resolve_glob_import(
        &mut self,
        pending_import: &PendingImport,
        ir_program: &mut Program,
    ) -> Result<Option<Vec<ResolvedImport>>, CompileError> {
        // Extract the qualified path from the glob import
        let qualified_path = match &pending_import.use_statement.path {
            ast::UsePath::Glob(qualified_path) => qualified_path,
            _ => unreachable!("try_resolve_glob_import called on non-glob import"),
        };

        // 1. VALIDATION: Resolve target module and check accessibility
        let target_module = match self.resolve_qualified_path_as_module(qualified_path, ir_program)
        {
            Ok(module_id) => module_id,
            Err(_) => {
                // Module not found yet - import cannot be resolved
                return Ok(None);
            }
        };

        // 2. VALIDATION: Check module accessibility
        if !self.is_module_accessible(&pending_import.importing_module, &target_module, ir_program)
        {
            return Err(CompileError::ModuleNotAccessible {
                target: target_module,
                from: pending_import.importing_module.clone(),
            });
        }

        // 3. VALIDATION: Check for immediate conflicts with existing symbols
        // We validate current conflicts but allow runtime resolution for incremental changes
        if let Err(conflict) = self.validate_glob_conflicts(
            &pending_import.importing_module,
            &target_module,
            ir_program,
        ) {
            return Err(conflict);
        }

        // 4. RECORD RE-EXPORT: Add the re-export relationship for runtime resolution
        ir_program.registry_mut().add_re_export(
            pending_import.importing_module.clone(),
            target_module.clone(),
        );

        // 5. Record as resolved glob import for tracking
        self.resolved_glob_imports.push(super::ResolvedGlobImport {
            importing_module: pending_import.importing_module.clone(),
            target_module,
        });


        // Return empty resolved imports - actual resolution happens at runtime
        // This allows incremental changes to be picked up automatically
        Ok(Some(Vec::new()))
    }

    /// Check if a target module is accessible from the importing module
    fn is_module_accessible(
        &self,
        _importing_module: &ModuleId,
        _target_module: &ModuleId,
        _ir_program: &Program,
    ) -> bool {
        // For now, all public modules are accessible
        // In the future, this could check:
        // - Crate boundaries
        // - pub(crate) visibility rules
        // - pub(super) visibility rules
        true
    }

    /// Validate that a glob import doesn't create immediate conflicts
    fn validate_glob_conflicts(
        &self,
        importing_module: &ModuleId,
        target_module: &ModuleId,
        ir_program: &Program,
    ) -> Result<(), CompileError> {
        // Get symbols that would be imported
        let target_symbols = ir_program.registry().get_all_visible_items(target_module);

        // Check each symbol for conflicts with existing symbols in the importing module
        for (item_name, _item_id) in target_symbols {
            // Check for conflicts with direct items
            if let Some(_existing) =
                self.find_existing_direct_symbol(importing_module, &item_name, ir_program)
            {
                // Direct items take precedence over glob imports - no conflict
                continue;
            }

            // Check for conflicts with explicit imports
            if let Some(_existing) =
                self.find_existing_explicit_import(importing_module, &item_name, ir_program)
            {
                // Explicit imports take precedence over glob imports - no conflict
                continue;
            }

            // Check for conflicts with other glob imports (same precedence level)
            if let Some(existing_glob) =
                self.find_existing_glob_import(importing_module, &item_name, ir_program)
            {
                if existing_glob != *target_module {
                    return Err(CompileError::AmbiguousGlobImport {
                        symbol: item_name,
                        source1: existing_glob,
                        source2: target_module.clone(),
                        importing_module: importing_module.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Find an existing direct symbol in a module
    fn find_existing_direct_symbol(
        &self,
        module_id: &ModuleId,
        item_name: &ItemName,
        ir_program: &Program,
    ) -> Option<ItemId> {
        let direct_items = ir_program.registry().find_direct_items(
            module_id,
            item_name.name.as_ref(),
            item_name.kind,
        );
        direct_items.into_iter().next()
    }

    /// Find an existing explicit import in a module
    fn find_existing_explicit_import(
        &self,
        module_id: &ModuleId,
        item_name: &ItemName,
        ir_program: &Program,
    ) -> Option<ItemId> {
        let explicit_imports = ir_program.registry().find_explicit_imports(
            module_id,
            item_name.name.as_ref(),
            item_name.kind,
        );
        explicit_imports.into_iter().next()
    }

    /// Find an existing glob import that provides a symbol
    fn find_existing_glob_import(
        &self,
        module_id: &ModuleId,
        item_name: &ItemName,
        ir_program: &Program,
    ) -> Option<ModuleId> {
        // Check all re-exported modules for this symbol
        if let Some(re_exported_modules) = ir_program.registry().get_re_exports(module_id) {
            for re_exported in re_exported_modules {
                let symbols = ir_program.registry().get_all_visible_items(re_exported);
                if symbols.contains_key(item_name) {
                    return Some(re_exported.clone());
                }
            }
        }
        None
    }

    /// Try to resolve a list import (use path::{item1, item2})
    fn try_resolve_list_import(
        &self,
        _pending_import: &PendingImport,
        _ir_program: &mut Program,
    ) -> Result<Option<Vec<ResolvedImport>>, CompileError> {
        unimplemented!();
    }
}

#[cfg(test)]
mod resolution_tests;
