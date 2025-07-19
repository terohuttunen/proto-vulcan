//! Import system for Proto-Vulcan interpreter
//!
//! This module provides a comprehensive solution for imports that handles:
//! - Visibility-aware symbol resolution
//! - Memory-efficient symbol management  
//! - Circular dependency detection
//! - Intelligent conflict resolution
//! - Performance optimization through caching

pub mod dependency;
pub mod registry;
pub mod resolver;
pub mod types;
pub mod visibility;

// Re-export main public API
pub use dependency::DependencyTracker;
pub use registry::{SymbolId, SymbolRef, SymbolRegistry};
pub use resolver::ImportResolver;
pub use types::*;
pub use visibility::VisibilityChecker;
