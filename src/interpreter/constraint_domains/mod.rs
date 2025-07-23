//! Constraint Domains Plugin Architecture
//!
//! This module provides a flexible system where constraint domains can register
//! their own parsers and converters. Each domain (like clpfd, clpr, clpb) can
//! define its own syntax and semantics.

use super::execution::ExecutionContext;
use super::parser::ast::ConstraintBody;
use super::InterpreterError;
use crate::engine::{Engine, DefaultEngine};
use crate::goal::Goal;
use crate::user::{User, DefaultUser};
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

/// Non-generic trait for compiled constraint templates that execute to produce Goals
pub trait DomainConstraintTemplate: std::fmt::Debug {
    /// Execute the template with the given execution context to produce a Goal
    fn execute(&self, execution_context: &mut super::execution::ExecutionContext<DefaultUser, DefaultEngine<DefaultUser>>) 
        -> Result<Goal<DefaultUser, DefaultEngine<DefaultUser>>, InterpreterError>;
}

/// IR-specific trait for compiled constraint templates that work with IR ExecutionContext
pub trait IrDomainConstraintTemplate: std::fmt::Debug {
    /// Execute the template with the IR execution context to produce a Goal
    fn execute(&self, execution_context: &mut super::ir::context::ExecutionContext) 
        -> Result<Goal<DefaultUser, DefaultEngine<DefaultUser>>, InterpreterError>;
}

/// Trait that all constraint domains must implement
pub trait ConstraintDomain<U: User, E: Engine<U>> {
    /// Domain name (e.g., "clpfd", "clpr", "clpb")
    fn name(&self) -> &str;

    /// Parse raw constraint body into domain-specific representation
    fn parse_constraints(
        &self,
        body: &ConstraintBody,
        source_span: &super::parser::ast::Span,
    ) -> Result<Box<dyn DomainConstraints<U, E>>, InterpreterError>;

    /// Get description of supported syntax for error messages
    fn syntax_help(&self) -> &str;

    /// Get list of unbound variable names that this constraint block requires
    fn get_unbound_variables(&self, body: &ConstraintBody) -> Result<Vec<String>, InterpreterError>;

    /// Compile constraint block with resolved variable information into a template
    fn compile_template(
        &self,
        body: &ConstraintBody,
        variables: HashMap<String, VariableInfo>,
    ) -> Result<Rc<dyn DomainConstraintTemplate>, InterpreterError>;

    /// Compile constraint block for IR system with resolved variable information into an IR template
    fn compile_ir_template(
        &self,
        body: &ConstraintBody,
        variables: HashMap<String, VariableInfo>,
    ) -> Result<Rc<dyn IrDomainConstraintTemplate>, InterpreterError>;
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
        source_span: &super::parser::ast::Span,
    ) -> Result<Goal<U, E>, InterpreterError> {
        let domain = self.get_domain(domain_name).ok_or_else(|| {
            InterpreterError::InvalidConstraintSyntax {
                domain: domain_name.to_string(),
                error: format!("Unknown constraint domain: {}", domain_name),
            }
        })?;

        let parsed_constraints = domain.parse_constraints(body, source_span)?;
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
