//! IR Compiler - transforms AST to IR with full symbol resolution
//!
//! The compiler is organized into focused modules:
//! - `symbol_collection`: Phase 1 - Collect all symbols and use clauses
//! - `resolution`: Phase 2 - Resolve imports and build symbol maps  
//! - `compilation`: Phase 3 - Compile AST bodies to IR with full symbol resolution
//!
//! Uses a three-phase compilation approach:
//! 1. Symbol collection: Collect all type, predicate, and module declarations
//! 2. Import resolution: Resolve use clauses and build final symbol maps
//! 3. Body compilation: Compile bodies with full symbol resolution

use super::*;
use crate::engine::{Engine, DefaultEngine};
use crate::interpreter::parser::ast;
use crate::interpreter::symbol_table::InternedSymbol;
use crate::interpreter::constraint_domains::{ConstraintDomainRegistry, VariableInfo, VariableType};
use crate::user::{User, DefaultUser};
use std::collections::HashMap;

pub use super::errors::{CompileError, CompileWarning, CompilationContext, CompilationOptions};
use super::errors::{CanonicalPath, ResolutionError};

// Import phase modules
mod symbol_collection;
mod resolution;
mod compilation;

// Re-export types used by multiple phases
pub(super) use symbol_collection::{SymbolContext, CompilationPhase, ModuleSymbolMap, PendingImport};

/// The IR compiler transforms AST to IR with full symbol resolution
pub struct Compiler<U: User, E: Engine<U>> {
    /// Current module path stack for resolving relative paths
    pub(super) module_path_stack: Vec<String>,
    /// Symbol resolution context
    pub(super) symbol_context: SymbolContext,
    /// Compilation phases tracking
    pub(super) compilation_phase: CompilationPhase,
    /// Fast symbol lookup maps per module (compilation-time only)
    pub(super) module_symbol_maps: HashMap<ModuleId, ModuleSymbolMap>,
    /// Global list of pending imports to be resolved (simplified architecture)
    pub(super) pending_imports: Vec<PendingImport>,
    /// Compilation context for warnings and validation
    pub(super) compilation_context: CompilationContext,
    /// Constraint domain registry for template compilation
    pub(super) constraint_domains: ConstraintDomainRegistry<DefaultUser, DefaultEngine<DefaultUser>>,
    /// Marker for generic parameters
    _phantom: std::marker::PhantomData<(U, E)>,
}

impl<U: User, E: Engine<U>> Compiler<U, E> {
    /// Create a new IR compiler
    pub fn new() -> Self {
        Self::with_options(CompilationOptions::default())
    }
    
    /// Create a new IR compiler with specific compilation options
    pub fn with_options(options: CompilationOptions) -> Self {
        let mut module_symbol_maps = HashMap::new();
        // Initialize symbol map for the global module
        module_symbol_maps.insert(ModuleId::new("::"), ModuleSymbolMap::new());

        Self {
            module_path_stack: vec!["::".to_string()], // Start at global module
            symbol_context: SymbolContext {
                current_module: ModuleId::new("::"),
            },
            compilation_phase: CompilationPhase::SymbolAndUseClauseCollection,
            module_symbol_maps,
            pending_imports: Vec::new(),
            compilation_context: CompilationContext::new(options),
            constraint_domains: ConstraintDomainRegistry::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Compile an AST program to IR using multi-phase compilation
    pub fn compile_from_ast(&mut self, program: ast::Program) -> Result<Program, CompileError> {
        let mut ir_program = Program::new();

        // Phase 1: Collect symbols and use clauses in a single pass
        self.compilation_phase = CompilationPhase::SymbolAndUseClauseCollection;
        self.collect_symbols_and_use_clauses(&program, &mut ir_program)?;

        // Phase 2: Import resolution
        self.compilation_phase = CompilationPhase::ImportResolution;
        self.resolve_imports(&mut ir_program)?;

        // Phase 3: Body compilation
        self.compilation_phase = CompilationPhase::BodyCompilation;
        self.compile_bodies(&program, &mut ir_program)?;

        // Phase 4: Validation
        self.compilation_phase = CompilationPhase::Validation;
        self.validate_compilation(&ir_program)?;

        Ok(ir_program)
    }
    
    /// Add a warning to the compilation context
    pub(super) fn add_warning(&mut self, warning: CompileWarning) {
        self.compilation_context.add_warning(warning);
    }
    
    /// Get all collected warnings
    pub fn get_warnings(&self) -> &[CompileWarning] {
        self.compilation_context.get_warnings()
    }
    
    /// Clear all warnings
    pub fn clear_warnings(&mut self) {
        self.compilation_context.clear_warnings();
    }
    
    /// Get compilation options
    pub fn get_options(&self) -> &CompilationOptions {
        &self.compilation_context.options
    }
    
    /// Phase 4: Validate compilation and handle warnings
    fn validate_compilation(&self, ir_program: &Program) -> Result<(), CompileError> {
        // Run existing IR validation
        let validator = super::validation::Validator::new();
        validator.validate_program(ir_program)?;
        
        // Validate warnings based on compilation options
        self.compilation_context.validate_warnings()?;
        
        Ok(())
    }

    /// Shared utility: Resolve item path from current module context
    pub(super) fn resolve_item_path(&self, name: &str) -> String {
        if name.starts_with("::") {
            // Absolute path
            name.to_string()
        } else {
            // Relative path - resolve from current module
            let current_path = &self.symbol_context.current_module.id.path;
            if current_path.as_ref() == "::" {
                // At global module, just add global prefix
                format!("::{}", name)
            } else {
                // Nested module - append to current path
                format!("{}::{}", current_path.as_ref(), name)
            }
        }
    }

    /// Shared utility: Convert AST visibility to IR visibility
    pub(super) fn convert_visibility(&self, ast_vis: &ast::Visibility) -> Result<Visibility, CompileError> {
        match ast_vis {
            ast::Visibility::Public => Ok(Visibility::Public),
            ast::Visibility::Private => Ok(Visibility::Private),
            ast::Visibility::Crate => Ok(Visibility::Public), // Treat crate as public for now
            ast::Visibility::Super => Ok(Visibility::Public), // Treat super as public for now  
            ast::Visibility::SelfModule => Ok(Visibility::Private), // Same as private
            ast::Visibility::Restricted(_) => Ok(Visibility::Public), // Treat restricted as public for now
        }
    }

    /// Shared utility: Convert qualified path to string representation
    pub(super) fn qualified_path_to_string(&self, path: &ast::QualifiedPath) -> String {
        match path {
            ast::QualifiedPath::Global(segments) => {
                format!("::{}", segments.iter().map(|s| s.to_string()).collect::<Vec<_>>().join("::"))
            }
            ast::QualifiedPath::Absolute(segments) => {
                format!("::{}", segments.iter().map(|s| s.to_string()).collect::<Vec<_>>().join("::"))
            }
            ast::QualifiedPath::Relative(segments) => {
                let current_path = &self.symbol_context.current_module.id.path;
                let base = if current_path.as_ref() == "::" {
                    "::".to_string()
                } else {
                    current_path.as_ref().to_string()
                };
                if segments.is_empty() {
                    base
                } else {
                    format!("{}::{}", base, segments.iter().map(|s| s.to_string()).collect::<Vec<_>>().join("::"))
                }
            }
            ast::QualifiedPath::Self_(segments) => {
                // 'self' refers to current module
                let current_path = &self.symbol_context.current_module.id.path;
                if segments.is_empty() {
                    current_path.as_ref().to_string()
                } else {
                    format!("{}::{}", current_path.as_ref(), segments.iter().map(|s| s.to_string()).collect::<Vec<_>>().join("::"))
                }
            }
            ast::QualifiedPath::Super(levels, segments) => {
                // Get current module from registry to find parent
                // This is a simplified implementation - full implementation would use Module.parent
                let current_path = &self.symbol_context.current_module.id.path;
                let parent_path = if let Some(last_sep) = current_path.as_ref().rfind("::") {
                    if last_sep == 0 {
                        "::".to_string() // Parent is global module
                    } else {
                        current_path.as_ref()[..last_sep].to_string()
                    }
                } else {
                    "::".to_string() // Already at top level
                };

                if segments.is_empty() {
                    parent_path
                } else {
                    format!("{}::{}", parent_path, segments.iter().map(|s| s.to_string()).collect::<Vec<_>>().join("::"))
                }
            }
            ast::QualifiedPath::External(crate_name, segments) => {
                if segments.is_empty() {
                    format!("::{}", crate_name)
                } else {
                    format!("::{}::{}", crate_name, segments.iter().map(|s| s.to_string()).collect::<Vec<_>>().join("::"))
                }
            }
        }
    }
}

impl<U: User, E: Engine<U>> Default for Compiler<U, E> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::DefaultEngine;
    use crate::user::DefaultUser;

    #[test]
    fn test_compiler_creation() {
        let _compiler: Compiler<DefaultUser, DefaultEngine<DefaultUser>> = Compiler::new();
    }

    #[test]
    fn test_resolve_item_path() {
        let compiler: Compiler<DefaultUser, DefaultEngine<DefaultUser>> = Compiler::new();
        
        // Test absolute paths
        assert_eq!(compiler.resolve_item_path("::std::list"), "::std::list");
        
        // Test relative paths from global module
        assert_eq!(compiler.resolve_item_path("MyType"), "::MyType");
    }

    #[test]
    fn test_qualified_path_to_string() {
        let compiler: Compiler<DefaultUser, DefaultEngine<DefaultUser>> = Compiler::new();
        
        // Test global path
        let global_path = ast::QualifiedPath::Global(vec![
            InternedSymbol::from_text("std"),
            InternedSymbol::from_text("list"),
        ]);
        assert_eq!(compiler.qualified_path_to_string(&global_path), "::std::list");
    }
}