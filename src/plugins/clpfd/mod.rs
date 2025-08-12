//! CLPFD Plugin - Constraint Logic Programming over Finite Domains
//!
//! This plugin provides finite domain constraint processing including:
//! - Substitution extension processing for finite domain variables
//! - Constraint block compilation for CLPFD syntax

use crate::state::{ConstraintPlugin, SMap, SResult, State};
use crate::interpreter::constraint_domains::ConstraintCompiler;

/// CLPFD Plugin - provides finite domain constraint processing
#[derive(Debug)]
pub struct ClpfdPlugin;

impl ClpfdPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl ConstraintPlugin for ClpfdPlugin {
    fn name(&self) -> &str {
        "clpfd"
    }
    
    fn process_extension(&self, state: State, _extension: &SMap) -> SResult {
        // For now, CLPFD plugin doesn't need custom extension processing
        // The finite domain processing is handled in State::process_extension_fd
        Ok(state)
    }
    
    fn constraint_compiler(&self) -> Option<Box<dyn ConstraintCompiler>> {
        Some(Box::new(crate::interpreter::constraint_domains::clpfd::ClpfdCompiler::new()))
    }
}

impl Default for ClpfdPlugin {
    fn default() -> Self {
        Self::new()
    }
}