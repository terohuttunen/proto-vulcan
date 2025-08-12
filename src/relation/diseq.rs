//! # CLP(Tree)
//! Proto-vulcan implements disequality constraint for tree-terms with built-in syntax: `x != y`.
//!
//! # Example
//! ```rust
//! extern crate proto_vulcan;
//! use proto_vulcan::prelude::*;
//! fn main() {
//!     let query = proto_vulcan_query!(|x, y| {
//!         [x, 1] != [2, y],
//!     });
//!
//!     for result in query.run() {
//!         println!("{}", result);
//!     }
//! }
//! ```
//! Because the variables are not fully constrained, they can be anything except specific values,
//! and the output of the example is:
//! ```text
//! x: _.3  where  { _.3 != 2 }
//! y: _.4  where  { _.4 != 1 }
//! ```
//!

use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::solver::{Solve, Solver};
use crate::state::{unify_rec, Constraint, SMap, SResult, State};
use crate::stream::Stream;

use std::rc::Rc;

#[derive(Debug)]
pub struct Diseq
{
    u: LTerm,
    v: LTerm,
}

impl Diseq
{
    pub fn new<G: AnyGoal>(u: LTerm, v: LTerm) -> InferredGoal<G> {
        InferredGoal::new(G::dynamic(Rc::new(Diseq { u, v })))
    }
}

impl Solve for Diseq
{
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        // Return state where u and v are unified under s, or None if unification is not possible
        match state.disunify(&self.u, &self.v) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

/// Disequality relation.
///
/// The disequality relation adds a disequality constraint. Proto-vulcan provides a built-in
/// syntax `x != y` that avoids adding the use-clause: `use proto_vulcan::relation::diseq`.
///
/// Note: currently this is only tree-disequality. For finite-domain disequality, diseqfd-relation
/// must be used instead.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// fn main() {
///     let query = proto_vulcan_query!(|x, y| {
///         [x, 1] != [2, y],
///     });
///     let mut iter = query.run();
///     let result = iter.next().unwrap();
///     assert!(result.x.is_any_except(&2));
///     assert!(result.y.is_any_except(&1));
///     assert!(iter.next().is_none());
/// }
/// ```
pub fn diseq<G>(u: LTerm, v: LTerm) -> InferredGoal<G>
where
    G: AnyGoal,
{
    Diseq::new(u, v)
}

// Disequality constraint
#[derive(Debug)]
pub struct DisequalityConstraint(SMap);

impl DisequalityConstraint
{
    pub fn new(smap: SMap) -> Rc<dyn Constraint> {
        Rc::new(DisequalityConstraint(smap))
    }

    /// If the `self` subsumes the `other`.
    ///
    /// A constraint is subsumed by another constraint if unifying the constraint in the
    /// substitution of the another constraint does not extend the constraint.
    pub fn subsumes(&self, other: &dyn Constraint) -> bool {
        match other.downcast_ref::<Self>() {
            Some(other) => {
                let mut extension = SMap::new();
                let mut state = State::new(std::rc::Rc::new(crate::state::ConstraintPluginRegistry::default())).with_smap(other.smap_ref().clone());
                for (u, v) in self.0.iter() {
                    match unify_rec(state, &mut extension, &u, &v) {
                        Err(()) => return false,
                        Ok(s) => state = s,
                    }
                }

                extension.is_empty()
            }
            None => false,
        }
    }

    pub fn smap_ref(&self) -> &SMap {
        &self.0
    }

    pub fn walk_star(&self, smap: &SMap) -> SMap {
        let mut n = SMap::new();
        for (k, v) in self.smap_ref().iter() {
            let kwalk = smap.walk_star(k);
            let vwalk = smap.walk_star(v);
            assert!(kwalk.is_var());
            n.extend(kwalk, vwalk);
        }
        n
    }
}

impl Constraint for DisequalityConstraint {
    fn run(self: Rc<Self>, state: State) -> SResult {
        let mut extension = SMap::new();
        let mut test_state = state.clone();
        for (u, v) in self.0.iter() {
            match unify_rec(test_state, &mut extension, &u, &v) {
                Err(_) => return Ok(state),
                Ok(new_state) => test_state = new_state,
            }
        }

        if extension.is_empty() {
            Err(())
        } else {
            let c = DisequalityConstraint::new(extension);
            Ok(state.with_constraint(c))
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        self.0.operands()
    }
}

impl std::fmt::Display for DisequalityConstraint
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (u, v) in self.0.iter() {
            write!(f, "{} != {},", u, v)?;
        }
        write!(f, "")
    }
}

