use crate::compound::CompoundObject;
use crate::lterm::{LTerm, LTermInner};
use std::collections::HashMap;
use std::ops::Deref;

/// Substitution Map
///
/// Substitution maps track the binding of variables to terms.
#[derive(Debug, Clone)]
pub struct SMap(HashMap<LTerm, LTerm>);

impl SMap {
    /// Construct an an empty substitution map with no substitutions
    pub fn new() -> SMap {
        SMap(HashMap::new())
    }

    /// Extend substitution map with a new substitution
    pub fn extend(&mut self, k: LTerm, v: LTerm) {
        self.0.insert(k, v);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Walk substitution map
    ///
    /// Walking the substitution map recursively traverses the map until no next term is found,
    /// or the term found is a non-variable.
    pub fn walk<'a>(&'a self, mut k: &'a LTerm) -> &'a LTerm {
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
    pub fn walk_if<'a, 'b>(&'a self, k: &'b LTerm) -> Option<&'a LTerm> {
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
    pub fn walk_star(&self, v: &LTerm) -> LTerm {
        let v = self.walk(v);
        match v.as_ref() {
            LTermInner::Cons(head, tail) => LTerm::cons(self.walk_star(head), self.walk_star(tail)),
            LTermInner::Compound(compound) => compound.walk_star(self),
            _ => v.clone(),
        }
    }

    /// Check that the variable `x` is not contained in the compound object `compound`.
    fn occurs_check_compound(&self, x: &LTerm, compound: &dyn CompoundObject) -> bool {
        compound.children().any(|child| match child.as_term() {
            Some(v) => self.occurs_check(x, v),
            None => self.occurs_check_compound(x, child),
        })
    }

    /// Check that the variable `x` is not contained in the term `v`.
    ///
    /// Occurs check is used to prevent unification of terms that would cause the variable to
    /// be contained in itself.
    pub fn occurs_check(&self, x: &LTerm, v: &LTerm) -> bool {
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

    fn reify_compound(&self, compound: &dyn CompoundObject) -> SMap {
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
    pub fn reify(&self, v: &LTerm) -> SMap {
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

    fn is_anyvar_compound(&self, compound: &dyn CompoundObject) -> bool {
        compound.children().any(|child| match child.as_term() {
            Some(v) => self.is_anyvar(v),
            None => self.is_anyvar_compound(child),
        })
    }

    /// Check if the given logic term refers to any unassociated variables
    pub fn is_anyvar(&self, v: &LTerm) -> bool {
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
    pub fn get_vars(&self) -> Vec<&LTerm> {
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
    pub fn operands(&self) -> Vec<LTerm> {
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

impl IntoIterator for SMap {
    type Item = (LTerm, LTerm);
    type IntoIter = ::std::collections::hash_map::IntoIter<LTerm, LTerm>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Deref for SMap {
    type Target = HashMap<LTerm, LTerm>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
