use crate::lterm::LTerm;
use crate::state::constraint::Constraint;
use crate::state::{SMap, SResult, State, User};
use std::rc::Rc;

// Disequality constraint
#[derive(Clone, Debug)]
pub struct DisequalityConstraint<U: User>(SMap<U>);

impl<U: User> DisequalityConstraint<U> {
    pub fn new(smap: SMap<U>) -> Rc<dyn Constraint<U>> {
        Rc::new(DisequalityConstraint(smap))
    }

    /// If the `self` subsumes the `other`.
    ///
    /// A constraint is subsumed by another constraint if unifying the constraint in the
    /// substitution of the another constraint does not extend the constraint.
    pub fn subsumes(&self, other: &dyn Constraint<U>) -> bool {
        match other.downcast_ref::<Self>() {
            Some(other) => {
                let mut extension = SMap::new();
                let mut state = State::new(Default::default()).with_smap(other.smap_ref().clone());
                for (u, v) in self.0.iter() {
                    match U::unify(state, &mut extension, &u, &v) {
                        Err(()) => return false,
                        Ok(s) => state = s,
                    }
                }

                extension.is_empty()
            }
            None => false,
        }
    }

    pub fn smap_ref(&self) -> &SMap<U> {
        &self.0
    }

    pub fn walk_star(&self, smap: &SMap<U>) -> SMap<U> {
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

impl<U: User> Constraint<U> for DisequalityConstraint<U> {
    fn run(self: Rc<Self>, state: State<U>) -> SResult<U> {
        let mut extension = SMap::new();
        let mut test_state = state.clone();
        for (u, v) in self.0.iter() {
            match U::unify(test_state, &mut extension, &u, &v) {
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

    fn operands(&self) -> Vec<LTerm<U>> {
        self.0.operands()
    }
}

impl<U: User> std::fmt::Display for DisequalityConstraint<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (u, v) in self.0.iter() {
            write!(f, "{} != {},", u, v)?;
        }
        write!(f, "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::EmptyUser;
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
        match (
            c0.downcast_ref::<DisequalityConstraint<EmptyUser>>(),
            c1.downcast_ref::<DisequalityConstraint<EmptyUser>>(),
        ) {
            (Some(t0), Some(t1)) => {
                assert!(t1.subsumes(&*t0))
            }
            _ => assert!(false),
        }
    }
}
