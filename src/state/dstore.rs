//! Dynamic Domain Store for Proto-Vulcan
//!
//! This module implements the dynamic domain store for constraints that allows
//! different domain types to be used with lattice-based operations.

use crate::lterm::LTerm;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;

/// Trait for domain values that can be stored in the dynamic domain store
///
/// This trait provides a minimal, clean interface focused on essential cross-domain operations.
/// It combines lattice-based operations (meet, join, subsumes) with fundamental domain operations
/// that work across all constraint domain types.
///
/// Domain-specific operations (like bounds extraction, set operations, range filtering) are
/// kept as methods on concrete domain types and accessed via downcasting using as_any().
///
/// Supported domain types:
/// - CLP(FD): finite integer domains - implemented by FiniteDomain
/// - CLP(R): real number domains - future extension
/// - CLP(Q): rational number domains - future extension  
/// - CLP(B): boolean domains - future extension
/// - CLP(Set): set constraint domains - future extension
pub trait DomainValue: Debug + Send + Sync + Any {
    // === CORE LATTICE OPERATIONS (Pure mathematical) ===

    /// Meet operation (greatest lower bound) - intersection of constraints
    /// Returns a more restrictive domain that satisfies both sets of constraints
    /// Returns None if constraints are inconsistent (meet would be bottom/empty)
    fn meet(&self, other: &dyn DomainValue) -> Option<Box<dyn DomainValue>>;

    /// Join operation (least upper bound) - union/approximation of constraints
    /// Returns a less restrictive domain that over-approximates both inputs
    /// Used for widening in abstract interpretation to ensure convergence
    fn join(&self, other: &dyn DomainValue) -> Option<Box<dyn DomainValue>>;

    /// Lattice ordering: check if this domain is less restrictive than other
    /// Returns true if this domain subsumes other (this ⊒ other in lattice)
    fn subsumes(&self, other: &dyn DomainValue) -> bool;

    // === ESSENTIAL CROSS-DOMAIN OPERATIONS ===

    /// Get the domain type identifier for this domain value (e.g., "clpfd", "clpr", "clpb")
    fn domain_type(&self) -> &str;

    /// Check if the domain contains a specific LTerm value
    /// This is the most general containment check that works for all domain types
    fn contains_lterm(&self, value: &crate::lterm::LTerm) -> bool;

    /// Check if the domain is empty/bottom (inconsistent constraints)
    fn is_empty(&self) -> bool;

    /// Check if the domain represents a singleton (single value)
    fn is_singleton(&self) -> bool;

    /// Get the singleton value as an LTerm if this domain contains exactly one element
    /// Returns None if the domain is not a singleton
    fn singleton_lterm(&self) -> Option<crate::lterm::LTerm>;

    // === INFRASTRUCTURE OPERATIONS ===

    /// Clone the domain value into a new boxed instance
    fn clone_box(&self) -> Box<dyn DomainValue>;

    /// Cast to Any for downcasting to concrete types
    /// Use this to access domain-specific operations via downcast_ref::<ConcreteType>()
    fn as_any(&self) -> &dyn Any;

    // === MINIMAL NON-LATTICE API (with sensible defaults) ===

    /// Get the size of the domain (number of elements) for finite domains
    /// Returns None for infinite domains (default implementation)
    fn size(&self) -> Option<usize> {
        None // Default: assume infinite domain
    }

    /// Check if two domains are equivalent (same constraints)
    /// Default implementation uses lattice operations: equivalent if both subsume each other
    fn equivalent(&self, other: &dyn DomainValue) -> bool
    where
        Self: Sized,
    {
        self.subsumes(other) && other.subsumes(self)
    }

    /// Get a human-readable representation of the domain for debugging
    /// Default implementation uses Debug trait
    fn display_string(&self) -> String {
        format!("{:?}", self)
    }
}

/// Helper functions for safe downcasting to specific domain types
pub fn as_finite_domain(domain: &dyn DomainValue) -> Option<&crate::state::FiniteDomain> {
    domain.as_any().downcast_ref::<crate::state::FiniteDomain>()
}

/// Extract two finite domains from domain values, returning None if either isn't a finite domain
pub fn extract_two_finite_domains<'a>(
    domain1: &'a dyn DomainValue,
    domain2: &'a dyn DomainValue,
) -> Option<(
    &'a crate::state::FiniteDomain,
    &'a crate::state::FiniteDomain,
)> {
    match (as_finite_domain(domain1), as_finite_domain(domain2)) {
        (Some(fd1), Some(fd2)) => Some((fd1, fd2)),
        _ => None,
    }
}

/// Dynamic domain store that can hold different types of domain values
#[derive(Debug)]
pub struct DynamicDomainStore {
    /// Map from variables to their domain values
    domains: HashMap<LTerm, Box<dyn DomainValue>>,
}

impl DynamicDomainStore {
    /// Create a new dynamic domain store
    pub fn new() -> Self {
        Self {
            domains: HashMap::new(),
        }
    }

    /// Add or update a domain for a variable
    pub fn insert(&mut self, var: LTerm, domain: Box<dyn DomainValue>) {
        self.domains.insert(var, domain);
    }

    /// Get the domain for a variable
    pub fn get(&self, var: &LTerm) -> Option<&dyn DomainValue> {
        self.domains.get(var).map(|d| d.as_ref())
    }

    /// Remove a domain for a variable
    pub fn remove(&mut self, var: &LTerm) -> Option<Box<dyn DomainValue>> {
        self.domains.remove(var)
    }

    /// Check if a variable has a domain
    pub fn contains_key(&self, var: &LTerm) -> bool {
        self.domains.contains_key(var)
    }

    /// Get the number of variables with domains
    pub fn len(&self) -> usize {
        self.domains.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.domains.is_empty()
    }

    /// Clear all domains
    pub fn clear(&mut self) {
        self.domains.clear();
    }

    /// Get an iterator over all variable-domain pairs
    pub fn iter(&self) -> impl Iterator<Item = (&LTerm, &dyn DomainValue)> {
        self.domains.iter().map(|(k, v)| (k, v.as_ref()))
    }

    /// Clone the entire domain store
    pub fn clone_store(&self) -> DynamicDomainStore {
        let mut new_domains = HashMap::new();
        for (var, domain) in &self.domains {
            new_domains.insert(var.clone(), domain.clone_box());
        }
        DynamicDomainStore {
            domains: new_domains,
        }
    }

    /// Meet the domain of a variable with another domain (lattice meet operation)
    /// Returns None if meet fails (inconsistent constraints)
    pub fn meet_domain(&mut self, var: &LTerm, other_domain: &dyn DomainValue) -> Option<()> {
        if let Some(current_domain) = self.domains.get(var) {
            // Check if domains are compatible (same type)
            if current_domain.domain_type() != other_domain.domain_type() {
                return None; // Incompatible domain types
            }

            // Perform meet operation (greatest lower bound)
            if let Some(meet_domain) = current_domain.meet(other_domain) {
                if meet_domain.is_empty() {
                    return None; // Meet resulted in inconsistency (bottom)
                }
                self.domains.insert(var.clone(), meet_domain);
                Some(())
            } else {
                None // Meet failed
            }
        } else {
            // Variable doesn't have a domain yet, assign the other domain
            self.domains.insert(var.clone(), other_domain.clone_box());
            Some(())
        }
    }

    /// Join domains for widening in abstract interpretation
    /// Creates a less restrictive domain that subsumes both inputs
    pub fn join_domain(&mut self, var: &LTerm, other_domain: &dyn DomainValue) -> Option<()> {
        if let Some(current_domain) = self.domains.get(var) {
            if current_domain.domain_type() != other_domain.domain_type() {
                return None; // Incompatible domain types
            }

            if let Some(joined_domain) = current_domain.join(other_domain) {
                self.domains.insert(var.clone(), joined_domain);
                Some(())
            } else {
                None
            }
        } else {
            self.domains.insert(var.clone(), other_domain.clone_box());
            Some(())
        }
    }

    /// Check if one domain subsumes another (lattice ordering)
    pub fn domain_subsumes(&self, var1: &LTerm, var2: &LTerm) -> Option<bool> {
        let domain1 = self.domains.get(var1)?;
        let domain2 = self.domains.get(var2)?;

        if domain1.domain_type() == domain2.domain_type() {
            Some(domain1.subsumes(domain2.as_ref()))
        } else {
            None // Different domain types
        }
    }

    /// Get an iterator over all variable keys in the domain store
    pub fn keys(&self) -> impl Iterator<Item = &LTerm> {
        self.domains.keys()
    }
}

impl Clone for DynamicDomainStore {
    fn clone(&self) -> Self {
        self.clone_store()
    }
}
