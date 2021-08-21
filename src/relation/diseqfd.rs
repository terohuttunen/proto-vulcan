use crate::engine::Engine;
/// Constrain disequality in finite domains
use crate::goal::{Goal, Solve};
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::state::{Constraint, FiniteDomain, SResult, State};
use crate::stream::Stream;
use crate::user::User;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug(bound="U: User"))]
pub struct DiseqFd<U, E>
where
    U: User,
    E: Engine<U>,
{
    u: LTerm<U, E>,
    v: LTerm<U, E>,
}

impl<U, E> DiseqFd<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(u: LTerm<U, E>, v: LTerm<U, E>) -> Goal<U, E> {
        Goal::new(DiseqFd { u, v })
    }
}

impl<U, E> Solve<U, E> for DiseqFd<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _engine: &mut E, state: State<U, E>) -> Stream<U, E> {
        let u = self.u.clone();
        let v = self.v.clone();
        match DiseqFdConstraint::new(u, v).run(state) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

/// Disequality relation for finite domains.
///
/// Note: The built-in syntax `x != y` does not work with finite domains.
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// use proto_vulcan::relation::diseqfd;
/// use proto_vulcan::relation::infd;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         |x, y| {
///             infd(x, &[1, 2]),
///             infd(y, &[2, 3]),
///             diseqfd(x, y),
///             q == [x, y],
///         }
///     });
///     let mut iter = query.run();
///     let mut expected = vec![lterm!([2, 3]), lterm!([1, 2]), lterm!([1, 3])];
///     iter.for_each(|x| {
///         let n = x.q.clone();
///         assert!(expected.contains(&n));
///         expected.retain(|y| &n != y);
///     });
///     assert_eq!(expected.len(), 0);
/// }
/// ```
pub fn diseqfd<U, E>(u: LTerm<U, E>, v: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    DiseqFd::new(u, v)
}

#[derive(Derivative)]
#[derivative(Debug(bound="U: User"))]
pub struct DiseqFdConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    u: LTerm<U, E>,
    v: LTerm<U, E>,
}

impl<U, E> DiseqFdConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(u: LTerm<U, E>, v: LTerm<U, E>) -> Rc<dyn Constraint<U, E>> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        Rc::new(DiseqFdConstraint { u, v })
    }
}

impl<U, E> Constraint<U, E> for DiseqFdConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn run(self: Rc<Self>, state: State<U, E>) -> SResult<U, E> {
        let smap = state.get_smap();
        let dstore = state.get_dstore();

        let u = self.u.clone();
        let uwalk = smap.walk(&u);
        let singleton_udomain;
        let maybe_udomain = match uwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(uwalk),
            LTermInner::Val(LValue::Number(u)) => {
                singleton_udomain = Rc::new(FiniteDomain::from(*u));
                Some(&singleton_udomain)
            }
            _ => None,
        };

        let v = self.v.clone();
        let vwalk = smap.walk(&v);
        let singleton_vdomain;
        let maybe_vdomain = match vwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(vwalk),
            LTermInner::Val(LValue::Number(v)) => {
                singleton_vdomain = Rc::new(FiniteDomain::from(*v));
                Some(&singleton_vdomain)
            }
            _ => None,
        };

        match (maybe_udomain, maybe_vdomain) {
            (Some(udomain), Some(vdomain)) if udomain.is_singleton() && vdomain.is_singleton() => {
                // Both variables have singleton domains. If values are same, the constraint
                // fails in the current state and is dropped; if the values are different, the constraint
                // succeeds and is dropped.
                if udomain.min() == vdomain.min() {
                    Err(())
                } else {
                    Ok(state)
                }
            }
            (Some(udomain), Some(vdomain)) if udomain.is_disjoint(vdomain.as_ref()) => {
                // When the domains are disjoint, the constraint can never be violated.
                // Constraint can be dropped.
                Ok(state)
            }
            (Some(udomain), Some(vdomain)) => {
                // The domains are not both singleton or disjoint. The constraints are kept
                // until they can be resolved into singleton, or until they become disjoint.
                let state = state.with_constraint(self);
                if udomain.is_singleton() {
                    state.process_domain(vwalk, Rc::new(vdomain.diff(udomain.as_ref()).ok_or(())?))
                } else if vdomain.is_singleton() {
                    state.process_domain(uwalk, Rc::new(udomain.diff(vdomain.as_ref()).ok_or(())?))
                } else {
                    Ok(state)
                }
            }
            _ => {
                // One or both of the variables do not yet have domains. Keep the constraint
                // for later.
                Ok(state.with_constraint(self))
            }
        }
    }

    fn operands(&self) -> Vec<LTerm<U, E>> {
        vec![self.u.clone(), self.v.clone()]
    }
}

impl<U, E> std::fmt::Display for DiseqFdConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

#[cfg(test)]
mod tests {
    use super::diseqfd;
    use crate::prelude::*;
    use crate::relation::infd::infd;

    #[test]
    fn test_diseqfd_1() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                infd(x, &[1, 2]),
                infd(y, &[2, 3]),
                infd([z, q], &[2, 4]),
                x == y,
                diseqfd(x, z),
                q == z,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 4);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_diseqfd_2() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                x == y,
                infd(y, &[2, 3]),
                diseqfd(x, z),
                infd([z, q], &[2, 4]),
                q == z,
                infd(x, &[1, 2]),
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 4);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_diseqfd_3() {
        let query = proto_vulcan_query!(|x, y| {
            infd(x, &[1, 2]),
            infd(y, &[2, 3]),
            x == y,
        });
        let mut iter = query.run();
        let result = iter.next().unwrap();
        assert_eq!(result.x, 2);
        assert_eq!(result.y, 2);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_diseqfd_4() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                infd([x, y, z], &[1, 2]),
                diseqfd(x, y),
                diseqfd(x, z),
                diseqfd(y, z),
            }
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }
}
