//! Disequality constraint tests
//!
//! Tests for the CLP(Tree) disequality constraint implementation,
//! covering constraint subsumption and various disequality scenarios.


use crate::prelude::*;
use crate::relation::diseq::DisequalityConstraint;
use crate::state::SMap;

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
        c0.downcast_ref::<DisequalityConstraint>(),
        c1.downcast_ref::<DisequalityConstraint>(),
    ) {
        (Some(t0), Some(t1)) => {
            assert!(t1.subsumes(&*t0))
        }
        _ => assert!(false),
    }
}

#[test]
fn test_diseq_1() {
    let query = proto_vulcan_query!(|q| {
        3 != q,
        q == 3,
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_2() {
    let query = proto_vulcan_query!(|q| {
        q == 3,
        3 != q,
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_3() {
    let query = proto_vulcan_query!(|q| {
        |x, y| {
            x != y,
            x == y,
            x == q,
        }
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_4() {
    let query = proto_vulcan_query!(|q| {
        |x, y| {
            x == y,
            x != y,
            x == q,
        }
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_5() {
    let query = proto_vulcan_query!(|q| {
        |x, y| {
            x != y,
            3 == x,
            3 == y,
            x == q,
        }
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_6() {
    let query = proto_vulcan_query!(|q| {
        |x, y| {
            3 == x,
            x != y,
            3 == y,
            x == q,
        }
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_7() {
    let query = proto_vulcan_query!(|q| {
        |x, y| {
            3 == x,
            3 == y,
            x != y,
            x == q,
        }
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_8() {
    let query = proto_vulcan_query!(|q| {
        |x, y| {
            3 == x,
            3 == y,
            y != x,
            x == q,
        }
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_9() {
    let query = proto_vulcan_query!(|q| {
        |x, y, z| {
            x != y,
            x == [0, z, 1],
            y == [0, 1, 1],
            z == 1,
            [x, y] == q,
        }
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_10() {
    let query = proto_vulcan_query!(|q| {
        |x, y, z| {
            x != y,
            x == [0, z, 1],
            y == [0, 1, 1],
        }
    });
    let mut iter = query.run();
    let result = iter.next().unwrap();
    assert!(result.q.is_any() && !result.q.is_constrained());
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_11() {
    let query = proto_vulcan_query!(|q| {
        |x, y, z| {
            x != y,
            x == [0, z, 1],
            y == [0, 1, 1],
            z == 0,
        }
    });
    let mut iter = query.run();
    let result = iter.next().unwrap();
    assert!(result.q.is_any() && !result.q.is_constrained());
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_12() {
    let query = proto_vulcan_query!(|q| {
        |x, y, z| {
            z == 0,
            x != y,
            x == [0, z, 1],
            y == [0, 1, 1],
        }
    });
    let mut iter = query.run();
    let result = iter.next().unwrap();
    assert!(result.q.is_any() && !result.q.is_constrained());
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_13() {
    let query = proto_vulcan_query!(|q| {
        |x, y, z| {
            x == [0, z, 1],
            y == [0, 1, 1],
            x != y,
        }
    });
    let mut iter = query.run();
    let result = iter.next().unwrap();
    assert!(result.q.is_any() && !result.q.is_constrained());
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_14() {
    let query = proto_vulcan_query!(|q| {
        |x, y, z| {
            z == 1,
            x != y,
            x == [0, z, 1],
            y == [0, 1, 1],
        }
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_15() {
    let query = proto_vulcan_query!(|q| {
        |x, y, z| {
            z == 1,
            x == [0, z, 1],
            y == [0, 1, 1],
            x != y,
        }
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_16() {
    let query = proto_vulcan_query!(|q| {
        |x, y| {
            [x, 1] != [2, y],
            x == 2,
        }
    });
    let mut iter = query.run();
    let result = iter.next().unwrap();
    assert!(result.q.is_any() && !result.q.is_constrained());
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_17() {
    let query = proto_vulcan_query!(|q| {
        |x, y| {
            [x, 1] != [2, y],
            y == 1,
        }
    });
    let mut iter = query.run();
    let result = iter.next().unwrap();
    assert!(result.q.is_any() && !result.q.is_constrained());
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_18() {
    let query = proto_vulcan_query!(|q| {
        |x, y| {
            [x, 1] != [2, y],
            x == 2,
            y == 1,
        }
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_19() {
    let query = proto_vulcan_query!(|x, y| {
        [x, 1] != [2, y],
    });

    let mut iter = query.run();
    let result = iter.next().unwrap();
    assert!(result.x.is_any_except(&2));
    assert!(result.y.is_any_except(&1));
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_20() {
    let query = proto_vulcan_query!(|x, y| {
        [x, 1] != [2, y],
        x == 2,
    });
    let mut iter = query.run();
    let result = iter.next().unwrap();
    assert!(result.x == 2);
    assert!(result.y.is_any_except(&1));
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_21() {
    let query = proto_vulcan_query!(|x, y| {
        [x, 1] != [2, y],
        x == 2,
        y == 9,
    });
    let mut iter = query.run();
    let result = iter.next().unwrap();
    assert!(result.x == 2);
    assert!(result.y == 9);
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_22() {
    let query = proto_vulcan_query!(|q| {
        |a, d| {
            [a | d] == q,
            q != [5 | 6],
            a == 5,
            d == 6,
        }
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_23() {
    let query = proto_vulcan_query!(|q| {
        |x, y| {
            [x, 1] != [2, y],
            x == 2,
            y == 1,
            [x, y] == q,
        }
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_24() {
    let query = proto_vulcan_query!(|q| {
        |a, x, z| {
            a != [x, 1],
            a == [z, 1],
            x == z,
        }
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_25() {
    let query = proto_vulcan_query!(|x, z| {
        |a| {
            a != [x, 1],
            a == [z, 1],
            x == 5,
        }
    });
    let mut iter = query.run();
    let result = iter.next().unwrap();
    assert!(result.x == 5);
    assert!(result.z.is_any_except(&5));
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_26() {
    let query = proto_vulcan_query!(|q| {
        3 != 4,
    });
    let mut iter = query.run();
    let result = iter.next().unwrap();
    assert!(result.q.is_any());
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_27() {
    let query = proto_vulcan_query!(|q| {
        3 != 3,
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_28() {
    let query = proto_vulcan_query!(|q| {
        5 != q,
        6 != q,
        q == 5,
    });
    let mut iter = query.run();
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_29() {
    let query = proto_vulcan_query!(|a, d| {
        |q| {
            [a | d] == q,
            q != [5 | 6],
            a == 5,
        }
    });
    let mut iter = query.run();
    let result = iter.next().unwrap();
    assert!(result.a == 5);
    assert!(result.d.is_any_except(&6));
    assert!(iter.next().is_none());
}

#[test]
fn test_diseq_30() {
    let query = proto_vulcan_query!(|q| {
        |a| {
            3 == a,
            a != 4,
        }
    });
    let mut iter = query.run();
    let result = iter.next().unwrap();
    assert!(result.q.is_any());
    assert!(iter.next().is_none());
}