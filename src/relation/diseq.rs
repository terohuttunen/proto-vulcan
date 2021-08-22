use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::solver::{Solve, Solver};
use crate::state::{unify_rec, Constraint, SMap, SResult, State};
use crate::stream::Stream;
use crate::user::User;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct Diseq<U, E>
where
    U: User,
    E: Engine<U>,
{
    u: LTerm<U, E>,
    v: LTerm<U, E>,
}

impl<U, E> Diseq<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(u: LTerm<U, E>, v: LTerm<U, E>) -> Goal<U, E> {
        Goal::new(Diseq { u, v })
    }
}

impl<U, E> Solve<U, E> for Diseq<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        // Return state where u and v are unified under s, or None if unification is not possible
        match state.disunify(&self.u, &self.v) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

/// Disequality relation.
///
/// The disequality relation adds a disequality constraint. Proto-vulcan provides a built-in
/// syntax `x != y` that avoids adding the use-clause: `use proto_vulcan::relation::diseq`.
///
/// Note: currently this is only tree-disequality. For finite-domain disequality, diseqfd-relation
/// must be used instead.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// fn main() {
///     let query = proto_vulcan_query!(|x, y| {
///         [x, 1] != [2, y],
///     });
///     let mut iter = query.run();
///     let result = iter.next().unwrap();
///     assert!(result.x.is_any_except(&2));
///     assert!(result.y.is_any_except(&1));
///     assert!(iter.next().is_none());
/// }
/// ```
pub fn diseq<U, E>(u: LTerm<U, E>, v: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Diseq::new(u, v)
}

// Disequality constraint
#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"), Clone(bound = "U: User"))]
pub struct DisequalityConstraint<U: User, E: Engine<U>>(SMap<U, E>);

impl<U, E> DisequalityConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(smap: SMap<U, E>) -> Rc<dyn Constraint<U, E>> {
        Rc::new(DisequalityConstraint(smap))
    }

    /// If the `self` subsumes the `other`.
    ///
    /// A constraint is subsumed by another constraint if unifying the constraint in the
    /// substitution of the another constraint does not extend the constraint.
    pub fn subsumes(&self, other: &dyn Constraint<U, E>) -> bool {
        match other.downcast_ref::<Self>() {
            Some(other) => {
                let mut extension = SMap::new();
                let mut state = State::new(Default::default()).with_smap(other.smap_ref().clone());
                for (u, v) in self.0.iter() {
                    match unify_rec(state, &mut extension, &u, &v) {
                        Err(()) => return false,
                        Ok(s) => state = s,
                    }
                }

                extension.is_empty()
            }
            None => false,
        }
    }

    pub fn smap_ref(&self) -> &SMap<U, E> {
        &self.0
    }

    pub fn walk_star(&self, smap: &SMap<U, E>) -> SMap<U, E> {
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

impl<U, E> Constraint<U, E> for DisequalityConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn run(self: Rc<Self>, state: State<U, E>) -> SResult<U, E> {
        let mut extension = SMap::new();
        let mut test_state = state.clone();
        for (u, v) in self.0.iter() {
            match unify_rec(test_state, &mut extension, &u, &v) {
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

    fn operands(&self) -> Vec<LTerm<U, E>> {
        self.0.operands()
    }
}

impl<U, E> std::fmt::Display for DisequalityConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
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
    use crate::engine::DefaultEngine;
    use crate::prelude::*;

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
            c0.downcast_ref::<DisequalityConstraint<DefaultUser, DefaultEngine<DefaultUser>>>(),
            c1.downcast_ref::<DisequalityConstraint<DefaultUser, DefaultEngine<DefaultUser>>>(),
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
}
