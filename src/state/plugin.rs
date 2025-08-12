//! ConstraintPlugin System for Proto-Vulcan State
//!
//! This module provides a plugin architecture for extending State behavior,
//! particularly for handling substitution extensions and integrating with
//! interpreter constraint block compilation.

use crate::state::{SMap, SResult, State};
use crate::interpreter::constraint_domains::{ConstraintCompiler, ConstraintCompilerRegistry};
use std::rc::Rc;

/// Trait for constraint plugins that extend State functionality
///
/// ConstraintPlugins provide callbacks for substitution extension processing
/// and can optionally provide constraint block compilers for the interpreter.
pub trait ConstraintPlugin: std::fmt::Debug {
    /// Plugin name/identifier
    fn name(&self) -> &str;
    
    /// Process substitution extension - called when State.smap is extended
    /// 
    /// This is called during unification when new variable bindings are added
    /// to the substitution map. Plugins can use this to maintain consistency
    /// of their own constraint stores or perform domain-specific processing.
    fn process_extension(&self, state: State, extension: &SMap) -> SResult;
    
    /// Get constraint compiler if this plugin provides one
    /// 
    /// This allows plugins to provide constraint block compilation capabilities
    /// for the interpreter's constraint domain DSL parsing.
    fn constraint_compiler(&self) -> Option<Box<dyn ConstraintCompiler>> {
        None
    }
}

/// Registry for constraint plugins
///
/// Manages both extension processing plugins and constraint block compilers,
/// providing a unified interface for State and interpreter integration.
#[derive(Debug)]
pub struct ConstraintPluginRegistry {
    /// Registered constraint plugins
    plugins: Vec<Box<dyn ConstraintPlugin>>,
    /// Constraint compiler registry for interpreter integration
    compiler_registry: ConstraintCompilerRegistry,
}

impl ConstraintPluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            compiler_registry: ConstraintCompilerRegistry::new(),
        }
    }
    
    /// Create a registry with default plugins
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        
        // Register default constraint compilers
        registry.compiler_registry = ConstraintCompilerRegistry::default();
        
        // Register default extension processing plugins
        registry.register(Box::new(crate::plugins::ClpfdPlugin::new()));
        registry.register(Box::new(crate::plugins::ClpzPlugin::new()));
        
        registry
    }
    
    /// Register a constraint plugin
    pub fn register(&mut self, plugin: Box<dyn ConstraintPlugin>) {
        // If plugin provides a constraint compiler, register it in the compiler registry
        if let Some(compiler) = plugin.constraint_compiler() {
            self.compiler_registry.register(compiler);
        }
        
        self.plugins.push(plugin);
    }
    
    /// Process extension through all registered plugins
    ///
    /// Called by State::process_extension to allow all plugins to process
    /// the substitution extension.
    pub fn process_extension(&self, mut state: State, extension: &SMap) -> SResult {
        for plugin in &self.plugins {
            state = plugin.process_extension(state, extension)?;
        }
        Ok(state)
    }
    
    /// Get constraint compiler by name
    ///
    /// Used by the interpreter to access constraint block compilers for
    /// parsing and compiling constraint domain DSL.
    pub fn get_compiler(&self, name: &str) -> Option<&dyn ConstraintCompiler> {
        self.compiler_registry.get_compiler(name)
    }
    
    /// List available constraint compilers
    pub fn list_compilers(&self) -> Vec<&str> {
        self.compiler_registry.list_compilers()
    }
    
    /// Get the number of registered plugins
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

impl Default for ConstraintPluginRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Empty plugin that does no processing - useful for testing
    #[derive(Debug)]
    struct EmptyPlugin;

    impl ConstraintPlugin for EmptyPlugin {
        fn name(&self) -> &str {
            "empty"
        }
        
        fn process_extension(&self, state: State, _extension: &SMap) -> SResult {
            Ok(state)
        }
    }
    
    #[test]
    fn test_empty_plugin() {
        let plugin = EmptyPlugin;
        assert_eq!(plugin.name(), "empty");
        assert!(plugin.constraint_compiler().is_none());
    }
    
    #[test]
    fn test_plugin_registry() {
        let mut registry = ConstraintPluginRegistry::new();
        assert_eq!(registry.plugin_count(), 0);
        
        registry.register(Box::new(EmptyPlugin));
        assert_eq!(registry.plugin_count(), 1);
    }
}