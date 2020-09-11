use crate::lterm::LTerm;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

/// Substitution Map
///
/// Substitution maps track the binding of variables to terms.
#[derive(Clone, Debug)]
pub struct SMap(HashMap<Rc<LTerm>, Rc<LTerm>>);

impl SMap {
    /// Construct an an empty substitution map with no substitutions
    pub fn new() -> SMap {
        SMap(HashMap::new())
    }

    /// Extend substitution map with a new substitution
    pub fn extend(&mut self, k: Rc<LTerm>, v: Rc<LTerm>) {
        self.0.insert(k, v);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Walk substitution map
    ///
    /// Walking the substitution map recursively traverses the map until no next term is found,
    /// or the term found is a non-variable.
    pub fn walk<'a>(&'a self, mut k: &'a Rc<LTerm>) -> &'a Rc<LTerm> {
        loop {
            match k.as_ref() {
                LTerm::Var(_, _) => {
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
    pub fn walk_if<'a, 'b>(&'a self, k: &'b Rc<LTerm>) -> Option<&'a Rc<LTerm>> {
        if k.is_var() {
            // First step
            let mut step = match self.0.get(k) {
                Some(first) => first,
                None => return None,
            };

            // Further steps have lifetime of `self`, not input `k`
            loop {
                match step.as_ref() {
                    LTerm::Var(_, _) => match self.0.get(step) {
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
    pub fn walk_star(&self, v: &Rc<LTerm>) -> Rc<LTerm> {
        let v = self.walk(v);
        match v.as_ref() {
            LTerm::Cons(head, tail) => LTerm::cons(self.walk_star(head), self.walk_star(tail)),
            _ => Rc::clone(v),
        }
    }

    /// Check that the variable `x` is not contained in the term `v`.
    ///
    /// Occurs check is used to prevent unification of terms that would cause the variable to
    /// be contained in itself.
    pub fn occurs_check(&self, x: &Rc<LTerm>, v: &Rc<LTerm>) -> bool {
        match self.walk(v).as_ref() {
            LTerm::Var(vvar, _) => match x.as_ref() {
                LTerm::Var(xvar, _) => *vvar == *xvar,
                _ => false,
            },
            LTerm::Cons(head, tail) => self.occurs_check(x, head) || self.occurs_check(x, tail),
            _ => false,
        }
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
    pub fn reify(&self, v: &Rc<LTerm>) -> SMap {
        let walkv = self.walk(v);
        match walkv.as_ref() {
            LTerm::Var(_, _) => {
                // If it was not possible to find substitution that ends in a value, then we
                // append substitution to Any-variable, which can have any value.
                let mut c = self.clone();
                c.extend(Rc::clone(walkv), LTerm::any());
                c
            }
            LTerm::Cons(head, tail) => self.reify(head).reify(tail),
            _ => self.clone(),
        }
    }

    /// Check if the given logic term refers to any unassociated variables
    pub fn is_anyvar(&self, v: &Rc<LTerm>) -> bool {
        match v.as_ref() {
            LTerm::Var(_, _) if self.contains_key(v) => {
                let walkv = self.walk(&v);
                walkv.is_var()
            }
            LTerm::Cons(u, v) => self.is_anyvar(u) || self.is_anyvar(v),
            _ => false,
        }
    }

    /// Returns a list of variables referenced by the substitution map
    pub fn get_vars(&self) -> Vec<&Rc<LTerm>> {
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
    pub fn operands(&self) -> Vec<&Rc<LTerm>> {
        let mut operands = vec![];
        for (k, v) in self.0.iter() {
            operands.push(k);
            if v.is_var() {
                operands.push(v);
            }
        }
        operands
    }
}

impl IntoIterator for SMap {
    type Item = (Rc<LTerm>, Rc<LTerm>);
    type IntoIter = ::std::collections::hash_map::IntoIter<Rc<LTerm>, Rc<LTerm>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Deref for SMap {
    type Target = HashMap<Rc<LTerm>, Rc<LTerm>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<HashMap<Rc<LTerm>, Rc<LTerm>>> for SMap {
    fn as_ref(&self) -> &HashMap<Rc<LTerm>, Rc<LTerm>> {
        &self.0
    }
}

impl AsMut<HashMap<Rc<LTerm>, Rc<LTerm>>> for SMap {
    fn as_mut(&mut self) -> &mut HashMap<Rc<LTerm>, Rc<LTerm>> {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn test_smap_new() {
        let smap = SMap::new();
        // A newly created SMap is empty
        assert!(smap.is_empty());
    }

    #[test]
    fn test_smap_extend() {
        let mut smap = SMap::new();
        let v = lterm!(_);
        let t = lterm!(1234);

        // In an empty substitution map, a walk leads to nowhere.
        let w = smap.walk(&v);
        assert!(Rc::ptr_eq(&w, &v));

        // In an extended substitution map, a walk follows the map.
        smap.extend(Rc::clone(&v), Rc::clone(&t));
        let w = smap.walk(&v);
        assert!(Rc::ptr_eq(&w, &t));
    }

    #[test]
    fn test_smap_occurs_check_1() {
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        // Extending empty substitution map cannot fail occurs check
        assert!(!smap.occurs_check(&v0, &v1));
        smap.extend(Rc::clone(&v0), Rc::clone(&v1));

        // Continuing variable substitution without forming a loop does not fail occurs check
        assert!(!smap.occurs_check(&v1, &v2));
        smap.extend(Rc::clone(&v1), Rc::clone(&v2));

        // Checking if it is possible to form a loop of substitutions will trigger the occurs check
        assert!(smap.occurs_check(&v2, &v0));
    }

    #[test]
    fn test_smap_occurs_check_2() {
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!(_);
        let l = LTerm::cons(Rc::clone(&v1), Rc::clone(&v2));

        // Extending empty substitution map cannot fail occurs check
        assert!(!smap.occurs_check(&v0, &l));
        smap.extend(Rc::clone(&v0), Rc::clone(&l));

        // Continuing variable substitution without forming a loop does not fail occurs check
        assert!(!smap.occurs_check(&v1, &v3));
        smap.extend(Rc::clone(&v1), Rc::clone(&v3));

        // Checking if it is possible to form a loop of substitutions will trigger the occurs check
        assert!(smap.occurs_check(&v2, &v0));
    }

    #[test]
    fn test_smap_walk_1() {
        // 1. Variable not found in map => input returned back as it is impossible to walk
        let smap = SMap::new();
        let v = lterm!(_);
        let w = smap.walk(&v);
        assert!(Rc::ptr_eq(&v, &w));
    }

    #[test]
    fn test_smap_walk_2() {
        // 2. Variable found => walked until no more variables: ends in last variable
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(Rc::clone(&v0), Rc::clone(&v1));
        smap.extend(Rc::clone(&v1), Rc::clone(&v2));

        let w = smap.walk(&v0);
        assert!(Rc::ptr_eq(&v2, &w));
    }

    #[test]
    fn test_smap_walk_3() {
        // 2. Variable found => walked until no more variables: ends in last value
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(Rc::clone(&v0), Rc::clone(&v1));
        smap.extend(Rc::clone(&v1), Rc::clone(&v2));

        let v3 = lterm!(1);
        smap.extend(Rc::clone(&v2), Rc::clone(&v3));
        let w = smap.walk(&v0);
        assert!(Rc::ptr_eq(&v3, &w));
    }

    #[test]
    fn test_smap_walk_4() {
        // 2. Variable found => walked until no more variables: ends in last list and does not
        //    recurse into the list.
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(Rc::clone(&v0), Rc::clone(&v1));
        smap.extend(Rc::clone(&v1), Rc::clone(&v2));

        let v3 = lterm!(_);
        let vs = LTerm::singleton(Rc::clone(&v3));
        let v4 = lterm!(_);
        smap.extend(Rc::clone(&v2), Rc::clone(&vs));
        smap.extend(Rc::clone(&v3), Rc::clone(&v4));
        let w = smap.walk(&v0);
        assert!(Rc::ptr_eq(&vs, &w));
    }

    #[test]
    fn test_smap_walk_star_1() {
        // 1. Variable not found in map => input returned back as it is impossible to walk
        let smap = SMap::new();
        let v = lterm!(_);
        let w = smap.walk_star(&v);
        assert!(Rc::ptr_eq(&v, &w));
    }

    #[test]
    fn test_smap_walk_star_2() {
        // 2. Variable found => walked until no more variables: ends in last variable
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(Rc::clone(&v0), Rc::clone(&v1));
        smap.extend(Rc::clone(&v1), Rc::clone(&v2));

        let w = smap.walk_star(&v0);
        assert!(Rc::ptr_eq(&v2, &w));
    }

    #[test]
    fn test_smap_walk_star_3() {
        // 2. Variable found => walked until no more variables: ends in last value
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(Rc::clone(&v0), Rc::clone(&v1));
        smap.extend(Rc::clone(&v1), Rc::clone(&v2));

        let v3 = lterm!(1);
        smap.extend(Rc::clone(&v2), Rc::clone(&v3));
        let w = smap.walk_star(&v0);
        assert!(Rc::ptr_eq(&v3, &w));
    }

    #[test]
    fn test_smap_walk_star_4() {
        // 2. Variable found => walked until no more variables: ends in last list and does
        //    recurse into the list.
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(Rc::clone(&v0), Rc::clone(&v1));
        smap.extend(Rc::clone(&v1), Rc::clone(&v2));

        let v3 = lterm!(_);
        let vs = LTerm::singleton(Rc::clone(&v3));
        let v4 = lterm!(_);
        smap.extend(Rc::clone(&v2), Rc::clone(&vs));
        smap.extend(Rc::clone(&v3), Rc::clone(&v4));
        let w = smap.walk_star(&v0);
        match w.as_ref() {
            LTerm::Cons(head, _) => {
                assert!(Rc::ptr_eq(head, &v4));
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_smap_reify() {
        let smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v = LTerm::cons(Rc::clone(&v0), LTerm::singleton(Rc::clone(&v1)));

        let r = smap.reify(&v);
        assert!(r.walk(&v0).as_ref().is_var());
        assert!(r.walk(&v1).as_ref().is_var());
    }
}
