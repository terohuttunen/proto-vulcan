//! Constraint Domains Plugin Architecture
//!
//! This module provides a flexible system where constraint domains can register
//! their own parsers and converters. Each domain (like clpfd, clpr, clpb) can
//! define its own syntax and semantics.

use super::execution::ExecutionContext;
use super::parser::ast::ConstraintBody;
use super::InterpreterError;
use crate::engine::Engine;
use crate::goal::Goal;
use crate::user::User;
use std::collections::HashMap;

pub mod clpfd;
pub mod clpz;

/// Trait that all constraint domains must implement
pub trait ConstraintDomain<U: User, E: Engine<U>> {
    /// Domain name (e.g., "clpfd", "clpr", "clpb")
    fn name(&self) -> &str;

    /// Parse raw constraint body into domain-specific representation
    fn parse_constraints(
        &self,
        body: &ConstraintBody,
    ) -> Result<Box<dyn DomainConstraints<U, E>>, InterpreterError>;

    /// Get description of supported syntax for error messages
    fn syntax_help(&self) -> &str;
}

/// Trait for parsed domain-specific constraints
pub trait DomainConstraints<U: User, E: Engine<U>> {
    /// Convert parsed constraints to runtime goals
    fn convert_to_goals(
        &self,
        execution_context: &mut ExecutionContext<U, E>,
    ) -> Result<Goal<U, E>, InterpreterError>;

    /// Extract variable names for query processing
    fn extract_variables(&self) -> Vec<String>;
}

/// Registry for constraint domains
pub struct ConstraintDomainRegistry<U: User, E: Engine<U>> {
    domains: HashMap<String, Box<dyn ConstraintDomain<U, E>>>,
}

impl<U: User, E: Engine<U>> ConstraintDomainRegistry<U, E> {
    pub fn new() -> Self {
        Self {
            domains: HashMap::new(),
        }
    }

    pub fn register(&mut self, domain: Box<dyn ConstraintDomain<U, E>>) {
        self.domains.insert(domain.name().to_string(), domain);
    }

    pub fn get_domain(&self, name: &str) -> Option<&dyn ConstraintDomain<U, E>> {
        self.domains.get(name).map(|d| d.as_ref())
    }

    pub fn list_domains(&self) -> Vec<&str> {
        self.domains.keys().map(|s| s.as_str()).collect()
    }

    pub fn convert_constraint_block(
        &self,
        execution_context: &mut ExecutionContext<U, E>,
        domain_name: &str,
        body: &ConstraintBody,
    ) -> Result<Goal<U, E>, InterpreterError> {
        let domain = self.get_domain(domain_name).ok_or_else(|| {
            InterpreterError::InvalidConstraintSyntax {
                domain: domain_name.to_string(),
                error: format!("Unknown constraint domain: {}", domain_name),
            }
        })?;

        let parsed_constraints = domain.parse_constraints(body)?;
        parsed_constraints.convert_to_goals(execution_context)
    }
}

impl<U: User, E: Engine<U>> Default for ConstraintDomainRegistry<U, E> {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(clpfd::ClpfdDomain::new()));
        registry.register(Box::new(clpz::ClpzDomain::new()));
        registry
    }
}
