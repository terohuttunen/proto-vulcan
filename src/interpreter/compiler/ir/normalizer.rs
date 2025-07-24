//! Variable normalization for structural hashing
//!
//! This module provides functionality to normalize goals by converting variable names
//! to canonical representations, enabling structural comparison that ignores variable
//! names while preserving their binding relationships.
//!
//! ## How Normalization Works
//!
//! Variable normalization is a key component of structural hashing that allows us to
//! recognize when two goals are "alpha-equivalent" - structurally identical except for
//! variable naming.
//!
//! ### The Problem
//!
//! Consider these two goals:
//! ```prolog
//! |x| { x == 42 }
//! |y| { y == 42 }
//! ```
//!
//! These goals are logically identical - they both introduce a fresh variable and
//! unify it with 42. However, their variable names differ (x vs y), so a naive
//! structural comparison would consider them different.
//!
//! ### The Solution: Variable Normalization
//!
//! The normalizer converts variable names to canonical identifiers based on their
//! binding relationships:
//!
//! 1. **Scoped Variables**: Variables bound by fresh/let constructs get normalized
//!    to their position within that scope: `BoundVariable { scope_level, local_id }`
//!
//! 2. **Free Variables**: Variables not bound in any scope get normalized to
//!    global IDs in order of first appearance: `FreeVariable(global_id)`
//!
//! ### Example Normalization
//!
//! ```prolog
//! Original: |x, y| { x == y, z == 42 }
//! Normalized: Fresh { 
//!   var_count: 2, 
//!   body: [
//!     Equality(BoundVariable{scope_level: 0, local_id: 0}, 
//!              BoundVariable{scope_level: 0, local_id: 1}),
//!     Equality(FreeVariable(0), Literal(Integer(42)))
//!   ]
//! }
//! ```
//!
//! Both `|x, y| { x == y, z == 42 }` and `|a, b| { a == b, z == 42 }` would
//! normalize to the same representation, enabling structural equivalence detection.
//!
//! ### Scope Handling
//!
//! The normalizer maintains a stack of scopes to handle nested variable bindings:
//!
//! ```prolog
//! |x| {           // Scope 0: x -> local_id 0
//!   |y| {         // Scope 1: y -> local_id 0  
//!     x == y      // x: BoundVariable{scope_level: 0, local_id: 0}
//!                 // y: BoundVariable{scope_level: 1, local_id: 0}
//!   }
//! }
//! ```
//!
//! This preserves the binding structure while making variable names canonical.
//!
//! ### Benefits
//!
//! 1. **Structural Equivalence**: Recognize logically identical goals with different variable names
//! 2. **Efficient Caching**: Cache proofs by structural hash rather than exact goal representation
//! 3. **Pattern Recognition**: Identify recurring goal patterns across different contexts
//! 4. **Memory Optimization**: Share structural representations via content-addressable storage

use super::{Goal, Term, Literal};
use crate::interpreter::symbol_table::InternedSymbol;
use im_rc::HashMap;

/// Normalizer for converting goals to variable-normalized form for structural hashing
pub struct VariableNormalizer {
    // Stack of scopes for proper variable binding handling
    scope_stack: Vec<HashMap<InternedSymbol, u32>>,
    next_global_id: u32,
}

impl VariableNormalizer {
    pub fn new() -> Self {
        Self {
            scope_stack: vec![HashMap::new()], // Global scope
            next_global_id: 0,
        }
    }
    
    pub fn normalize_goal(&mut self, goal: &Goal) -> NormalizedGoal {
        match goal {
            Goal::Equality(left, right) => {
                NormalizedGoal::Equality(
                    self.normalize_term(left),
                    self.normalize_term(right)
                )
            }
            Goal::Disequality(left, right) => {
                NormalizedGoal::Disequality(
                    self.normalize_term(left),
                    self.normalize_term(right)
                )
            }
            Goal::Fresh(fresh) => {
                // Create new scope for fresh variables
                let mut local_scope = HashMap::new();
                for (i, var) in fresh.variables.iter().enumerate() {
                    local_scope.insert(var.clone(), i as u32);
                }
                self.scope_stack.push(local_scope);
                
                let normalized_body: Vec<_> = fresh.body
                    .iter()
                    .map(|goal| self.normalize_goal(goal))
                    .collect();
                
                self.scope_stack.pop();
                
                NormalizedGoal::Fresh {
                    var_count: fresh.variables.len() as u32,
                    body: normalized_body,
                }
            }
            Goal::Conjunction(goals) => {
                NormalizedGoal::Conjunction(
                    goals.iter()
                        .map(|goal| self.normalize_goal(goal))
                        .collect()
                )
            }
            Goal::Disjunction(goals) => {
                NormalizedGoal::Disjunction(
                    goals.iter()
                        .map(|goal| self.normalize_goal(goal))
                        .collect()
                )
            }
            Goal::Boolean(b) => NormalizedGoal::Boolean(*b),
            
            // For now, other goal types use simplified normalization
            _ => NormalizedGoal::Other(format!("{:?}", goal)),
        }
    }
    
    fn normalize_term(&mut self, term: &Term) -> NormalizedTerm {
        match term {
            Term::Variable(var) => {
                // Look up variable in scope stack (innermost first)
                for (scope_level, scope) in self.scope_stack.iter().enumerate().rev() {
                    if let Some(&local_id) = scope.get(var) {
                        return NormalizedTerm::BoundVariable {
                            scope_level: scope_level as u32,
                            local_id,
                        };
                    }
                }
                
                // Free variable - assign global ID
                let global_id = if let Some(&existing_id) = self.scope_stack[0].get(var) {
                    existing_id
                } else {
                    let id = self.next_global_id;
                    self.next_global_id += 1;
                    self.scope_stack[0].insert(var.clone(), id);
                    id
                };
                
                NormalizedTerm::FreeVariable(global_id)
            }
            Term::Wildcard => NormalizedTerm::Wildcard,
            Term::Literal(literal) => NormalizedTerm::Literal(literal.clone()),
            Term::List(list) => {
                NormalizedTerm::List {
                    elements: list.elements.iter()
                        .map(|t| self.normalize_term(t))
                        .collect(),
                    tail: list.tail.as_ref().map(|t| Box::new(self.normalize_term(t))),
                }
            }
            // For now, other term types use simplified normalization
            _ => NormalizedTerm::Other(format!("{:?}", term)),
        }
    }
}

/// Normalized goal representation for structural hashing
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum NormalizedGoal {
    Equality(NormalizedTerm, NormalizedTerm),
    Disequality(NormalizedTerm, NormalizedTerm),
    Fresh {
        var_count: u32,
        body: Vec<NormalizedGoal>,
    },
    Conjunction(Vec<NormalizedGoal>),
    Disjunction(Vec<NormalizedGoal>),
    Boolean(bool),
    Other(String), // Simplified representation for other types
}

/// Normalized term representation for structural hashing
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum NormalizedTerm {
    BoundVariable {
        scope_level: u32,  // Which scope level (0 = outermost)
        local_id: u32,     // ID within that scope
    },
    FreeVariable(u32),     // Global free variable ID
    Wildcard,
    Literal(Literal),
    List {
        elements: Vec<NormalizedTerm>,
        tail: Option<Box<NormalizedTerm>>,
    },
    Other(String), // Simplified representation for other types
}

