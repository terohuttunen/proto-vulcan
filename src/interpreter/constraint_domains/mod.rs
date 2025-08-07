//! Constraint Domains Plugin Architecture
//!
//! This module provides a flexible system where constraint domains can register
//! their own parsers and converters. Each domain (like clpfd, clpr, clpb) can
//! define its own syntax and semantics.

use super::parser::ast::ConstraintBody;
use super::InterpreterError;
use crate::goal::Goal;
use std::collections::HashMap;
use std::rc::Rc;

pub mod clpfd;
pub mod clpz;

/// Helper function to map constraint parsing errors from constraint body coordinates
/// to source file coordinates using the constraint body span.
pub fn map_constraint_error_position(
    pest_error: &pest::error::Error<impl pest::RuleType>,
    constraint_body: &ConstraintBody,
) -> String {
    match pest_error.location {
        pest::error::InputLocation::Pos(body_pos) => {
            // Map position from constraint body to source file coordinates
            let source_pos = constraint_body.span.start + body_pos;
            format!(
                "Parse error at position {} (source position {}): {}",
                body_pos, source_pos, pest_error.variant
            )
        }
        pest::error::InputLocation::Span((start, end)) => {
            // Map span from constraint body to source file coordinates
            let source_start = constraint_body.span.start + start;
            let source_end = constraint_body.span.start + end;
            format!(
                "Parse error at positions {}-{} (source positions {}-{}): {}",
                start, end, source_start, source_end, pest_error.variant
            )
        }
    }
}

/// Variable information for constraint template compilation
#[derive(Debug, Clone)]
pub struct VariableInfo {
    /// Variable name
    pub name: String,
    /// Variable type (relational or meta)
    pub var_type: VariableType,
}

/// Type of variable for constraint compilation
#[derive(Debug, Clone, PartialEq)]
pub enum VariableType {
    /// Relational variable for logic computation
    Relational,
    /// Meta variable for compile-time computation
    Meta,
}

/// Trait for compiled constraint templates that execute with execution context
pub trait DomainConstraintTemplate: std::fmt::Debug {
    /// Get the list of required variables for this template
    fn required_variables(&self) -> &[String];
    
    /// Execute the template with execution context to produce a Goal
    fn execute(
        &self,
        execution_context: &mut crate::interpreter::runtime::context::ExecutionContext,
        variables: &std::collections::HashMap<String, VariableInfo>,
    ) -> Result<Goal, InterpreterError>;
}

/// Represents a resolved variable value for constraint template execution
#[derive(Debug, Clone)]
pub enum ResolvedValue {
    /// Relational variable resolved to LTerm
    Relational(crate::lterm::LTerm),
    /// Meta variable resolved to runtime value
    Meta(crate::interpreter::compiler::ir::MetaValue),
}

/// Trait that all constraint domains must implement
pub trait ConstraintDomain {
    /// Domain name (e.g., "clpfd", "clpr", "clpb")
    fn name(&self) -> &str;

    /// Get description of supported syntax for error messages
    fn syntax_help(&self) -> &str;

    /// Compile constraint block with variable binding validation
    fn compile(
        &self,
        body: &ConstraintBody,
        binder: &dyn Fn(&str) -> Option<VariableInfo>,
    ) -> Result<Rc<dyn DomainConstraintTemplate>, InterpreterError>;
}

/// Registry for constraint domains
pub struct ConstraintDomainRegistry {
    domains: HashMap<String, Box<dyn ConstraintDomain>>,
}

impl ConstraintDomainRegistry {
    pub fn new() -> Self {
        Self {
            domains: HashMap::new(),
        }
    }

    pub fn register(&mut self, domain: Box<dyn ConstraintDomain>) {
        self.domains.insert(domain.name().to_string(), domain);
    }

    pub fn get_domain(&self, name: &str) -> Option<&dyn ConstraintDomain> {
        self.domains.get(name).map(|d| d.as_ref())
    }

    pub fn list_domains(&self) -> Vec<&str> {
        self.domains.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ConstraintDomainRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(clpfd::ClpfdDomain::new()));
        registry.register(Box::new(clpz::ClpzDomain::new()));
        registry
    }
}
