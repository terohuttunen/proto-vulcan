//! Import resolution phase for IR compilation
//!
//! This module handles the second phase of compilation: resolving use clauses
//! and building final symbol maps using an iterative fixpoint algorithm.

use super::*;
use crate::interpreter::symbol_table::InternedSymbol;

/// Import resolution methods for the IR compiler
impl<U: User, E: Engine<U>> Compiler<U, E> {
    /// Phase 2: Resolve imports using simplified fixpoint algorithm with global pending list
    pub(super) fn resolve_imports(&mut self, ir_program: &mut Program) -> Result<(), CompileError> {
        loop {
            // Use std::mem::replace to take current pending imports, leaving empty list
            let current_pending = std::mem::replace(&mut self.pending_imports, Vec::new());

            if current_pending.is_empty() {
                break; // No more imports to resolve
            }

            let mut resolved_count = 0;

            // Try to resolve each import directly
            for pending_import in current_pending {
                match self.try_resolve_import(&pending_import, ir_program)? {
                    Some(resolved_item_id) => {
                        // Check if this is a glob import (identified by the special marker format)
                        let is_glob_import = matches!(&pending_import.use_statement.path, ast::UsePath::Glob(_));
                        
                        if is_glob_import {
                            // Glob imports handle symbol addition internally, no need to add the marker
                            // The resolved_item_id is just a tracking marker for successful glob processing
                        } else {
                            // Regular import - add to symbol maps
                            let local_name = pending_import
                                .alias
                                .clone()
                                .unwrap_or_else(|| pending_import.symbol_name.clone());

                            if let Some(module_map) = self
                                .module_symbol_maps
                                .get_mut(&pending_import.importing_module)
                            {
                                let import_symbol = InternedSymbol::from_text(&pending_import.symbol_name);
                                match module_map.try_add_imported_symbol(
                                    local_name,
                                    resolved_item_id,
                                    &pending_import.target_module,
                                    &import_symbol,
                                )? {
                                    Some(warning) => {
                                        // Add warning to compilation context
                                        self.add_warning(warning);
                                    }
                                    None => {
                                        // Import added successfully without warnings
                                    }
                                }
                            }
                        }
                        resolved_count += 1;
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
            let first_unresolved = &self.pending_imports[0];
            let attempted_path = format!(
                "{}::{}",
                first_unresolved.target_module, first_unresolved.symbol_name
            );
            return Err(CompileError::UnresolvedReference {
                attempted_item: ItemId::new(attempted_path, ItemKind::Module), // Default to Module since type is unknown
                symbol: InternedSymbol::from_text(&first_unresolved.symbol_name),
            });
        }

        Ok(())
    }

    /// Try to resolve a single import, returning the resolved ItemId if successful
    fn try_resolve_import(
        &mut self,
        pending_import: &PendingImport,
        ir_program: &Program,
    ) -> Result<Option<ItemId>, CompileError> {
        match &pending_import.use_statement.path {
            ast::UsePath::Simple(_, _) => {
                self.try_resolve_simple_import(pending_import, ir_program)
            }
            ast::UsePath::Glob(_) => self.try_resolve_glob_import(pending_import, ir_program),
            ast::UsePath::List(_, _) => self.try_resolve_list_import(pending_import, ir_program),
        }
    }

    /// Resolve a qualified path from a use statement to canonical module tree path
    fn resolve_use_clause_path(
        &self,
        qualified_path: &ast::QualifiedPath,
        context_module: &ModuleId,
        symbol_name: &str,
        ir_program: &Program,
    ) -> Result<CanonicalPath, ResolutionError> {
        match qualified_path {
            ast::QualifiedPath::Global(segments) | ast::QualifiedPath::Absolute(segments) => {
                self.resolve_absolute_path(segments, symbol_name)
            }
            ast::QualifiedPath::Relative(segments) => {
                self.resolve_relative_path(segments, context_module, symbol_name)
            }
            ast::QualifiedPath::Self_(segments) => {
                self.resolve_self_path(segments, context_module, symbol_name)
            }
            ast::QualifiedPath::Super(levels, segments) => {
                self.resolve_super_path(*levels, segments, context_module, symbol_name, ir_program)
            }
            ast::QualifiedPath::External(crate_name, segments) => {
                self.resolve_external_path(crate_name, segments, symbol_name)
            }
        }
    }

    /// Resolve an absolute path starting from root (::foo::bar::Item)
    fn resolve_absolute_path(
        &self,
        segments: &[InternedSymbol],
        symbol_name: &str,
    ) -> Result<CanonicalPath, ResolutionError> {
        let root_module = ModuleId::new("::");
        self.resolve_segments_from_module(segments, &root_module, symbol_name)
    }

    /// Resolve a relative path from the given context module  
    fn resolve_relative_path(
        &self,
        segments: &[InternedSymbol],
        context_module: &ModuleId,
        symbol_name: &str,
    ) -> Result<CanonicalPath, ResolutionError> {
        self.resolve_segments_from_module(segments, context_module, symbol_name)
    }

    /// Resolve a self path (self::foo::Item)
    fn resolve_self_path(
        &self,
        segments: &[InternedSymbol],
        context_module: &ModuleId,
        symbol_name: &str,
    ) -> Result<CanonicalPath, ResolutionError> {
        // self:: paths start from the current module
        self.resolve_segments_from_module(segments, context_module, symbol_name)
    }

    /// Resolve a super path (super::foo::Item)
    fn resolve_super_path(
        &self,
        levels: usize,
        segments: &[InternedSymbol],
        context_module: &ModuleId,
        symbol_name: &str,
        ir_program: &Program,
    ) -> Result<CanonicalPath, ResolutionError> {
        let mut current_module = context_module.clone();

        // Go up 'levels' number of parent modules using direct parent references
        for _ in 0..levels {
            if let Some(module) = ir_program.registry.get_module(&current_module) {
                if let Some(parent_id) = &module.parent {
                    current_module = parent_id.clone(); // Cheap clone as noted by user
                } else {
                    // At root, can't go up further
                    return Err(ResolutionError::Failed(CompileError::UnresolvedModule {
                        attempted_item: ModuleId::new("super"),
                        symbol: InternedSymbol::from_text("super"),
                    }));
                }
            } else {
                // Module doesn't exist in registry
                return Err(ResolutionError::Failed(CompileError::UnresolvedModule {
                    attempted_item: current_module.clone(),
                    symbol: InternedSymbol::from_text(current_module.as_ref().path.as_ref()),
                }));
            }
        }

        self.resolve_segments_from_module(segments, &current_module, symbol_name)
    }

    /// Resolve an external path (external_crate::foo::Item)
    fn resolve_external_path(
        &self,
        crate_name: &InternedSymbol,
        segments: &[InternedSymbol],
        symbol_name: &str,
    ) -> Result<CanonicalPath, ResolutionError> {
        // External crates are resolved from root with crate name as first segment
        let mut full_segments = vec![crate_name.clone()];
        full_segments.extend_from_slice(segments);

        let root_module = ModuleId::new("::");
        self.resolve_segments_from_module(&full_segments, &root_module, symbol_name)
    }

    /// Resolve path segments starting from a specific module
    fn resolve_segments_from_module(
        &self,
        segments: &[InternedSymbol],
        start_module: &ModuleId,
        symbol_name: &str,
    ) -> Result<CanonicalPath, ResolutionError> {
        let mut current_module = start_module.clone();

        // Walk through all segments to find the target module
        for segment in segments {
            match self.resolve_segment_in_module(&segment.to_string(), &current_module)? {
                Some(resolved_module) => {
                    current_module = resolved_module;
                }
                None => {
                    // Segment couldn't be resolved yet - blocked
                    return Err(ResolutionError::Blocked);
                }
            }
        }

        // All segments resolved - return canonical path
        Ok(CanonicalPath {
            module_path: current_module.id.path.to_string(),
            symbol_name: symbol_name.to_string(),
        })
    }

    /// Resolve a single segment within a module, following no-shadowing rules
    fn resolve_segment_in_module(
        &self,
        segment: &str,
        module: &ModuleId,
    ) -> Result<Option<ModuleId>, ResolutionError> {
        if let Some(module_map) = self.module_symbol_maps.get(module) {
            // Check local items first (no-shadowing rule: locals take precedence)
            if let Some(local_item_id) = module_map.get_local_module(segment) {
                return Ok(Some(ModuleId {
                    id: local_item_id.clone(),
                }));
            }

            // Then check resolved imports
            if let Some(imported_item_id) = module_map.get_imported_symbol(segment) {
                // Check if the imported item is a module
                if imported_item_id.kind == ItemKind::Module {
                    return Ok(Some(ModuleId {
                        id: imported_item_id.clone(),
                    }));
                } else {
                    // Imported item is not a module - path resolution fails
                    return Err(ResolutionError::Failed(CompileError::UnresolvedModule {
                        attempted_item: ModuleId::new(format!("{}::{}", module.id.path, segment)),
                        symbol: InternedSymbol::from_text(segment),
                    }));
                }
            }

            // Segment not found in this module - might be resolved later
            Ok(None)
        } else {
            // Module not loaded yet - blocked
            Ok(None)
        }
    }

    /// Try to resolve a simple import (use path::item)
    fn try_resolve_simple_import(
        &self,
        pending_import: &PendingImport,
        ir_program: &Program,
    ) -> Result<Option<ItemId>, CompileError> {
        // Extract the qualified path from the use statement
        let (qualified_path, symbol_name) = match &pending_import.use_statement.path {
            ast::UsePath::Simple(qualified_path, symbol) => (qualified_path, symbol.to_string()),
            _ => {
                // Not a simple import, fall back to old method for now
                let target_module = &pending_import.target_module;
                let symbol_name = &pending_import.symbol_name;

                if let Some(target_module_map) = self.module_symbol_maps.get(target_module.as_str())
                {
                    if let Some(item_id) = target_module_map.has_symbol(symbol_name) {
                        return Ok(Some(item_id.clone()));
                    }
                }
                return Ok(None);
            }
        };

        // Use new path resolution
        match self.resolve_use_clause_path(
            qualified_path,
            &pending_import.importing_module,
            &symbol_name,
            ir_program,
        ) {
            Ok(canonical_path) => {
                // Look up the symbol in the resolved canonical module
                let module_id = ModuleId::new(canonical_path.module_path);
                if let Some(module_map) = self.module_symbol_maps.get(&module_id) {
                    if let Some(item_id) = module_map.has_symbol(&canonical_path.symbol_name) {
                        return Ok(Some(item_id.clone()));
                    }
                }
                // Symbol not found in resolved module
                Ok(None)
            }
            Err(ResolutionError::Blocked) => {
                // Path couldn't be fully resolved yet - try again later
                Ok(None)
            }
            Err(ResolutionError::Failed(err)) => {
                // Permanent resolution failure
                Err(err)
            }
        }
    }

    /// Try to resolve a glob import (use path::*)
    fn try_resolve_glob_import(
        &mut self,
        pending_import: &PendingImport,
        ir_program: &Program,
    ) -> Result<Option<ItemId>, CompileError> {
        let target_module = &pending_import.target_module;

        // Check if target module exists
        if let Some(target_module_map) = self.module_symbol_maps.get(target_module.as_str()) {
            // Collect all symbols from the target module (both local and imported)
            let symbols_to_import: Vec<(String, ItemId)> = target_module_map
                .get_all_symbols()
                .map(|(name, item_id)| (name.clone(), item_id.clone()))
                .collect();

            // Get mutable reference to the importing module's symbol map
            if let Some(importing_module_map) = self
                .module_symbol_maps
                .get_mut(&pending_import.importing_module)
            {
                let mut symbols_imported = 0;

                // Import each symbol from the target module
                // Glob imports are shadowed by existing symbols (both local and explicit imports)
                for (symbol_name, item_id) in symbols_to_import {
                    match importing_module_map.try_add_glob_imported_symbol(
                        symbol_name.clone(),
                        item_id,
                        target_module,
                    ) {
                        Ok(true) => {
                            // Symbol successfully imported
                            symbols_imported += 1;
                        }
                        Ok(false) => {
                            // Symbol was shadowed by existing symbol - this is expected behavior
                            // No warning needed, glob imports are supposed to be shadowed
                        }
                        Err(e) => {
                            // Unexpected error during glob import
                            eprintln!("Warning: Failed to import symbol '{}' from glob import '{}::*': {}", 
                                     symbol_name, target_module, e);
                        }
                    }
                }

                // Return a special marker indicating successful glob import processing
                // We use the importing module path with glob suffix for tracking
                Ok(Some(ItemId::new(
                    format!("{}::*[{}]", target_module, symbols_imported),
                    ItemKind::Module,
                )))
            } else {
                Err(CompileError::SemanticError {
                    message: format!(
                        "Importing module '{}' not found during glob import resolution",
                        pending_import.importing_module
                    ),
                    symbol: InternedSymbol::from_text(&pending_import.importing_module.id.path),
                })
            }
        } else {
            // Target module doesn't exist yet, can't resolve
            Ok(None)
        }
    }

    /// Try to resolve a list import (use path::{item1, item2})
    fn try_resolve_list_import(
        &self,
        pending_import: &PendingImport,
        ir_program: &Program,
    ) -> Result<Option<ItemId>, CompileError> {
        // List imports are handled the same as simple imports since we break them down
        // into individual PendingImport entries during collection
        self.try_resolve_simple_import(pending_import, ir_program)
    }

    /// Extract the target module path from a qualified path (removes the last segment which is the item name)
    fn extract_target_module_from_path(&self, qualified_path: &ast::QualifiedPath) -> String {
        match qualified_path {
            ast::QualifiedPath::Global(segments) => {
                if segments.len() > 1 {
                    format!(
                        "::{}",
                        segments[..segments.len() - 1]
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join("::")
                    )
                } else {
                    "::".to_string()
                }
            }
            ast::QualifiedPath::Absolute(segments) => {
                if segments.len() > 1 {
                    format!(
                        "::{}",
                        segments[..segments.len() - 1]
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join("::")
                    )
                } else {
                    "::".to_string()
                }
            }
            ast::QualifiedPath::Relative(segments) => {
                if segments.len() > 1 {
                    segments[..segments.len() - 1]
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join("::")
                } else {
                    self.symbol_context.current_module.id.path.to_string() // Current module for relative single-segment paths
                }
            }
            ast::QualifiedPath::Super(levels, segments) => {
                // TODO: Handle super paths properly - for now just use current module
                self.symbol_context.current_module.id.path.to_string()
            }
            ast::QualifiedPath::Self_(segments) => {
                // self:: paths target the current module
                self.symbol_context.current_module.id.path.to_string()
            }
            ast::QualifiedPath::External(crate_name, segments) => {
                if segments.len() > 1 {
                    format!(
                        "{}::{}",
                        crate_name.to_string(),
                        segments[..segments.len() - 1]
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join("::")
                    )
                } else {
                    crate_name.to_string()
                }
            }
        }
    }
}