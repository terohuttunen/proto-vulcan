use crate::lterm::LTerm;
use crate::state::constraint::{BaseConstraint, Constraint, TreeConstraint};
use crate::state::unification::unify_rec;
use crate::state::{SMap, SResult, State, User};
use std::rc::Rc;

// Disequality constraint
#[derive(Clone, Debug)]
pub struct DisequalityConstraint(SMap);

impl DisequalityConstraint {
    pub fn new<U: User>(smap: SMap) -> Constraint<U> {
        Constraint::Tree(Rc::new(DisequalityConstraint(smap)))
    }
}

impl<U: User> BaseConstraint<U> for DisequalityConstraint {
    fn run(self: Rc<Self>, state: State<U>) -> SResult<U> {
        let mut extension = SMap::new();
        let mut smap = Rc::new(state.smap_ref().clone());
        for (u, v) in self.0.iter() {
            if !unify_rec(&mut smap, &mut extension, &u, &v) {
                return Ok(state);
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

impl std::fmt::Display for DisequalityConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (u, v) in self.0.iter() {
            write!(f, "{} != {},", u, v)?;
        }
        write!(f, "")
    }
}

impl<U: User> TreeConstraint<U> for DisequalityConstraint {
    /// If the `self` subsumes the `other`.
    ///
    /// A constraint is subsumed by another constraint if unifying the constraint in the
    /// substitution of the another constraint does not extend the constraint.
    fn subsumes(&self, other: &dyn TreeConstraint<U>) -> bool {
        let mut extension = SMap::new();
        let mut smap = Rc::new(other.smap_ref().clone());
        for (u, v) in self.0.iter() {
            if !unify_rec(&mut smap, &mut extension, &u, &v) {
                return false;
            }
        }

        extension.is_empty()
    }

    fn smap_ref(&self) -> &SMap {
        &self.0
    }

    fn walk_star(&self, smap: &SMap) -> SMap {
        let mut n = SMap::new();
        for (k, v) in TreeConstraint::<U>::smap_ref(self).iter() {
            let kwalk = smap.walk_star(k);
            let vwalk = smap.walk_star(v);
            assert!(kwalk.is_var());
            n.extend(kwalk, vwalk);
        }
        n
    }
}

impl<U: User> From<Rc<DisequalityConstraint>> for Constraint<U> {
    fn from(c: Rc<DisequalityConstraint>) -> Constraint<U> {
        Constraint::Tree(c as Rc<dyn TreeConstraint<U>>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::EmptyUser;
    use crate::*;

    #[test]
    fn test_subsumes_1() {
        // ((x.5)) subsumes ((x.5)(y.6))
        let x = lterm!(_);
        let y = lterm!(_);
        let five = lterm!(5);
        let six = lterm!(6);
        let mut smap = SMap::new();
        smap.extend(x.clone(), five.clone());
        smap.extend(y.clone(), six.clone());
        let c0 = DisequalityConstraint::new(smap);
        let mut smap = SMap::new();
        smap.extend(x.clone(), five.clone());
        let c1 = DisequalityConstraint::new(smap);
        match (c0, c1) {
            (Constraint::Tree(t0), Constraint::Tree(t1)) => assert!(TreeConstraint::<EmptyUser>::subsumes(&*t1, &*t0)),
            _=> assert!(false)
        }
    }
}
