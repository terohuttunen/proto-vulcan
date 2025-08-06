//! Error types for IR compilation

use super::ir::{ItemId, ModuleId, PredicateId, TypeId};
use crate::interpreter::compiler::PendingImport;
use crate::interpreter::symbol_table::InternedSymbol;
use crate::interpreter::InterpreterError;
use thiserror::Error;

/// Warnings that can occur during IR compilation
#[derive(Debug, Clone)]
pub enum CompileWarning {
    #[allow(dead_code)]
    ShadowingWarning {
        symbol_name: String,
        import_source: String,
        local_symbol: InternedSymbol,
        import_symbol: InternedSymbol,
    },

    #[allow(dead_code)]
    AmbiguousImportWarning {
        symbol_name: String,
        sources: Vec<String>,
        symbol: InternedSymbol,
    },

    #[allow(dead_code)]
    UnusedImport {
        symbol_name: String,
        import_source: String,
        symbol: InternedSymbol,
    },
}

impl std::fmt::Display for CompileWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileWarning::ShadowingWarning {
                symbol_name,
                import_source,
                local_symbol,
                import_symbol,
            } => {
                write!(
                    f,
                    "Warning: Symbol '{}' imported from '{}' shadows local symbol defined at {}:{}",
                    symbol_name,
                    import_source,
                    local_symbol.file_path().display(),
                    local_symbol.span()
                )
            }
            CompileWarning::AmbiguousImportWarning {
                symbol_name,
                sources,
                symbol,
            } => {
                write!(f, "Warning: Ambiguous import: symbol '{}' can be resolved from multiple sources: [{}] at {}:{}", 
                       symbol_name, sources.join(", "), symbol.file_path().display(), symbol.span())
            }
            CompileWarning::UnusedImport {
                symbol_name,
                import_source,
                symbol,
            } => {
                write!(
                    f,
                    "Warning: Unused import: symbol '{}' from '{}' at {}:{}",
                    symbol_name,
                    import_source,
                    symbol.file_path().display(),
                    symbol.span()
                )
            }
        }
    }
}

/// Compilation options controlling validation behavior
#[derive(Debug, Clone)]
pub struct CompilationOptions {
    /// Treat warnings as errors
    pub strict_mode: bool,
    /// Enable shadowing warnings
    pub warn_shadowing: bool,
    /// Enable unused import warnings
    pub warn_unused_imports: bool,
}

impl Default for CompilationOptions {
    fn default() -> Self {
        Self {
            strict_mode: false,
            warn_shadowing: true,
            warn_unused_imports: true,
        }
    }
}

/// Context for collecting warnings and errors during compilation
#[derive(Debug, Clone)]
pub struct CompilationContext {
    /// Collected warnings during compilation
    pub warnings: Vec<CompileWarning>,
    /// Compilation options
    pub options: CompilationOptions,
}

impl CompilationContext {
    pub fn new(options: CompilationOptions) -> Self {
        Self {
            warnings: Vec::new(),
            options,
        }
    }

    /// Add a warning to the compilation context
    pub fn add_warning(&mut self, warning: CompileWarning) {
        self.warnings.push(warning);
    }

    /// Convert warnings to errors if in strict mode
    pub fn validate_warnings(&self) -> Result<(), CompileError> {
        if self.options.strict_mode && !self.warnings.is_empty() {
            // Convert first warning to error in strict mode
            let first_warning = &self.warnings[0];
            return Err(self.warning_to_error(first_warning));
        }
        Ok(())
    }

    /// Convert a warning to an error
    fn warning_to_error(&self, warning: &CompileWarning) -> CompileError {
        match warning {
            CompileWarning::ShadowingWarning {
                symbol_name,
                import_source,
                local_symbol,
                import_symbol,
            } => CompileError::ShadowingError {
                symbol_name: symbol_name.clone(),
                import_source: import_source.clone(),
                local_symbol: local_symbol.clone(),
                import_symbol: import_symbol.clone(),
            },
            CompileWarning::AmbiguousImportWarning {
                symbol_name,
                sources,
                symbol,
            } => CompileError::AmbiguousImport {
                symbol_name: symbol_name.clone(),
                sources: sources.clone(),
                symbol: symbol.clone(),
            },
            CompileWarning::UnusedImport {
                symbol_name,
                import_source,
                symbol,
            } => CompileError::ConflictingSymbol {
                symbol_name: symbol_name.clone(),
                conflict_description: format!("Unused import from '{}'", import_source),
                symbol: symbol.clone(),
                related_symbols: vec![],
            },
        }
    }

    /// Get all warnings
    pub fn get_warnings(&self) -> &[CompileWarning] {
        &self.warnings
    }

    /// Clear all warnings
    pub fn clear_warnings(&mut self) {
        self.warnings.clear();
    }
}

/// Convert CompileError to CompileWarning (used when errors should be demoted to warnings)
impl From<CompileError> for CompileWarning {
    fn from(error: CompileError) -> Self {
        match error {
            CompileError::ShadowingError {
                symbol_name,
                import_source,
                local_symbol,
                import_symbol,
            } => CompileWarning::ShadowingWarning {
                symbol_name,
                import_source,
                local_symbol,
                import_symbol,
            },
            CompileError::AmbiguousImport {
                symbol_name,
                sources,
                symbol,
            } => CompileWarning::AmbiguousImportWarning {
                symbol_name,
                sources,
                symbol,
            },
            _ => {
                // For other errors, create a generic unused import warning as fallback
                CompileWarning::UnusedImport {
                    symbol_name: "unknown".to_string(),
                    import_source: "unknown".to_string(),
                    symbol: InternedSymbol::from_text("unknown"),
                }
            }
        }
    }
}

/// Errors that can occur during IR compilation
#[derive(Debug, Error, Clone)]
pub enum CompileError {
    #[error("Cannot resolve type '{attempted_item}' at {}:{}", symbol.file_path().display(), symbol.span())]
    UnresolvedType {
        /// The type we attempted to resolve
        attempted_item: TypeId,
        /// Symbol with precise source location
        symbol: InternedSymbol,
    },

    #[error("Cannot resolve predicate '{attempted_item}' at {}:{}", symbol.file_path().display(), symbol.span())]
    UnresolvedPredicate {
        /// The predicate we attempted to resolve
        attempted_item: PredicateId,
        /// Symbol with precise source location
        symbol: InternedSymbol,
    },

    #[error("Cannot resolve module '{attempted_item}' at {}:{}", symbol.file_path().display(), symbol.span())]
    UnresolvedModule {
        /// The module we attempted to resolve
        attempted_item: ModuleId,
        /// Symbol with precise source location
        symbol: InternedSymbol,
    },

    #[error("Cannot resolve imports  '{imports:?}' ")]
    UnresolvedImports { imports: Vec<PendingImport> },

    #[error("Arity mismatch for predicate '{predicate_item}' at {}:{}: expected {expected_arity} arguments, found {actual_arity}", symbol.file_path().display(), symbol.span())]
    ArityMismatch {
        /// The predicate being called
        predicate_item: PredicateId,
        /// Expected number of arguments
        expected_arity: usize,
        /// Actual number of arguments provided
        actual_arity: usize,
        /// Symbol with precise source location
        symbol: InternedSymbol,
    },

    #[error("Semantic analysis error: {message} at {}:{}", symbol.file_path().display(), symbol.span())]
    SemanticError {
        /// Error message
        message: String,
        /// Symbol with precise source location
        symbol: InternedSymbol,
    },

    #[error("Environment error: {0}")]
    Environment(#[from] InterpreterError),

    #[error("Duplicate item: {item} at {}:{}", symbol.file_path().display(), symbol.span())]
    DuplicateItem {
        item: ItemId,
        /// Symbol with precise source location
        symbol: InternedSymbol,
    },

    #[error("Cannot resolve reference '{attempted_item}' at {}:{}", symbol.file_path().display(), symbol.span())]
    UnresolvedReference {
        /// The item we attempted to resolve
        attempted_item: ItemId,
        /// Symbol with precise source location
        symbol: InternedSymbol,
    },

    #[error("Duplicate field '{field_name}' at {}:{}", symbol.file_path().display(), symbol.span())]
    DuplicateField {
        /// The field name that was duplicated
        field_name: String,
        /// Symbol with precise source location
        symbol: InternedSymbol,
    },

    #[error("Duplicate enum variant '{variant_name}' at {}:{}", symbol.file_path().display(), symbol.span())]
    DuplicateEnumVariant {
        /// The variant name that was duplicated
        variant_name: String,
        /// Symbol with precise source location
        symbol: InternedSymbol,
    },

    #[error("Duplicate parameter '{parameter_name}' at {}:{}", symbol.file_path().display(), symbol.span())]
    DuplicateParameter {
        /// The parameter name that was duplicated
        parameter_name: String,
        /// Symbol with precise source location
        symbol: InternedSymbol,
    },

    #[error("Circular dependency detected: {cycle} at {}:{}", symbol.file_path().display(), symbol.span())]
    CircularDependency {
        /// The cycle description
        cycle: String,
        /// Symbol with precise source location
        symbol: InternedSymbol,
    },

    #[error("Symbol '{symbol_name}' imported from '{import_source}' shadows local symbol defined at {}:{}", local_symbol.file_path().display(), local_symbol.span())]
    ShadowingError {
        /// The name of the conflicting symbol
        symbol_name: String,
        /// The source of the import that causes shadowing
        import_source: String,
        /// Location of the local symbol being shadowed
        local_symbol: InternedSymbol,
        /// Location of the import causing the shadow
        import_symbol: InternedSymbol,
    },

    #[error("Ambiguous import: symbol '{symbol_name}' can be resolved from multiple sources: [{}] at {}:{}", sources.join(", "), symbol.file_path().display(), symbol.span())]
    AmbiguousImport {
        /// The name of the ambiguous symbol
        symbol_name: String,
        /// List of possible sources for the symbol
        sources: Vec<String>,
        /// Symbol with precise source location
        symbol: InternedSymbol,
    },

    #[error("Conflicting symbol '{symbol_name}': {conflict_description} at {}:{}", symbol.file_path().display(), symbol.span())]
    ConflictingSymbol {
        /// The name of the conflicting symbol
        symbol_name: String,
        /// Description of the conflict
        conflict_description: String,
        /// Symbol with precise source location
        symbol: InternedSymbol,
        /// Related symbols involved in the conflict
        related_symbols: Vec<InternedSymbol>,
    },

    #[error("Invalid parameter type for '{parameter_name}' at {}:{}: expected {expected_type}, got {actual_type}", symbol.file_path().display(), symbol.span())]
    InvalidParameterType {
        /// The parameter name with invalid type
        parameter_name: String,
        /// The expected type annotation
        expected_type: String,
        /// The actual type provided
        actual_type: String,
        /// Symbol with precise source location
        symbol: InternedSymbol,
    },

    #[error("Module '{target}' is not accessible from module '{from}'")]
    ModuleNotAccessible {
        /// The target module that cannot be accessed
        target: ModuleId,
        /// The module attempting to access the target
        from: ModuleId,
    },

    #[error("Ambiguous glob import in module '{importing_module}': symbol '{symbol}' is available from both '{source1}' and '{source2}'")]
    AmbiguousGlobImport {
        /// The ambiguous symbol name
        symbol: super::ir::ItemName,
        /// First source module
        source1: ModuleId,
        /// Second source module  
        source2: ModuleId,
        /// Module where the glob import is located
        importing_module: ModuleId,
    },
}

/// Result of path resolution during use-clause resolution
#[derive(Debug, Clone)]
pub enum ResolutionError {
    /// Temporarily blocked - segment not resolved yet, should retry in next iteration
    Blocked,
    /// Permanent failure - segment doesn't exist
    Failed(CompileError),
}

/// A fully resolved canonical path to an item
#[derive(Debug, Clone)]
pub struct CanonicalPath {
    /// Absolute module tree path (e.g., "::std::collections")
    pub module_path: String,
    /// Final symbol name (e.g., "HashMap")
    pub symbol_name: String,
}

impl CompileError {
    /// Get the source location information from this error, if available
    pub fn source_location(
        &self,
    ) -> Option<(
        std::path::PathBuf,
        crate::interpreter::parser::ast::Location,
    )> {
        match self {
            CompileError::UnresolvedType { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::UnresolvedPredicate { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::UnresolvedModule { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::UnresolvedImports { .. } => None,
            CompileError::ArityMismatch { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::SemanticError { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::DuplicateItem { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::UnresolvedReference { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::DuplicateField { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::DuplicateEnumVariant { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::DuplicateParameter { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::CircularDependency { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::ShadowingError { import_symbol, .. } => {
                Some((import_symbol.file_path().clone(), import_symbol.span()))
            }
            CompileError::AmbiguousImport { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::ConflictingSymbol { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::InvalidParameterType { symbol, .. } => {
                Some((symbol.file_path().clone(), symbol.span()))
            }
            CompileError::ModuleNotAccessible { .. } => None,
            CompileError::AmbiguousGlobImport { .. } => None,
            CompileError::Environment(_) => None,
        }
    }

    /// Get the symbol associated with this error, if available
    pub fn symbol(&self) -> Option<&InternedSymbol> {
        match self {
            CompileError::UnresolvedType { symbol, .. } => Some(symbol),
            CompileError::UnresolvedPredicate { symbol, .. } => Some(symbol),
            CompileError::UnresolvedModule { symbol, .. } => Some(symbol),
            CompileError::UnresolvedImports { .. } => None,
            CompileError::ArityMismatch { symbol, .. } => Some(symbol),
            CompileError::SemanticError { symbol, .. } => Some(symbol),
            CompileError::DuplicateItem { symbol, .. } => Some(symbol),
            CompileError::UnresolvedReference { symbol, .. } => Some(symbol),
            CompileError::DuplicateField { symbol, .. } => Some(symbol),
            CompileError::DuplicateEnumVariant { symbol, .. } => Some(symbol),
            CompileError::DuplicateParameter { symbol, .. } => Some(symbol),
            CompileError::CircularDependency { symbol, .. } => Some(symbol),
            CompileError::ShadowingError { import_symbol, .. } => Some(import_symbol),
            CompileError::AmbiguousImport { symbol, .. } => Some(symbol),
            CompileError::ConflictingSymbol { symbol, .. } => Some(symbol),
            CompileError::InvalidParameterType { symbol, .. } => Some(symbol),
            CompileError::ModuleNotAccessible { .. } => None,
            CompileError::AmbiguousGlobImport { .. } => None,
            CompileError::Environment(_) => None,
        }
    }
}
