//! CLPZ Plugin - Constraint Logic Programming over Integers
//!
//! This plugin provides integer constraint processing including:
//! - Substitution extension processing for integer variables
//! - Constraint block compilation for CLPZ syntax

use crate::state::{ConstraintPlugin, SMap, SResult, State};
use crate::interpreter::constraint_domains::ConstraintCompiler;

/// CLPZ Plugin - provides integer constraint processing
#[derive(Debug)]
pub struct ClpzPlugin;

impl ClpzPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl ConstraintPlugin for ClpzPlugin {
    fn name(&self) -> &str {
        "clpz"
    }
    
    fn process_extension(&self, state: State, _extension: &SMap) -> SResult {
        // For now, CLPZ plugin doesn't need custom extension processing
        Ok(state)
    }
    
    fn constraint_compiler(&self) -> Option<Box<dyn ConstraintCompiler>> {
        Some(Box::new(crate::interpreter::constraint_domains::clpz::ClpzCompiler::new()))
    }
}

impl Default for ClpzPlugin {
    fn default() -> Self {
        Self::new()
    }
}