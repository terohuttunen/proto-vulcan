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


    /// Try to resolve a list import (use path::{item1, item2})
    fn try_resolve_list_import(
        &self,
        pending_import: &PendingImport,
        ir_program: &mut Program,
    ) -> Result<Option<Vec<ResolvedImport>>, CompileError> {
        let (qualified_path, imports) = match &pending_import.use_statement.path {
            ast::UsePath::List(path, imports) => (path, imports),
            _ => return Err(CompileError::UnresolvedImports { 
                imports: vec![pending_import.clone()]
            }),
        };

        let mut all_resolved_imports = vec![];
        let mut _missing_items = vec![];

        // Resolve each item in the list
        for (item_name, alias) in imports {
            // Try to resolve this individual item
            match self.resolve_qualified_path_and_item(qualified_path, item_name, ir_program) {
                Ok(resolved_path) => {
                    // For each resolved item, create alias with the specified local name
                    let local_name = alias.as_ref().unwrap_or(item_name);
                    
                    // Add all matching item kinds (same pattern as simple imports)
                    if let Some(type_id) = resolved_path.as_type {
                        let import_name = ir::ItemName::new(&**local_name, ir::ItemKind::Type)
                            .map_err(|_| CompileError::UnresolvedImports { 
                                imports: vec![pending_import.clone()]
                            })?;
                        all_resolved_imports.push(ResolvedImport {
                            importing_module: self.current_module_id(),
                            imported_item: type_id.clone().into(),
                            import_name,
                        });
                    }

                    if let Some(predicate_id) = resolved_path.as_predicate {
                        let import_name = ir::ItemName::new(&**local_name, ir::ItemKind::Predicate)
                            .map_err(|_| CompileError::UnresolvedImports { 
                                imports: vec![pending_import.clone()]
                            })?;
                        all_resolved_imports.push(ResolvedImport {
                            importing_module: self.current_module_id(),
                            imported_item: predicate_id.clone().into(),
                            import_name,
                        });
                    }

                    if let Some(module_id) = resolved_path.as_module {
                        let import_name = ir::ItemName::new(&**local_name, ir::ItemKind::Module)
                            .map_err(|_| CompileError::UnresolvedImports { 
                                imports: vec![pending_import.clone()]
                            })?;
                        all_resolved_imports.push(ResolvedImport {
                            importing_module: self.current_module_id(),
                            imported_item: module_id.clone().into(),
                            import_name,
                        });
                    }
                }
                Err(_) => {
                    _missing_items.push(item_name.to_string());
                }
            }
        }

        // If no items were resolved, return None
        if all_resolved_imports.is_empty() {
            return Ok(None);
        }

        // Return what we found (even if some items were missing)
        Ok(Some(all_resolved_imports))
    }
}

#[cfg(test)]
mod resolution_tests;
