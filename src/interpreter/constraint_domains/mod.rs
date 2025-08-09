//! Constraint Domains Plugin Architecture
//!
//! This module provides a flexible system where constraint domains can register
//! their own parsers and converters. Each domain (like clpfd, clpr, clpb) can
//! define its own syntax and semantics.

use super::parser::ast::ConstraintBody;
use super::InterpreterError;
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::interpreter::metaprogramming::MetaValue;
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

/// Runtime value that can be bound to constraint variables
#[derive(Debug, Clone)]
pub enum RuntimeValue {
    /// Relational variable (LTerm)
    Relational(LTerm),
    /// Meta variable (compile-time value)
    Meta(MetaValue),
}

/// Context stack for managing nested fresh variable scopes
#[derive(Debug, Clone)]
pub struct FreshVariableContext {
    /// Stack of fresh variable scopes (innermost first)
    scope_stack: Vec<FreshScope>,
}

#[derive(Debug, Clone)]
pub struct FreshScope {
    /// All fresh variables in this scope (shared namespace)
    variables: HashMap<String, RuntimeValue>,
}

impl FreshVariableContext {
    pub fn new() -> Self {
        Self {
            scope_stack: Vec::new(),
        }
    }
    
    /// Push a fresh variable scope with both relational and meta variables
    pub fn push_scope(
        &mut self, 
        relational_vars: &[String],
        meta_vars: &[(String, MetaValue)],
    ) -> (Vec<LTerm>, Vec<(String, MetaValue)>) {
        let mut scope = FreshScope {
            variables: HashMap::new(),
        };
        
        // Create relational fresh variables
        let mut rel_lterms = Vec::new();
        for var_name in relational_vars {
            let fresh_var = LTerm::var(var_name);
            scope.variables.insert(var_name.clone(), RuntimeValue::Relational(fresh_var.clone()));
            rel_lterms.push(fresh_var);
        }
        
        // Store meta fresh variables
        let mut meta_pairs = Vec::new();
        for (var_name, meta_value) in meta_vars {
            scope.variables.insert(var_name.clone(), RuntimeValue::Meta(meta_value.clone()));
            meta_pairs.push((var_name.clone(), meta_value.clone()));
        }
        
        self.scope_stack.push(scope);
        (rel_lterms, meta_pairs)
    }
    
    /// Pop the most recent fresh variable scope
    pub fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }
    
    /// Resolve variable through the context stack (innermost to outermost)
    pub fn resolve_fresh_variable(&self, var_name: &str) -> Option<RuntimeValue> {
        // Search from innermost scope to outermost (proper shadowing)
        for scope in self.scope_stack.iter().rev() {
            if let Some(runtime_value) = scope.variables.get(var_name) {
                return Some(runtime_value.clone());
            }
        }
        None
    }
    
    /// Check if a variable is bound in any fresh scope
    pub fn has_fresh_variable(&self, var_name: &str) -> bool {
        self.scope_stack.iter().any(|scope| 
            scope.variables.contains_key(var_name)
        )
    }
}

/// Trait for compiled constraint templates that execute with context stack
pub trait DomainConstraintTemplate: std::fmt::Debug {
    /// Execute template with external binder and fresh variable context
    fn to_goal_with_context(
        &self,
        external_binder: &dyn Fn(&str) -> Option<RuntimeValue>,
        fresh_context: &mut FreshVariableContext,
    ) -> Result<Goal, InterpreterError>;
    
    /// Get external relational variables referenced by this template
    fn external_relational_variables(&self) -> &[String];
    
    /// Get external meta variables referenced by this template
    fn external_meta_variables(&self) -> &[String];
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

    /// Compile constraint block with variable type validation
    fn compile(
        &self,
        body: &ConstraintBody,
        binder: &dyn Fn(&str) -> Option<VariableType>,
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
