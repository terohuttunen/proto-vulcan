use crate::compound::CompoundObject;
use crate::lterm::{LTerm, LTermInner};
use crate::user::{EmptyUser, User};
use std::collections::HashMap;
use std::ops::Deref;

/// Substitution Map
///
/// Substitution maps track the binding of variables to terms.
#[derive(Clone, Debug)]
pub struct SMap<U = EmptyUser>(HashMap<LTerm<U>, LTerm<U>>)
where
    U: User;

impl<U: User> SMap<U> {
    /// Construct an an empty substitution map with no substitutions
    pub fn new() -> SMap<U> {
        SMap(HashMap::new())
    }

    /// Extend substitution map with a new substitution
    pub fn extend(&mut self, k: LTerm<U>, v: LTerm<U>) {
        self.0.insert(k, v);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Walk substitution map
    ///
    /// Walking the substitution map recursively traverses the map until no next term is found,
    /// or the term found is a non-variable.
    pub fn walk<'a>(&'a self, mut k: &'a LTerm<U>) -> &'a LTerm<U> {
        loop {
            match k.as_ref() {
                LTermInner::Var(_, _) => {
                    match self.0.get(k) {
                        Some(s) => k = s, // recurse for variable-kind
                        None => return k, // if no next term found
                    }
                }
                _ => return k, // if the term is not a variable
            }
        }
    }

    /// Alternative walk of the substitution map that does not bind the return value lifetime
    /// to lifetime of the input variable `k`.
    pub fn walk_if<'a, 'b>(&'a self, k: &'b LTerm<U>) -> Option<&'a LTerm<U>> {
        if k.is_var() {
            // First step
            let mut step = match self.0.get(k) {
                Some(first) => first,
                None => return None,
            };

            // Further steps have lifetime of `self`, not input `k`
            loop {
                match step.as_ref() {
                    LTermInner::Var(_, _) => match self.0.get(step) {
                        Some(next) => step = next,
                        None => return Some(step),
                    },
                    _ => return Some(step),
                }
            }
        } else {
            None
        }
    }

    /// Deeper walk of substitution map
    ///
    /// Walks the substitution map recursively like `walk()`, but does not stop at lists, and
    /// instead recurses to do the deep walk also for the list elements. Returns a term which
    /// is a tree where all leaves are walked terms.
    pub fn walk_star(&self, v: &LTerm<U>) -> LTerm<U> {
        let v = self.walk(v);
        match v.as_ref() {
            LTermInner::Cons(head, tail) => LTerm::cons(self.walk_star(head), self.walk_star(tail)),
            LTermInner::Compound(compound) => compound.walk_star(self),
            _ => v.clone(),
        }
    }

    /// Check that the variable `x` is not contained in the compound object `compound`.
    fn occurs_check_compound(&self, x: &LTerm<U>, compound: &dyn CompoundObject<U>) -> bool {
        compound.children().any(|child| match child.as_term() {
            Some(v) => self.occurs_check(x, v),
            None => self.occurs_check_compound(x, child),
        })
    }

    /// Check that the variable `x` is not contained in the term `v`.
    ///
    /// Occurs check is used to prevent unification of terms that would cause the variable to
    /// be contained in itself.
    pub fn occurs_check(&self, x: &LTerm<U>, v: &LTerm<U>) -> bool {
        match self.walk(v).as_ref() {
            LTermInner::Var(vvar, _) => match x.as_ref() {
                LTermInner::Var(xvar, _) => *vvar == *xvar,
                _ => false,
            },
            LTermInner::Cons(head, tail) => {
                self.occurs_check(x, head) || self.occurs_check(x, tail)
            }
            LTermInner::Compound(compound) => self.occurs_check_compound(x, compound.as_ref()),
            _ => false,
        }
    }

    fn reify_compound(&self, compound: &dyn CompoundObject<U>) -> SMap<U> {
        let mut smap = self.clone();
        for child in compound.children() {
            match child.as_term() {
                Some(v) => smap = smap.reify(v),
                None => smap = smap.reify_compound(child),
            }
        }
        smap
    }

    /// Reify substitution map
    ///
    /// Reification modifies the substitution map such that all variables of the given LTerm
    /// have walkable values assigned to them in the substitution map. If the term or any subterm
    /// walks into a variable, a reified name is added to the substitution map. The reified name
    /// denotes that the the solution solves the logic query with any value of the variable.
    ///
    /// This is typically used to generate a reifying substitution map from an empty map. The
    /// reifying map maps free variables to reified names. See State::reify().
    pub fn reify(&self, v: &LTerm<U>) -> SMap<U> {
        let walkv = self.walk(v);
        match walkv.as_ref() {
            LTermInner::Var(_, _) => {
                // If it was not possible to find substitution that ends in a value, then we
                // append substitution to Any-variable, which can have any value.
                let mut c = self.clone();
                c.extend(walkv.clone(), LTerm::any());
                c
            }
            LTermInner::Cons(head, tail) => self.reify(head).reify(tail),
            LTermInner::Compound(compound) => self.reify_compound(compound.as_ref()),
            _ => self.clone(),
        }
    }

    fn is_anyvar_compound(&self, compound: &dyn CompoundObject<U>) -> bool {
        compound.children().any(|child| match child.as_term() {
            Some(v) => self.is_anyvar(v),
            None => self.is_anyvar_compound(child),
        })
    }

    /// Check if the given logic term refers to any unassociated variables
    pub fn is_anyvar(&self, v: &LTerm<U>) -> bool {
        match v.as_ref() {
            LTermInner::Var(_, _) if self.contains_key(v) => {
                let walkv = self.walk(&v);
                walkv.is_var()
            }
            LTermInner::Cons(u, v) => self.is_anyvar(u) || self.is_anyvar(v),
            LTermInner::Compound(compound) => self.is_anyvar_compound(compound.as_ref()),
            _ => false,
        }
    }

    /// Returns a list of variables referenced by the substitution map
    pub fn get_vars(&self) -> Vec<&LTerm<U>> {
        let mut vars = vec![];
        for (k, v) in self.0.iter() {
            vars.push(k);
            if v.is_var() {
                vars.push(v);
            }
        }
        vars
    }

    /// Returns a set of variables operands referencesd by the substitution
    pub fn operands(&self) -> Vec<LTerm<U>> {
        let mut operands = vec![];
        for (k, v) in self.0.iter() {
            operands.push(k.clone());
            if v.is_var() {
                operands.push(v.clone());
            }
        }
        operands
    }
}

impl<U: User> IntoIterator for SMap<U> {
    type Item = (LTerm<U>, LTerm<U>);
    type IntoIter = ::std::collections::hash_map::IntoIter<LTerm<U>, LTerm<U>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<U: User> Deref for SMap<U> {
    type Target = HashMap<LTerm<U>, LTerm<U>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smap_new() {
        let smap = SMap::<EmptyUser>::new();
        // A newly created SMap is empty
        assert!(smap.is_empty());
    }

    #[test]
    fn test_smap_extend() {
        let mut smap = SMap::<EmptyUser>::new();
        let v = lterm!(_);
        let t = lterm!(1234);

        // In an empty substitution map, a walk leads to nowhere.
        let w = smap.walk(&v);
        assert!(LTerm::ptr_eq(&w, &v));

        // In an extended substitution map, a walk follows the map.
        smap.extend(v.clone(), t.clone());
        let w = smap.walk(&v);
        assert!(LTerm::ptr_eq(&w, &t));
    }

    #[test]
    fn test_smap_occurs_check_1() {
        let mut smap = SMap::<EmptyUser>::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        // Extending empty substitution map cannot fail occurs check
        assert!(!smap.occurs_check(&v0, &v1));
        smap.extend(v0.clone(), v1.clone());

        // Continuing variable substitution without forming a loop does not fail occurs check
        assert!(!smap.occurs_check(&v1, &v2));
        smap.extend(v1.clone(), v2.clone());

        // Checking if it is possible to form a loop of substitutions will trigger the occurs check
        assert!(smap.occurs_check(&v2, &v0));
    }

    #[test]
    fn test_smap_occurs_check_2() {
        let mut smap = SMap::<EmptyUser>::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!(_);
        let l = LTerm::cons(v1.clone(), v2.clone());

        // Extending empty substitution map cannot fail occurs check
        assert!(!smap.occurs_check(&v0, &l));
        smap.extend(v0.clone(), l.clone());

        // Continuing variable substitution without forming a loop does not fail occurs check
        assert!(!smap.occurs_check(&v1, &v3));
        smap.extend(v1.clone(), v3.clone());

        // Checking if it is possible to form a loop of substitutions will trigger the occurs check
        assert!(smap.occurs_check(&v2, &v0));
    }

    #[test]
    fn test_smap_walk_1() {
        // 1. Variable not found in map => input returned back as it is impossible to walk
        let smap = SMap::<EmptyUser>::new();
        let v = lterm!(_);
        let w = smap.walk(&v);
        assert!(LTerm::ptr_eq(&v, &w));
    }

    #[test]
    fn test_smap_walk_2() {
        // 2. Variable found => walked until no more variables: ends in last variable
        let mut smap = SMap::<EmptyUser>::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(v0.clone(), v1.clone());
        smap.extend(v1.clone(), v2.clone());

        let w = smap.walk(&v0);
        assert!(LTerm::ptr_eq(&v2, &w));
    }

    #[test]
    fn test_smap_walk_3() {
        // 2. Variable found => walked until no more variables: ends in last value
        let mut smap = SMap::<EmptyUser>::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(v0.clone(), v1.clone());
        smap.extend(v1.clone(), v2.clone());

        let v3 = lterm!(1);
        smap.extend(v2.clone(), v3.clone());
        let w = smap.walk(&v0);
        assert!(LTerm::ptr_eq(&v3, &w));
    }

    #[test]
    fn test_smap_walk_4() {
        // 2. Variable found => walked until no more variables: ends in last list and does not
        //    recurse into the list.
        let mut smap = SMap::<EmptyUser>::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(v0.clone(), v1.clone());
        smap.extend(v1.clone(), v2.clone());

        let v3 = lterm!(_);
        let vs = LTerm::singleton(v3.clone());
        let v4 = lterm!(_);
        smap.extend(v2.clone(), vs.clone());
        smap.extend(v3.clone(), v4.clone());
        let w = smap.walk(&v0);
        assert!(LTerm::ptr_eq(&vs, &w));
    }

    #[test]
    fn test_smap_walk_star_1() {
        // 1. Variable not found in map => input returned back as it is impossible to walk
        let smap = SMap::<EmptyUser>::new();
        let v = lterm!(_);
        let w = smap.walk_star(&v);
        assert!(LTerm::ptr_eq(&v, &w));
    }

    #[test]
    fn test_smap_walk_star_2() {
        // 2. Variable found => walked until no more variables: ends in last variable
        let mut smap = SMap::<EmptyUser>::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(v0.clone(), v1.clone());
        smap.extend(v1.clone(), v2.clone());

        let w = smap.walk_star(&v0);
        assert!(LTerm::ptr_eq(&v2, &w));
    }

    #[test]
    fn test_smap_walk_star_3() {
        // 2. Variable found => walked until no more variables: ends in last value
        let mut smap = SMap::<EmptyUser>::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(v0.clone(), v1.clone());
        smap.extend(v1.clone(), v2.clone());

        let v3 = lterm!(1);
        smap.extend(v2.clone(), v3.clone());
        let w = smap.walk_star(&v0);
        assert!(LTerm::ptr_eq(&v3, &w));
    }

    #[test]
    fn test_smap_walk_star_4() {
        // 2. Variable found => walked until no more variables: ends in last list and does
        //    recurse into the list.
        let mut smap = SMap::<EmptyUser>::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(v0.clone(), v1.clone());
        smap.extend(v1.clone(), v2.clone());

        let v3 = lterm!(_);
        let vs = LTerm::singleton(v3.clone());
        let v4 = lterm!(_);
        smap.extend(v2.clone(), vs.clone());
        smap.extend(v3.clone(), v4.clone());
        let w = smap.walk_star(&v0);
        match w.as_ref() {
            LTermInner::Cons(head, _) => {
                assert!(LTerm::ptr_eq(head, &v4));
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_smap_reify() {
        let smap = SMap::<EmptyUser>::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v = LTerm::cons(v0.clone(), LTerm::singleton(v1.clone()));

        let r = smap.reify(&v);
        assert!(r.walk(&v0).is_var());
        assert!(r.walk(&v1).is_var());
    }
}
