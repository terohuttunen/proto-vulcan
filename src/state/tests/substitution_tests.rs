//! Substitution map tests
//!
//! Tests for the substitution map (SMap) implementation,
//! covering variable substitution, walking, occurs check, and reification.

use crate::engine::DefaultEngine;
use crate::lterm::{LTerm, LTermInner};
use crate::state::substitution::SMap;
use crate::user::DefaultUser;

#[test]
fn test_smap_new() {
    let smap = SMap::<DefaultUser, DefaultEngine<DefaultUser>>::new();
    // A newly created SMap is empty
    assert!(smap.is_empty());
}

#[test]
fn test_smap_extend() {
    let mut smap = SMap::<DefaultUser, DefaultEngine<DefaultUser>>::new();
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
    let mut smap = SMap::<DefaultUser, DefaultEngine<DefaultUser>>::new();
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
    let mut smap = SMap::<DefaultUser, DefaultEngine<DefaultUser>>::new();
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
    let smap = SMap::<DefaultUser, DefaultEngine<DefaultUser>>::new();
    let v = lterm!(_);
    let w = smap.walk(&v);
    assert!(LTerm::ptr_eq(&v, &w));
}

#[test]
fn test_smap_walk_2() {
    // 2. Variable found => walked until no more variables: ends in last variable
    let mut smap = SMap::<DefaultUser, DefaultEngine<DefaultUser>>::new();
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
    let mut smap = SMap::<DefaultUser, DefaultEngine<DefaultUser>>::new();
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
    let mut smap = SMap::<DefaultUser, DefaultEngine<DefaultUser>>::new();
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
    let smap = SMap::<DefaultUser, DefaultEngine<DefaultUser>>::new();
    let v = lterm!(_);
    let w = smap.walk_star(&v);
    assert!(LTerm::ptr_eq(&v, &w));
}

#[test]
fn test_smap_walk_star_2() {
    // 2. Variable found => walked until no more variables: ends in last variable
    let mut smap = SMap::<DefaultUser, DefaultEngine<DefaultUser>>::new();
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
    let mut smap = SMap::<DefaultUser, DefaultEngine<DefaultUser>>::new();
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
    let mut smap = SMap::<DefaultUser, DefaultEngine<DefaultUser>>::new();
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
    let smap = SMap::<DefaultUser, DefaultEngine<DefaultUser>>::new();
    let v0 = lterm!(_);
    let v1 = lterm!(_);
    let v = LTerm::cons(v0.clone(), LTerm::singleton(v1.clone()));

    let r = smap.reify(&v);
    assert!(r.walk(&v0).is_var());
    assert!(r.walk(&v1).is_var());
}