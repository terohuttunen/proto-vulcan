//! Main import resolver that orchestrates the comprehensive glob import system

use super::super::environment::ModuleInfo;
use super::super::parser::ast::QualifiedPath;
use super::super::parser::ast::StructDefinition;
use super::super::runtime_value::RuntimeValue;
use super::dependency::DependencyTracker;
use super::registry::SymbolRegistry;
use super::types::*;
use super::visibility::{SymbolAccessibility, VisibilityChecker};
use std::collections::HashMap;
use std::time::Instant;

/// Main import resolver that orchestrates the three-phase import process
pub struct ImportResolver {
    /// Visibility checker for context-aware symbol access control
    visibility_checker: VisibilityChecker,

    /// Symbol collector for efficient symbol gathering
    symbol_collector: SymbolCollector,

    /// Conflict resolver for intelligent namespace management
    conflict_resolver: ConflictResolver,

    /// Import cache for performance optimization
    import_cache: ImportCache,

    /// Dependency tracker for cycle detection
    dependency_tracker: DependencyTracker,

    /// Symbol registry for memory-efficient symbol management
    symbol_registry: SymbolRegistry,
}

impl ImportResolver {
    pub fn new() -> Self {
        Self {
            visibility_checker: VisibilityChecker::new(),
            symbol_collector: SymbolCollector::new(),
            conflict_resolver: ConflictResolver::new(ConflictResolutionStrategy::Error),
            import_cache: ImportCache::new(),
            dependency_tracker: DependencyTracker::new(),
            symbol_registry: SymbolRegistry::new(),
        }
    }

    /// Configure conflict resolution strategy
    pub fn set_conflict_strategy(&mut self, strategy: ConflictResolutionStrategy) {
        self.conflict_resolver.set_strategy(strategy);
    }

    /// Register a module in the import system
    pub fn register_module(&mut self, module_path: ModulePath, parent: Option<ModulePath>) {
        self.visibility_checker.register_module(module_path, parent);
    }

    /// Perform enhanced glob import with full visibility support
    pub fn import_glob_enhanced(
        &mut self,
        target_path: &QualifiedPath,
        importing_module: ModulePath,
        loaded_modules: &HashMap<String, ModuleInfo>,
        existing_globals: &HashMap<String, RuntimeValue>,
        existing_types: &HashMap<String, StructDefinition>,
    ) -> Result<ImportResult, ImportError> {
        let start_time = Instant::now();

        // Phase 1: Discovery - Resolve target module and validate path
        let target_module = self.visibility_checker.validate_qualified_path(
            target_path,
            &ImportContext::new(
                importing_module.clone(),
                ModulePath::new(vec![]), // Will be set after resolution
            ),
        )?;

        let context = ImportContext::new(importing_module.clone(), target_module.clone());

        // Check for circular dependencies
        self.dependency_tracker
            .check_circular_dependency(importing_module.clone(), target_module.clone())?;

        // Check cache first
        let cache_key = CacheKey::new(
            target_module.clone(),
            importing_module.clone(),
            ImportType::Glob,
        );
        if let Some(cached_result) = self.import_cache.get(&cache_key) {
            let mut metrics = ImportMetrics::default();
            metrics.cache_hits = 1;
            metrics.resolution_time_ms = start_time.elapsed().as_millis() as u64;

            return Ok(ImportResult {
                imported_symbols: cached_result.clone(),
                conflicts: vec![],
                warnings: vec![],
                metrics,
            });
        }

        // Phase 2: Filtering - Collect accessible symbols based on visibility
        let target_module_info =
            loaded_modules
                .get(&target_module.to_string())
                .ok_or_else(|| ImportError::ModuleNotFound {
                    requested_path: target_path.clone(),
                    searched_paths: vec![], // TODO: Populate with actual search paths
                })?;

        // Use enhanced symbol collector to gather symbols with metadata
        let accessible_symbols = self.symbol_collector.collect_accessible_symbols(
            target_module_info,
            &context,
            &self.visibility_checker,
        )?;

        // Phase 3: Integration - Resolve conflicts and merge symbols
        // First register existing symbols to detect conflicts
        self.conflict_resolver
            .register_existing_symbols(existing_globals, existing_types);

        let conflict_result = self
            .conflict_resolver
            .resolve_conflicts(accessible_symbols, &context)?;

        // Update dependency graph
        self.dependency_tracker.add_import_edge(
            importing_module,
            target_module.clone(),
            ImportType::Glob,
        );

        // Cache the result
        self.import_cache
            .insert(cache_key, conflict_result.resolved_symbols.clone());

        // Prepare metrics
        let mut metrics = ImportMetrics::default();
        metrics.symbols_processed = conflict_result.resolved_symbols.values.len()
            + conflict_result.resolved_symbols.types.len();
        metrics.cache_misses = 1;
        metrics.resolution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(ImportResult {
            imported_symbols: conflict_result.resolved_symbols,
            conflicts: conflict_result.conflicts,
            warnings: conflict_result.warnings,
            metrics,
        })
    }

    /// Perform selective import (use module::{item1, item2})
    pub fn import_selective(
        &mut self,
        target_path: &QualifiedPath,
        requested_symbols: &[(String, Option<String>)], // (name, alias)
        importing_module: ModulePath,
        loaded_modules: &HashMap<String, ModuleInfo>,
        existing_globals: &HashMap<String, RuntimeValue>,
        existing_types: &HashMap<String, StructDefinition>,
    ) -> Result<ImportResult, ImportError> {
        let start_time = Instant::now();

        // Resolve target module
        let target_module = self.visibility_checker.validate_qualified_path(
            target_path,
            &ImportContext::new(importing_module.clone(), ModulePath::new(vec![])),
        )?;

        let context = ImportContext::new(importing_module.clone(), target_module.clone());

        // Check for circular dependencies
        self.dependency_tracker
            .check_circular_dependency(importing_module.clone(), target_module.clone())?;

        // Get target module info
        let target_module_info =
            loaded_modules
                .get(&target_module.to_string())
                .ok_or_else(|| ImportError::ModuleNotFound {
                    requested_path: target_path.clone(),
                    searched_paths: vec![],
                })?;

        // Collect only requested symbols
        let mut accessible_symbols = AccessibleSymbols::new();
        let mut warnings = Vec::new();

        for (symbol_name, alias) in requested_symbols {
            let import_name = alias.as_ref().unwrap_or(symbol_name);

            if let Some(accessibility) = self.visibility_checker.check_symbol_accessibility(
                symbol_name,
                target_module_info,
                &context,
            ) {
                match accessibility {
                    SymbolAccessibility::Value(value) => {
                        accessible_symbols.values.insert(import_name.clone(), value);
                    }
                    SymbolAccessibility::Type(type_def) => {
                        accessible_symbols
                            .types
                            .insert(import_name.clone(), type_def);
                    }
                }
            } else {
                warnings.push(ImportWarning {
                    message: format!(
                        "Symbol '{}' not found or not accessible in module '{}'",
                        symbol_name,
                        target_module.to_string()
                    ),
                    symbol_name: Some(symbol_name.clone()),
                    module_path: Some(target_module.clone()),
                });
            }
        }

        // Resolve conflicts (should be minimal for selective imports)
        // First register existing symbols to detect conflicts
        self.conflict_resolver
            .register_existing_symbols(existing_globals, existing_types);

        let conflict_result = self
            .conflict_resolver
            .resolve_conflicts(accessible_symbols, &context)?;

        // Update dependency graph
        let import_symbols: Vec<String> = requested_symbols
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        self.dependency_tracker.add_import_edge(
            importing_module,
            target_module,
            ImportType::Selective(import_symbols),
        );

        let mut metrics = ImportMetrics::default();
        metrics.symbols_processed = conflict_result.resolved_symbols.values.len()
            + conflict_result.resolved_symbols.types.len();
        metrics.resolution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(ImportResult {
            imported_symbols: conflict_result.resolved_symbols,
            conflicts: conflict_result.conflicts,
            warnings: [warnings, conflict_result.warnings].concat(),
            metrics,
        })
    }

    /// Get import statistics for debugging and monitoring
    pub fn get_import_stats(&self) -> ImportStats {
        ImportStats {
            dependency_stats: self.dependency_tracker.get_stats(),
            registry_stats: self.symbol_registry.get_stats(),
            cache_stats: self.import_cache.get_stats(),
            conflict_stats: self.conflict_resolver.get_stats(),
            collection_stats: self.symbol_collector.get_stats(),
        }
    }

    /// Clear all import data (useful for testing)
    pub fn clear(&mut self) {
        self.dependency_tracker.clear();
        self.import_cache.clear();
    }

    /// Perform garbage collection on cached data
    pub fn garbage_collect(&mut self) {
        self.symbol_registry.garbage_collect();
        self.import_cache.garbage_collect();
    }
}

/// Symbol collector for gathering symbols based on visibility rules
struct SymbolCollector {
    /// Cache for symbol collection results
    collection_cache: HashMap<(ModulePath, ModulePath), AccessibleSymbols>,
    /// Statistics for collection operations
    collection_stats: SymbolCollectionStats,
}

impl SymbolCollector {
    fn new() -> Self {
        Self {
            collection_cache: HashMap::new(),
            collection_stats: SymbolCollectionStats::default(),
        }
    }

    /// Collect accessible symbols from a module with enhanced metadata and filtering
    fn collect_accessible_symbols(
        &mut self,
        module_info: &ModuleInfo,
        context: &ImportContext,
        visibility_checker: &VisibilityChecker,
    ) -> Result<AccessibleSymbols, ImportError> {
        let start_time = std::time::Instant::now();

        // Check cache first
        let cache_key = (
            context.importing_module.clone(),
            context.target_module.clone(),
        );
        if let Some(cached_symbols) = self.collection_cache.get(&cache_key) {
            self.collection_stats.cache_hits += 1;
            return Ok(cached_symbols.clone());
        }

        self.collection_stats.cache_misses += 1;

        // Use visibility checker to get base accessible symbols
        let mut accessible_symbols =
            visibility_checker.get_accessible_symbols(module_info, context);

        // Apply additional filtering and enhancement
        self.apply_advanced_filtering(&mut accessible_symbols, context)?;

        // Add symbol metadata and analysis
        self.enhance_symbol_metadata(&mut accessible_symbols, module_info, context)?;

        // Update statistics
        self.collection_stats.symbols_collected +=
            accessible_symbols.values.len() + accessible_symbols.types.len();
        self.collection_stats.collection_time_ms += start_time.elapsed().as_millis() as u64;

        // Cache the result
        self.collection_cache
            .insert(cache_key, accessible_symbols.clone());

        Ok(accessible_symbols)
    }

    /// Apply advanced filtering rules beyond basic visibility
    fn apply_advanced_filtering(
        &self,
        symbols: &mut AccessibleSymbols,
        context: &ImportContext,
    ) -> Result<(), ImportError> {
        // Filter out symbols that shouldn't be imported
        // For example, remove internal symbols, deprecated symbols, etc.

        let mut symbols_to_remove = Vec::new();

        // Check for symbols with names indicating they shouldn't be imported
        for name in symbols.values.keys() {
            if self.should_filter_symbol(name, context) {
                symbols_to_remove.push(name.clone());
            }
        }

        // Remove filtered symbols
        for name in symbols_to_remove {
            symbols.values.remove(&name);
        }

        // Apply similar filtering to types
        let mut types_to_remove = Vec::new();
        for name in symbols.types.keys() {
            if self.should_filter_symbol(name, context) {
                types_to_remove.push(name.clone());
            }
        }

        for name in types_to_remove {
            symbols.types.remove(&name);
        }

        Ok(())
    }

    /// Determine if a symbol should be filtered out
    fn should_filter_symbol(&self, name: &str, _context: &ImportContext) -> bool {
        // Filter internal symbols (starting with underscore)
        if name.starts_with('_') && !name.starts_with("__builtin_") {
            return true;
        }

        // Filter compiler-generated symbols
        if name.contains("$") || name.contains("#") {
            return true;
        }

        // Filter test-only symbols in production imports
        if name.starts_with("test_") || name.ends_with("_test") {
            return true;
        }

        false
    }

    /// Enhance symbols with additional metadata and analysis
    fn enhance_symbol_metadata(
        &self,
        symbols: &mut AccessibleSymbols,
        _module_info: &ModuleInfo,
        _context: &ImportContext,
    ) -> Result<(), ImportError> {
        // For now, symbols are already enhanced through the visibility checker
        // This is where we could add additional metadata like:
        // - Symbol documentation
        // - Deprecation warnings
        // - Usage statistics
        // - Compatibility information

        // Count symbols for statistics
        self.collection_stats.total_symbols_processed.fetch_add(
            symbols.values.len() + symbols.types.len(),
            std::sync::atomic::Ordering::Relaxed,
        );

        Ok(())
    }

    /// Clear the collection cache
    fn clear_cache(&mut self) {
        self.collection_cache.clear();
    }

    /// Get collection statistics
    fn get_stats(&self) -> SymbolCollectionStats {
        self.collection_stats.clone()
    }

    /// Garbage collect the collection cache
    fn garbage_collect(&mut self) {
        // Remove entries with low access frequency
        // For now, just clear if too large
        if self.collection_cache.len() > 100 {
            self.collection_cache.clear();
        }
    }
}

/// Statistics for symbol collection operations
#[derive(Debug, Default)]
struct SymbolCollectionStats {
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub symbols_collected: usize,
    pub collection_time_ms: u64,
    pub total_symbols_processed: std::sync::atomic::AtomicUsize,
}

impl Clone for SymbolCollectionStats {
    fn clone(&self) -> Self {
        Self {
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            symbols_collected: self.symbols_collected,
            collection_time_ms: self.collection_time_ms,
            total_symbols_processed: std::sync::atomic::AtomicUsize::new(
                self.total_symbols_processed
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }
}

/// Conflict resolver for intelligent namespace management
struct ConflictResolver {
    strategy: ConflictResolutionStrategy,
    existing_symbols: HashMap<String, SymbolOrigin>,
    existing_types: HashMap<String, TypeOrigin>,
}

/// Origin information for tracking symbol sources
#[derive(Debug, Clone)]
struct SymbolOrigin {
    value: RuntimeValue,
    module: ModulePath,
    is_qualified: bool,
}

/// Origin information for tracking type sources
#[derive(Debug, Clone)]
struct TypeOrigin {
    type_def: StructDefinition,
    module: ModulePath,
    is_qualified: bool,
}

impl ConflictResolver {
    fn new(strategy: ConflictResolutionStrategy) -> Self {
        Self {
            strategy,
            existing_symbols: HashMap::new(),
            existing_types: HashMap::new(),
        }
    }

    fn set_strategy(&mut self, strategy: ConflictResolutionStrategy) {
        self.strategy = strategy;
    }

    /// Register existing symbols from the environment to detect conflicts
    fn register_existing_symbols(
        &mut self,
        globals: &HashMap<String, RuntimeValue>,
        types: &HashMap<String, StructDefinition>,
    ) {
        // Register existing global symbols
        for (name, value) in globals {
            self.existing_symbols.insert(
                name.clone(),
                SymbolOrigin {
                    value: value.clone(),
                    module: ModulePath::from_string("global"),
                    is_qualified: false,
                },
            );
        }

        // Register existing types
        for (name, type_def) in types {
            self.existing_types.insert(
                name.clone(),
                TypeOrigin {
                    type_def: type_def.clone(),
                    module: ModulePath::from_string("global"),
                    is_qualified: false,
                },
            );
        }
    }

    fn resolve_conflicts(
        &mut self,
        incoming_symbols: AccessibleSymbols,
        context: &ImportContext,
    ) -> Result<ConflictResult, ImportError> {
        let mut resolved_symbols = AccessibleSymbols::new();
        let mut conflicts = Vec::new();
        let mut warnings = Vec::new();

        // Detect and resolve value symbol conflicts
        for (name, incoming_value) in incoming_symbols.values {
            match self.resolve_value_conflict(&name, incoming_value, &context.target_module) {
                ConflictDecision::Accept(final_name, final_value) => {
                    resolved_symbols
                        .values
                        .insert(final_name.clone(), final_value.clone());
                    // Update existing symbols registry
                    let is_qualified = name != final_name;
                    self.existing_symbols.insert(
                        final_name,
                        SymbolOrigin {
                            value: final_value,
                            module: context.target_module.clone(),
                            is_qualified,
                        },
                    );
                }
                ConflictDecision::Reject(reason) => {
                    conflicts.push(ImportConflict {
                        symbol_name: name.clone(),
                        existing_module: self
                            .existing_symbols
                            .get(&name)
                            .map(|o| o.module.clone())
                            .unwrap_or_else(|| ModulePath::from_string("unknown")),
                        imported_module: context.target_module.clone(),
                        conflict_type: reason,
                    });
                }
                ConflictDecision::Warn(final_name, final_value, warning_msg) => {
                    resolved_symbols
                        .values
                        .insert(final_name.clone(), final_value.clone());
                    warnings.push(ImportWarning {
                        message: warning_msg,
                        symbol_name: Some(name.clone()),
                        module_path: Some(context.target_module.clone()),
                    });
                    // Update existing symbols registry
                    let is_qualified = name != final_name;
                    self.existing_symbols.insert(
                        final_name,
                        SymbolOrigin {
                            value: final_value,
                            module: context.target_module.clone(),
                            is_qualified,
                        },
                    );
                }
            }
        }

        // Detect and resolve type symbol conflicts
        for (name, incoming_type) in incoming_symbols.types {
            match self.resolve_type_conflict(&name, incoming_type, &context.target_module) {
                ConflictDecision::Accept(final_name, final_type) => {
                    resolved_symbols
                        .types
                        .insert(final_name.clone(), final_type.clone());
                    // Update existing types registry
                    let is_qualified = name != final_name;
                    self.existing_types.insert(
                        final_name,
                        TypeOrigin {
                            type_def: final_type,
                            module: context.target_module.clone(),
                            is_qualified,
                        },
                    );
                }
                ConflictDecision::Reject(reason) => {
                    conflicts.push(ImportConflict {
                        symbol_name: name.clone(),
                        existing_module: self
                            .existing_types
                            .get(&name)
                            .map(|o| o.module.clone())
                            .unwrap_or_else(|| ModulePath::from_string("unknown")),
                        imported_module: context.target_module.clone(),
                        conflict_type: reason,
                    });
                }
                ConflictDecision::Warn(final_name, final_type, warning_msg) => {
                    resolved_symbols
                        .types
                        .insert(final_name.clone(), final_type.clone());
                    warnings.push(ImportWarning {
                        message: warning_msg,
                        symbol_name: Some(name.clone()),
                        module_path: Some(context.target_module.clone()),
                    });
                    // Update existing types registry
                    let is_qualified = name != final_name;
                    self.existing_types.insert(
                        final_name,
                        TypeOrigin {
                            type_def: final_type,
                            module: context.target_module.clone(),
                            is_qualified,
                        },
                    );
                }
            }
        }

        // Check for cross-category conflicts (value vs type)
        self.detect_cross_category_conflicts(
            &mut conflicts,
            &resolved_symbols,
            &context.target_module,
        );

        Ok(ConflictResult {
            resolved_symbols,
            conflicts,
            warnings,
        })
    }

    /// Resolve conflicts for value symbols
    fn resolve_value_conflict(
        &self,
        name: &str,
        incoming_value: RuntimeValue,
        importing_module: &ModulePath,
    ) -> ConflictDecision<RuntimeValue> {
        if let Some(existing_origin) = self.existing_symbols.get(name) {
            // Found a conflict with existing symbol
            match self.strategy {
                ConflictResolutionStrategy::Error => {
                    ConflictDecision::Reject(ConflictType::ValueValue)
                }
                ConflictResolutionStrategy::PreferExisting => ConflictDecision::Warn(
                    name.to_string(),
                    existing_origin.value.clone(),
                    format!(
                        "Symbol '{}' already exists, keeping existing from module '{}'",
                        name,
                        existing_origin.module.to_string()
                    ),
                ),
                ConflictResolutionStrategy::PreferImported => ConflictDecision::Warn(
                    name.to_string(),
                    incoming_value,
                    format!(
                        "Symbol '{}' already exists, replacing with import from module '{}'",
                        name,
                        importing_module.to_string()
                    ),
                ),
                ConflictResolutionStrategy::Qualified => {
                    let qualified_name = format!("{}::{}", importing_module.to_string(), name);
                    ConflictDecision::Accept(qualified_name, incoming_value)
                }
            }
        } else {
            // No conflict, accept as-is
            ConflictDecision::Accept(name.to_string(), incoming_value)
        }
    }

    /// Resolve conflicts for type symbols
    fn resolve_type_conflict(
        &self,
        name: &str,
        incoming_type: StructDefinition,
        importing_module: &ModulePath,
    ) -> ConflictDecision<StructDefinition> {
        if let Some(existing_origin) = self.existing_types.get(name) {
            // Found a conflict with existing type
            match self.strategy {
                ConflictResolutionStrategy::Error => {
                    ConflictDecision::Reject(ConflictType::TypeType)
                }
                ConflictResolutionStrategy::PreferExisting => ConflictDecision::Warn(
                    name.to_string(),
                    existing_origin.type_def.clone(),
                    format!(
                        "Type '{}' already exists, keeping existing from module '{}'",
                        name,
                        existing_origin.module.to_string()
                    ),
                ),
                ConflictResolutionStrategy::PreferImported => ConflictDecision::Warn(
                    name.to_string(),
                    incoming_type,
                    format!(
                        "Type '{}' already exists, replacing with import from module '{}'",
                        name,
                        importing_module.to_string()
                    ),
                ),
                ConflictResolutionStrategy::Qualified => {
                    let qualified_name = format!("{}::{}", importing_module.to_string(), name);
                    ConflictDecision::Accept(qualified_name, incoming_type)
                }
            }
        } else {
            // No conflict, accept as-is
            ConflictDecision::Accept(name.to_string(), incoming_type)
        }
    }

    /// Detect conflicts between values and types (cross-category conflicts)
    fn detect_cross_category_conflicts(
        &self,
        conflicts: &mut Vec<ImportConflict>,
        resolved_symbols: &AccessibleSymbols,
        importing_module: &ModulePath,
    ) {
        // Check if any resolved values conflict with existing types
        for value_name in resolved_symbols.values.keys() {
            if let Some(existing_type_origin) = self.existing_types.get(value_name) {
                conflicts.push(ImportConflict {
                    symbol_name: value_name.clone(),
                    existing_module: existing_type_origin.module.clone(),
                    imported_module: importing_module.clone(),
                    conflict_type: ConflictType::ValueType,
                });
            }
        }

        // Check if any resolved types conflict with existing values
        for type_name in resolved_symbols.types.keys() {
            if let Some(existing_value_origin) = self.existing_symbols.get(type_name) {
                conflicts.push(ImportConflict {
                    symbol_name: type_name.clone(),
                    existing_module: existing_value_origin.module.clone(),
                    imported_module: importing_module.clone(),
                    conflict_type: ConflictType::TypeValue,
                });
            }
        }
    }

    /// Clear the conflict resolver state
    fn clear(&mut self) {
        self.existing_symbols.clear();
        self.existing_types.clear();
    }

    /// Get statistics about the conflict resolver state
    fn get_stats(&self) -> ConflictResolverStats {
        ConflictResolverStats {
            tracked_symbols: self.existing_symbols.len(),
            tracked_types: self.existing_types.len(),
            qualified_symbols: self
                .existing_symbols
                .values()
                .filter(|o| o.is_qualified)
                .count(),
            qualified_types: self
                .existing_types
                .values()
                .filter(|o| o.is_qualified)
                .count(),
        }
    }
}

/// Decision for resolving a symbol conflict
enum ConflictDecision<T> {
    /// Accept the symbol with the given name
    Accept(String, T),
    /// Reject the symbol with the given conflict type
    Reject(ConflictType),
    /// Accept with warning message
    Warn(String, T, String),
}

/// Statistics about the conflict resolver
#[derive(Debug, Clone)]
pub struct ConflictResolverStats {
    pub tracked_symbols: usize,
    pub tracked_types: usize,
    pub qualified_symbols: usize,
    pub qualified_types: usize,
}

/// Result of conflict resolution
struct ConflictResult {
    resolved_symbols: AccessibleSymbols,
    conflicts: Vec<ImportConflict>,
    warnings: Vec<ImportWarning>,
}

/// Import cache for performance optimization
struct ImportCache {
    cache: HashMap<CacheKey, AccessibleSymbols>,
    access_counts: HashMap<CacheKey, usize>,
    max_size: usize,
}

impl ImportCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            access_counts: HashMap::new(),
            max_size: 1000, // Configurable cache size
        }
    }

    fn get(&mut self, key: &CacheKey) -> Option<&AccessibleSymbols> {
        if let Some(symbols) = self.cache.get(key) {
            *self.access_counts.entry(key.clone()).or_insert(0) += 1;
            Some(symbols)
        } else {
            None
        }
    }

    fn insert(&mut self, key: CacheKey, symbols: AccessibleSymbols) {
        // Evict if cache is full
        if self.cache.len() >= self.max_size {
            self.evict_lru();
        }

        self.cache.insert(key.clone(), symbols);
        self.access_counts.insert(key, 1);
    }

    fn evict_lru(&mut self) {
        if let Some((lru_key, _)) = self.access_counts.iter().min_by_key(|(_, &count)| count) {
            let lru_key = lru_key.clone();
            self.cache.remove(&lru_key);
            self.access_counts.remove(&lru_key);
        }
    }

    fn clear(&mut self) {
        self.cache.clear();
        self.access_counts.clear();
    }

    fn garbage_collect(&mut self) {
        // Remove entries with very low access counts
        let threshold = 2;
        let to_remove: Vec<_> = self
            .access_counts
            .iter()
            .filter(|(_, &count)| count < threshold)
            .map(|(key, _)| key.clone())
            .collect();

        for key in to_remove {
            self.cache.remove(&key);
            self.access_counts.remove(&key);
        }
    }

    fn get_stats(&self) -> CacheStats {
        CacheStats {
            entries: self.cache.len(),
            max_size: self.max_size,
            total_accesses: self.access_counts.values().sum(),
        }
    }
}

/// Cache key for import results
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct CacheKey {
    target_module: ModulePath,
    importing_module: ModulePath,
    import_type: ImportType,
}

impl CacheKey {
    fn new(
        target_module: ModulePath,
        importing_module: ModulePath,
        import_type: ImportType,
    ) -> Self {
        Self {
            target_module,
            importing_module,
            import_type,
        }
    }
}

/// Statistics about import operations
#[derive(Debug, Clone)]
pub struct ImportStats {
    pub dependency_stats: super::dependency::DependencyStats,
    pub registry_stats: super::registry::RegistryStats,
    pub cache_stats: CacheStats,
    pub conflict_stats: ConflictResolverStats,
    pub collection_stats: SymbolCollectionStats,
}

/// Statistics about the import cache
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub max_size: usize,
    pub total_accesses: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::symbol_table::InternedSymbol;

    use crate::interpreter::parser::ast::{Conjunction, Goal as AstGoal, Location};
    use crate::interpreter::parser::ast::{PredicateDefinition, PredicateKind, Visibility};

    fn create_test_module_info() -> ModuleInfo {
        let mut module_info = ModuleInfo::new(std::path::PathBuf::from("test.pv"));

        let predicate = PredicateDefinition {
            attributes: vec![],
            visibility: Visibility::Public,
            predicate_kind: PredicateKind::Relation,
            name: "example_predicate".to_string().into(),
            parameters: vec![],
            search_strategy: None,
            body: vec![AstGoal::Conjunction(
                Conjunction {
                    body: vec![],
                    params: None,
                },
                Location::dummy(),
            )],
            span: Location::dummy(),
        };

        module_info.public_symbols.insert(
            "example_symbol".to_string(),
            RuntimeValue::Relation(predicate),
        );

        module_info
    }

    #[test]
    fn test_glob_import_basic() {
        let mut resolver = ImportResolver::new();
        let mut loaded_modules = HashMap::new();
        let globals = HashMap::new();
        let types = HashMap::new();

        loaded_modules.insert("test_module".to_string(), create_test_module_info());

        let target_path = QualifiedPath::Absolute(vec![InternedSymbol::from_text("test_module")]);
        let importing_module = ModulePath::from_string("importing_module");

        let result = resolver.import_glob_enhanced(
            &target_path,
            importing_module,
            &loaded_modules,
            &globals,
            &types,
        );

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
        let import_result = result.unwrap();
        assert!(!import_result.imported_symbols.values.is_empty());
        assert!(import_result
            .imported_symbols
            .values
            .contains_key("example_symbol"));
    }

    #[test]
    fn test_selective_import() {
        let mut resolver = ImportResolver::new();
        let mut loaded_modules = HashMap::new();
        let globals = HashMap::new();
        let types = HashMap::new();

        loaded_modules.insert("test_module".to_string(), create_test_module_info());

        let target_path = QualifiedPath::Absolute(vec![InternedSymbol::from_text("test_module")]);
        let importing_module = ModulePath::from_string("importing_module");
        let requested_symbols = vec![("example_symbol".to_string(), None)];

        let result = resolver.import_selective(
            &target_path,
            &requested_symbols,
            importing_module,
            &loaded_modules,
            &globals,
            &types,
        );

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
        let import_result = result.unwrap();
        assert_eq!(import_result.imported_symbols.values.len(), 1);
        assert!(import_result
            .imported_symbols
            .values
            .contains_key("example_symbol"));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut resolver = ImportResolver::new();
        let mut loaded_modules = HashMap::new();
        let globals = HashMap::new();
        let types = HashMap::new();

        loaded_modules.insert("module_a".to_string(), create_test_module_info());
        loaded_modules.insert("module_b".to_string(), create_test_module_info());

        let module_a = ModulePath::from_string("module_a");
        let module_b = ModulePath::from_string("module_b");

        // Add A -> B dependency
        resolver.dependency_tracker.add_import_edge(
            module_a.clone(),
            module_b.clone(),
            ImportType::Glob,
        );

        // Try to import A from B (should detect cycle)
        let target_path = QualifiedPath::Absolute(vec![InternedSymbol::from_text("module_a")]);
        let result = resolver.import_glob_enhanced(
            &target_path,
            module_b,
            &loaded_modules,
            &globals,
            &types,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ImportError::CircularDependency(_) => {
                // Expected
            }
            other => panic!("Expected circular dependency error, got {:?}", other),
        }
    }

    #[test]
    fn test_import_cache() {
        let mut resolver = ImportResolver::new();
        let mut loaded_modules = HashMap::new();
        let globals = HashMap::new();
        let types = HashMap::new();

        loaded_modules.insert("test_module".to_string(), create_test_module_info());

        let target_path = QualifiedPath::Absolute(vec![InternedSymbol::from_text("test_module")]);
        let importing_module = ModulePath::from_string("importing_module");

        // First import (cache miss)
        let result1 = resolver
            .import_glob_enhanced(
                &target_path,
                importing_module.clone(),
                &loaded_modules,
                &globals,
                &types,
            )
            .expect("First import should succeed");
        assert_eq!(result1.metrics.cache_misses, 1);
        assert_eq!(result1.metrics.cache_hits, 0);

        // Second import (cache hit)
        let result2 = resolver
            .import_glob_enhanced(
                &target_path,
                importing_module,
                &loaded_modules,
                &globals,
                &types,
            )
            .expect("Second import should succeed");
        assert_eq!(result2.metrics.cache_hits, 1);
        assert_eq!(result2.metrics.cache_misses, 0);
    }
}
