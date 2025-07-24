//! Shared types and data structures for the import system

use super::super::parser::ast::StructDefinition;
use super::super::parser::ast::{QualifiedPath, Visibility};
use super::super::runtime_value::RuntimeValue;
use super::super::InterpreterError;
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a module path in the import system
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModulePath {
    pub segments: Vec<String>,
}

impl ModulePath {
    pub fn new(segments: Vec<String>) -> Self {
        Self { segments }
    }

    pub fn from_string(path: &str) -> Self {
        if path == "global" || path.is_empty() {
            Self { segments: vec![] }
        } else {
            Self {
                segments: path.split("::").map(|s| s.to_string()).collect(),
            }
        }
    }

    pub fn to_string(&self) -> String {
        if self.segments.is_empty() {
            "global".to_string()
        } else {
            self.segments.join("::")
        }
    }

    pub fn parent(&self) -> Option<ModulePath> {
        if self.segments.is_empty() {
            None
        } else {
            Some(ModulePath {
                segments: self.segments[..self.segments.len() - 1].to_vec(),
            })
        }
    }

    pub fn is_ancestor_of(&self, other: &ModulePath) -> bool {
        if self.segments.len() >= other.segments.len() {
            return false;
        }

        for (i, segment) in self.segments.iter().enumerate() {
            if other.segments.get(i) != Some(segment) {
                return false;
            }
        }
        true
    }

    pub fn is_same_crate(&self, _other: &ModulePath) -> bool {
        // For now, all modules are in the same crate
        // This can be extended for multi-crate support
        true
    }
}

/// Context information for import resolution
#[derive(Debug, Clone)]
pub struct ImportContext {
    /// The module performing the import
    pub importing_module: ModulePath,
    /// The target module being imported from  
    pub target_module: ModulePath,
    /// The crate root module
    pub crate_root: ModulePath,
}

impl ImportContext {
    pub fn new(importing_module: ModulePath, target_module: ModulePath) -> Self {
        Self {
            importing_module,
            target_module,
            crate_root: ModulePath::new(vec![]), // Root is empty path
        }
    }

    pub fn is_same_crate(&self) -> bool {
        self.importing_module.is_same_crate(&self.target_module)
    }

    pub fn is_parent_module(&self) -> bool {
        self.target_module.is_ancestor_of(&self.importing_module)
    }

    /// Check if the importing module is a parent of the target module
    /// (used for super visibility checks)
    pub fn is_importing_parent_of_target(&self) -> bool {
        self.importing_module.is_ancestor_of(&self.target_module)
    }

    pub fn is_same_module(&self) -> bool {
        self.importing_module == self.target_module
    }

    pub fn matches_restricted_path(&self, path: &QualifiedPath) -> bool {
        // Convert QualifiedPath to ModulePath and check if importing module matches
        let target_path = match path {
            QualifiedPath::Relative(segments) => ModulePath::new(segments.iter().map(|s| s.to_string()).collect()),
            QualifiedPath::Absolute(segments) => ModulePath::new(segments.iter().map(|s| s.to_string()).collect()),
            QualifiedPath::Global(segments) => ModulePath::new(segments.iter().map(|s| s.to_string()).collect()),
            _ => return false, // TODO: Handle other path types
        };

        self.importing_module == target_path || target_path.is_ancestor_of(&self.importing_module)
    }
}

/// Type of symbol for categorization
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolType {
    Relation,
    PredicateHandle,
    BuiltinRelation,
    Struct,
    Term,
    Type,
}

/// Import type for dependency tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImportType {
    Simple(String),
    Glob,
    Selective(Vec<String>),
}

/// Result of visibility checking
#[derive(Debug, Clone, PartialEq)]
pub enum VisibilityResult {
    Accessible,
    NotAccessible(VisibilityReason),
}

#[derive(Debug, Clone, PartialEq)]
pub enum VisibilityReason {
    Private,
    WrongCrate,
    WrongModule,
    RestrictedPath,
}

/// Collection of accessible symbols from a module
#[derive(Debug)]
pub struct AccessibleSymbols {
    pub values: HashMap<String, RuntimeValue>,
    pub types: HashMap<String, StructDefinition>,
}

impl Clone for AccessibleSymbols {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            types: self.types.clone(),
        }
    }
}

impl AccessibleSymbols {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            types: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.types.is_empty()
    }

    pub fn merge(&mut self, other: AccessibleSymbols) {
        self.values.extend(other.values);
        self.types.extend(other.types);
    }
}

/// Import conflict information
#[derive(Debug, Clone)]
pub struct ImportConflict {
    pub symbol_name: String,
    pub existing_module: ModulePath,
    pub imported_module: ModulePath,
    pub conflict_type: ConflictType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictType {
    ValueValue, // Two runtime values with same name
    TypeType,   // Two types with same name
    ValueType,  // Runtime value conflicts with type name
    TypeValue,  // Type conflicts with runtime value name
}

/// Import warning information
#[derive(Debug, Clone)]
pub struct ImportWarning {
    pub message: String,
    pub symbol_name: Option<String>,
    pub module_path: Option<ModulePath>,
}

/// Performance metrics for import operations
#[derive(Debug, Clone, Default)]
pub struct ImportMetrics {
    pub symbols_processed: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub resolution_time_ms: u64,
}

/// Strategy for resolving import conflicts
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictResolutionStrategy {
    /// Fail on any conflict with detailed error
    Error,
    /// Keep existing symbol, warn about conflict
    PreferExisting,
    /// Replace with imported symbol, warn about conflict
    PreferImported,
    /// Import with qualified name to avoid conflict
    Qualified,
}

/// Circular dependency error with detailed information
#[derive(Debug, Clone)]
pub struct CircularDependencyError {
    pub cycle: Vec<ModulePath>,
    pub import_chain: Vec<ImportType>,
}

/// Enhanced import error types
#[derive(Debug, Clone)]
pub enum ImportError {
    CircularDependency(CircularDependencyError),
    VisibilityViolation {
        symbol: String,
        symbol_module: ModulePath,
        importing_module: ModulePath,
        required_visibility: Visibility,
    },
    SymbolConflict {
        symbol: String,
        existing_module: ModulePath,
        imported_module: ModulePath,
        conflict_type: ConflictType,
    },
    ModuleNotFound {
        requested_path: QualifiedPath,
        searched_paths: Vec<PathBuf>,
    },
    InvalidGlobTarget {
        target_path: QualifiedPath,
        reason: String,
    },
}

impl From<CircularDependencyError> for ImportError {
    fn from(error: CircularDependencyError) -> Self {
        ImportError::CircularDependency(error)
    }
}

impl From<ImportError> for InterpreterError {
    fn from(error: ImportError) -> Self {
        match error {
            ImportError::CircularDependency(cycle_error) => {
                InterpreterError::RuntimeError(format!(
                    "Circular dependency detected: {}",
                    cycle_error
                        .cycle
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(" -> ")
                ))
            }
            ImportError::VisibilityViolation {
                symbol,
                symbol_module,
                importing_module,
                ..
            } => InterpreterError::RuntimeError(format!(
                "Symbol '{}' in module '{}' is not accessible from module '{}'",
                symbol,
                symbol_module.to_string(),
                importing_module.to_string()
            )),
            ImportError::SymbolConflict {
                symbol,
                existing_module,
                imported_module,
                conflict_type,
            } => InterpreterError::RuntimeError(format!(
                "Symbol conflict: '{}' exists in both '{}' and '{}' ({:?})",
                symbol,
                existing_module.to_string(),
                imported_module.to_string(),
                conflict_type
            )),
            ImportError::ModuleNotFound {
                requested_path,
                searched_paths: _,
            } => InterpreterError::ModuleNotFound(PathBuf::from(format!("{:?}", requested_path))),
            ImportError::InvalidGlobTarget {
                target_path,
                reason,
            } => InterpreterError::RuntimeError(format!(
                "Invalid glob import target '{:?}': {}",
                target_path, reason
            )),
        }
    }
}

/// Result of an import operation
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub imported_symbols: AccessibleSymbols,
    pub conflicts: Vec<ImportConflict>,
    pub warnings: Vec<ImportWarning>,
    pub metrics: ImportMetrics,
}
