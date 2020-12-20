use super::substitution::SMap;
use crate::lterm::{LTerm, LTermInner};
use std::rc::Rc;

/// Recursive unification of tree terms
pub fn unify_rec(mut smap: &mut Rc<SMap>, extension: &mut SMap, u: &LTerm, v: &LTerm) -> bool {
    let uwalk = smap.walk(u).clone();
    let vwalk = smap.walk(v).clone();
    match (uwalk.as_ref(), vwalk.as_ref()) {
        (LTermInner::Var(uvar, _), LTermInner::Var(vvar, _)) if uvar == vvar => {
            // If both terms are variables that walk to the same variable id, then the current
            // state can already unify the variables. Return the input state unchanged.
            true
        }
        (LTermInner::Var(_, _), _) => {
            // The term u is a variable and the term v is something else. The variable u and
            // the term v can be unified by extending the substitution map.
            if smap.occurs_check(&uwalk, &vwalk) {
                false
            } else {
                extension.extend(uwalk.clone(), vwalk.clone());
                Rc::make_mut(&mut smap).extend(uwalk.clone(), vwalk.clone());
                true
            }
        }
        (_, LTermInner::Var(_, _)) => {
            // The term `v` is a variable and the term `u` is something else. The variable `v`
            // and the term `u` can be unified by extending the substitution map.
            if smap.occurs_check(&vwalk, &uwalk) {
                false
            } else {
                extension.extend(vwalk.clone(), uwalk.clone());
                Rc::make_mut(&mut smap).extend(vwalk.clone(), uwalk.clone());
                true
            }
        }
        (LTermInner::Val(uval), LTermInner::Val(vval)) if uval == vval => {
            // If both terms walk to identical values, then they are already unified.
            true
        }
        (LTermInner::User(uuser), _) => uuser.unify(&uwalk, &vwalk, smap, extension),
        (_, LTermInner::User(vuser)) => vuser.unify(&vwalk, &uwalk, smap, extension),
        (LTermInner::Empty, LTermInner::Empty) => true,
        (LTermInner::Cons(uhead, utail), LTermInner::Cons(vhead, vtail)) => {
            if unify_rec(smap, extension, uhead, vhead) {
                unify_rec(smap, extension, utail, vtail)
            } else {
                false
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn test_unify_1() {
        // 1. var == var
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v0.clone());

        // both v1 and v2 can walk to same variable id, therefore unification should be successful
        // with current substitution
        let mut extension = SMap::new();
        assert!(unify_rec(&mut Rc::new(smap), &mut extension, &v1, &v2));
        assert!(extension.is_empty());
    }

    #[test]
    fn test_unify_2() {
        // 2. var != var
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!(_);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // both v1 and v2 can walk to different variable id, unify by substituting variables
        let mut extension = SMap::new();
        assert!(unify_rec(&mut Rc::new(smap), &mut extension, &v1, &v2));
        assert!(!extension.is_empty());
    }

    #[test]
    fn test_unify_3() {
        // 3. var == val
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!(1);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 walks to variable 0, v2 walks to value => success and extended map from v0 to v2
        let mut smap = Rc::new(smap);
        let mut extension = SMap::new();
        assert!(unify_rec(&mut smap, &mut extension, &v1, &v2));
        assert!(!extension.is_empty());
        let w = smap.walk(&v0);
        assert!(LTerm::ptr_eq(&v3, &w));
    }

    #[test]
    fn test_unify_4() {
        // 4. var == list
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!([1]);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 walks to variable 0, v2 walks to value => success and extended map from v0 to v2
        let mut smap = Rc::new(smap);
        let mut extension = SMap::new();
        assert!(unify_rec(&mut smap, &mut extension, &v1, &v2));
        assert!(!extension.is_empty());
        let w = smap.walk(&v0);
        assert!(LTerm::ptr_eq(&v3, &w));
    }

    #[test]
    fn test_unify_5() {
        // 5. val == var
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!(1);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 walks to variable 0, v2 walks to value => success and extended map from v0 to v2
        let mut smap = Rc::new(smap);
        let mut extension = SMap::new();
        assert!(unify_rec(&mut smap, &mut extension, &v1, &v2));
        assert!(!extension.is_empty());
        let w = smap.walk(&v0);
        assert!(LTerm::ptr_eq(&v3, &w));
    }

    #[test]
    fn test_unify_6() {
        // 6. list == var
        let mut smap = SMap::new();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!([1]);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 walks to variable 0, v2 walks to value => success and extended map from v0 to v2
        let mut smap = Rc::new(smap);
        let mut extension = SMap::new();
        assert!(unify_rec(&mut smap, &mut extension, &v1, &v2));
        assert!(!extension.is_empty());
        let w = smap.walk(&v0);
        assert!(LTerm::ptr_eq(&v3, &w));
    }

    #[test]
    fn test_unify_7() {
        // 7. val == val
        let mut smap = SMap::new();
        let v0 = lterm!(1);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!(1);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 and v2 walk to identical values => success
        let mut extension = SMap::new();
        assert!(unify_rec(&mut Rc::new(smap), &mut extension, &v1, &v2));
    }

    #[test]
    fn test_unify_8() {
        // 8. val != val
        let mut smap = SMap::new();
        let v0 = lterm!(1);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!(2);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 and v2 walk to different values => failure
        let mut extension = SMap::new();
        assert!(!unify_rec(&mut Rc::new(smap), &mut extension, &v1, &v2));
    }

    #[test]
    fn test_unify_9() {
        // 9. list[N] == list[N]
        let mut smap = Rc::new(SMap::new());
        let v0 = lterm!([1]);
        let v1 = lterm!([1]);

        // v0 and v1 are identical lists => success
        let mut extension = SMap::new();
        assert!(unify_rec(&mut smap, &mut extension, &v0, &v1));
        assert!(extension.is_empty());
    }

    #[test]
    fn test_unify_10() {
        // 10. list[N] != list[N]
        let mut smap = SMap::new();
        let v0 = lterm!([1]);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!([2]);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 and v2 walk to different lists of same length => failure
        let mut extension = SMap::new();
        assert!(!unify_rec(&mut Rc::new(smap), &mut extension, &v1, &v2));
    }

    #[test]
    fn test_unify_11() {
        // 11. list[N] != list[M] where N != M
        let mut smap = SMap::new();
        let v0 = lterm!([1 | 1]);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!([1]);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 and v2 walk to different length lists with same values => failure
        let mut extension = SMap::new();
        assert!(!unify_rec(&mut Rc::new(smap), &mut extension, &v1, &v2));
    }
}
