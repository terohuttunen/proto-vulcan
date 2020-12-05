use super::SMap;
use crate::lterm::LTerm;
use crate::state::constraint::{Constraint, DisequalityConstraint};
use crate::state::UserState;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct ConstraintStore<U: UserState>(HashSet<Constraint<U>>);

impl<U: UserState> ConstraintStore<U> {
    pub fn new() -> ConstraintStore<U> {
        ConstraintStore(HashSet::new())
    }

    /// Remove irrelevant constraints
    ///
    /// The method finds all constraints that refer to unassociated variables in the given
    /// substitution map. Unassociated variables can be Var(_) or Any. Associated variables are
    /// already fully constrained by the values they are associated with, whereas unassociated
    /// variables are constrained by the constraints.
    pub fn purify(self, r: &SMap) -> ConstraintStore<U> {
        let mut purified_cstore = ConstraintStore::new();
        for constraint in self.0.into_iter() {
            if let Constraint::Tree(ref tree_constraint) = constraint {
                if tree_constraint
                    .smap_ref()
                    .iter()
                    .any(|(u, _)| r.is_anyvar(u))
                {
                    purified_cstore.insert(constraint);
                }
            } else {
                purified_cstore.insert(constraint);
            }
        }
        purified_cstore
    }

    /// Do walk_star for each substitution of each constraint
    pub fn walk_star(&self, smap: &SMap) -> ConstraintStore<U> {
        let mut walked_cstore = ConstraintStore::new();
        for constraint in self.iter() {
            if let Constraint::Tree(tree_constraint) = constraint {
                let ws = tree_constraint.walk_star(smap);
                let c = Rc::new(DisequalityConstraint::from(ws));
                walked_cstore.insert(Constraint::from(c));
            }
        }
        walked_cstore
    }

    /// Add new constraint `c` while keeping the store normalized
    pub fn push_and_normalize(&mut self, newc: Constraint<U>) {
        if let Constraint::Tree(ref tree_newc) = newc {
            let mut normalized = HashSet::new();
            for storec in self.0.drain() {
                // All non-subsumable constraints are always carried along
                if let Constraint::Tree(ref tree_storec) = storec {
                    if !tree_storec.subsumes(&**tree_newc) && !tree_newc.subsumes(&**tree_storec) {
                        normalized.insert(storec);
                    }
                } else {
                    normalized.insert(storec);
                }
            }
            self.0 = normalized;
        }
        self.insert(newc);
    }

    /// Remove redundant constraints from the store
    pub fn normalize(self) -> ConstraintStore<U> {
        let mut normalized_store = ConstraintStore::new();
        for storec in self.0.into_iter() {
            normalized_store.push_and_normalize(storec.into());
        }
        normalized_store
    }

    pub fn iter(&self) -> impl Iterator<Item = &Constraint<U>> + '_ {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn take(&mut self, u: &Constraint<U>) -> Option<Constraint<U>> {
        self.0.take(u)
    }

    pub fn insert<T: Into<Constraint<U>>>(&mut self, key: T) -> bool {
        self.0.insert(key.into())
    }

    /// Iterate over constraints that refer to terms in `u`
    pub fn relevant<'a>(
        &'a self,
        relevant_operands: &Vec<LTerm>,
    ) -> impl Iterator<Item = &'a Constraint<U>> {
        let relevant_operands = relevant_operands.clone();
        self.iter().filter(move |c| {
            c.operands()
                .iter()
                .any(|operand| relevant_operands.contains(operand))
        })
    }

    pub fn display_relevant(&self, u: &LTerm, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let anyvars = u.anyvars();
        let mut count = 0;
        for storec in self.relevant(&anyvars) {
            if let Constraint::Tree(treec) = storec {
                // Tree-disequality constraint has a substitution map that may have
                // multiple disequality sub-constraints. Each disequality is printed
                // here separately if it is relevant to the given operands.
                for (cu, cv) in treec
                    .smap_ref()
                    .iter()
                    .filter(|(cu, cv)| anyvars.contains(cu) || anyvars.contains(cv))
                {
                    if count > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} != {}", cu, cv)?;
                    count += 1;
                }
            } else {
                if count > 0 {
                    write!(f, ", ")?;
                }
                std::fmt::Display::fmt(storec, f)?;
                count += 1;
            }
        }
        write!(f, "")
    }
}
