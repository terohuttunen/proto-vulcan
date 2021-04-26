use super::substitution::SMap;
use crate::lterm::{LTerm, LTermInner};
use crate::state::{SResult, State};
use crate::user::User;

/// Recursive unification of tree terms
pub fn unify_rec<U: User>(
    mut state: State<U>,
    extension: &mut SMap<U>,
    u: &LTerm<U>,
    v: &LTerm<U>,
) -> SResult<U> {
    let uwalk = state.smap_ref().walk(u).clone();
    let vwalk = state.smap_ref().walk(v).clone();
    match (uwalk.as_ref(), vwalk.as_ref()) {
        (LTermInner::Var(uvar, _), LTermInner::Var(vvar, _)) if uvar == vvar => {
            // If both terms are variables that walk to the same variable id, then the current
            // state can already unify the variables. Return the input state unchanged.
            Ok(state)
        }
        (LTermInner::Var(_, _), _) => {
            // The term u is a variable and the term v is something else. The variable u and
            // the term v can be unified by extending the substitution map.
            if state.smap_ref().occurs_check(&uwalk, &vwalk) {
                Err(())
            } else {
                extension.extend(uwalk.clone(), vwalk.clone());
                state.smap_to_mut().extend(uwalk, vwalk);
                Ok(state)
            }
        }
        (_, LTermInner::Var(_, _)) => {
            // The term `v` is a variable and the term `u` is something else. The variable `v`
            // and the term `u` can be unified by extending the substitution map.
            if state.smap_ref().occurs_check(&vwalk, &uwalk) {
                Err(())
            } else {
                extension.extend(vwalk.clone(), uwalk.clone());
                state.smap_to_mut().extend(vwalk, uwalk);
                Ok(state)
            }
        }
        (LTermInner::Val(uval), LTermInner::Val(vval)) if uval == vval => {
            // If both terms walk to identical values, then they are already unified.
            Ok(state)
        }
        (LTermInner::User(_), _) | (_, LTermInner::User(_)) => {
            U::unify(state, extension, uwalk, vwalk)
        }
        (LTermInner::Empty, LTermInner::Empty) => Ok(state),
        (LTermInner::Cons(uhead, utail), LTermInner::Cons(vhead, vtail)) => {
            match unify_rec(state, extension, uhead, vhead) {
                Ok(state) => unify_rec(state, extension, utail, vtail),
                Err(err) => Err(err),
            }
        }
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_unify_1() {
        // 1. var == var
        let mut state = State::<EmptyUser>::new(Default::default());
        let smap = state.smap_to_mut();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v0.clone());

        // both v1 and v2 can walk to same variable id, therefore unification should be successful
        // with current substitution
        let mut extension = SMap::new();
        assert!(matches!(unify_rec(state, &mut extension, &v1, &v2), Ok(_)));
        assert!(extension.is_empty());
    }

    #[test]
    fn test_unify_2() {
        // 2. var != var
        let mut state = State::<EmptyUser>::new(Default::default());
        let smap = state.smap_to_mut();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!(_);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // both v1 and v2 can walk to different variable id, unify by substituting variables
        let mut extension = SMap::new();
        assert!(matches!(unify_rec(state, &mut extension, &v1, &v2), Ok(_)));
        assert!(!extension.is_empty());
    }

    #[test]
    fn test_unify_3() {
        // 3. var == val
        let mut state = State::<EmptyUser>::new(Default::default());
        let smap = state.smap_to_mut();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!(1);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 walks to variable 0, v2 walks to value => success and extended map from v0 to v2
        let mut extension = SMap::new();
        match unify_rec(state, &mut extension, &v1, &v2) {
            Ok(state) => {
                assert!(!extension.is_empty());
                let w = state.smap_ref().walk(&v0);
                assert!(LTerm::ptr_eq(&v3, &w));
            }
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn test_unify_4() {
        // 4. var == list
        let mut state = State::<EmptyUser>::new(Default::default());
        let smap = state.smap_to_mut();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!([1]);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 walks to variable 0, v2 walks to value => success and extended map from v0 to v2
        let mut extension = SMap::new();
        match unify_rec(state, &mut extension, &v1, &v2) {
            Ok(state) => {
                assert!(!extension.is_empty());
                let w = state.smap_ref().walk(&v0);
                assert!(LTerm::ptr_eq(&v3, &w));
            }
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn test_unify_5() {
        // 5. val == var
        let mut state = State::<EmptyUser>::new(Default::default());
        let smap = state.smap_to_mut();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!(1);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 walks to variable 0, v2 walks to value => success and extended map from v0 to v2
        let mut extension = SMap::new();
        match unify_rec(state, &mut extension, &v1, &v2) {
            Ok(state) => {
                assert!(!extension.is_empty());
                let w = state.smap_ref().walk(&v0);
                assert!(LTerm::ptr_eq(&v3, &w));
            }
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn test_unify_6() {
        // 6. list == var
        let mut state = State::<EmptyUser>::new(Default::default());
        let smap = state.smap_to_mut();
        let v0 = lterm!(_);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!([1]);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 walks to variable 0, v2 walks to value => success and extended map from v0 to v2
        let mut extension = SMap::new();
        match unify_rec(state, &mut extension, &v1, &v2) {
            Ok(state) => {
                assert!(!extension.is_empty());
                let w = state.smap_ref().walk(&v0);
                assert!(LTerm::ptr_eq(&v3, &w));
            }
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn test_unify_7() {
        // 7. val == val
        let mut state = State::<EmptyUser>::new(Default::default());
        let smap = state.smap_to_mut();
        let v0 = lterm!(1);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!(1);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 and v2 walk to identical values => success
        let mut extension = SMap::new();
        assert!(matches!(unify_rec(state, &mut extension, &v1, &v2), Ok(_)));
    }

    #[test]
    fn test_unify_8() {
        // 8. val != val
        let mut state = State::<EmptyUser>::new(Default::default());
        let smap = state.smap_to_mut();
        let v0 = lterm!(1);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!(2);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 and v2 walk to different values => failure
        let mut extension = SMap::new();
        assert!(matches!(unify_rec(state, &mut extension, &v1, &v2), Err(_)));
    }

    #[test]
    fn test_unify_9() {
        // 9. list[N] == list[N]
        let state = State::<EmptyUser>::new(Default::default());
        let v0 = lterm!([1]);
        let v1 = lterm!([1]);

        // v0 and v1 are identical lists => success
        let mut extension = SMap::new();
        assert!(matches!(unify_rec(state, &mut extension, &v0, &v1), Ok(_)));
        assert!(extension.is_empty());
    }

    #[test]
    fn test_unify_10() {
        // 10. list[N] != list[N]
        let mut state = State::<EmptyUser>::new(Default::default());
        let smap = state.smap_to_mut();
        let v0 = lterm!([1]);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!([2]);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 and v2 walk to different lists of same length => failure
        let mut extension = SMap::new();
        assert!(matches!(unify_rec(state, &mut extension, &v1, &v2), Err(_)));
    }

    #[test]
    fn test_unify_11() {
        // 11. list[N] != list[M] where N != M
        let mut state = State::<EmptyUser>::new(Default::default());
        let smap = state.smap_to_mut();
        let v0 = lterm!([1 | 1]);
        let v1 = lterm!(_);
        let v2 = lterm!(_);
        let v3 = lterm!([1]);

        smap.extend(v1.clone(), v0.clone());
        smap.extend(v2.clone(), v3.clone());

        // v1 and v2 walk to different length lists with same values => failure
        let mut extension = SMap::new();
        assert!(matches!(unify_rec(state, &mut extension, &v1, &v2), Err(_)));
    }

    #[test]
    fn test_unify_12() {
        // Occurs check 1
        let state = State::<EmptyUser>::new(Default::default());
        let u = LTerm::var("u");
        let v = lterm!([1, 2, 3, u]);

        // term `v` cannot unify with `u`, because it contains `u`
        let mut extension = SMap::new();
        assert!(matches!(unify_rec(state, &mut extension, &u, &v), Err(_)));
    }

    #[test]
    fn test_unify_13() {
        // Occurs check 2
        let state = State::<EmptyUser>::new(Default::default());
        let u = LTerm::var("u");
        let v = lterm!([1, 2, 3, u]);

        // term `v` cannot unify with `u`, because it contains `u`
        let mut extension = SMap::new();
        assert!(matches!(unify_rec(state, &mut extension, &v, &u), Err(_)));
    }
}
