use crate::engine::Engine;
/// Less than or equal FD
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::solver::{Solve, Solver};
use crate::state::{Constraint, SResult, State};
use crate::stream::Stream;
use crate::user::User;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct LessThanOrEqualFd<U, E>
where
    U: User,
    E: Engine<U>,
{
    u: LTerm<U, E>,
    v: LTerm<U, E>,
}

impl<U, E> LessThanOrEqualFd<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new<G: AnyGoal<U, E>>(u: LTerm<U, E>, v: LTerm<U, E>) -> InferredGoal<U, E, G> {
        InferredGoal::new(G::dynamic(Rc::new(LessThanOrEqualFd { u, v })))
    }
}

impl<U, E> Solve<U, E> for LessThanOrEqualFd<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        match LessThanOrEqualFdConstraint::new(self.u.clone(), self.v.clone()).run(state) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

pub fn ltefd<U, E, G>(u: LTerm<U, E>, v: LTerm<U, E>) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    LessThanOrEqualFd::new(u, v)
}

// Finite Domain Constraints
#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"), Clone(bound = "U: User"))]
pub struct LessThanOrEqualFdConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    u: LTerm<U, E>,
    v: LTerm<U, E>,
}

impl<U, E> LessThanOrEqualFdConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(u: LTerm<U, E>, v: LTerm<U, E>) -> Rc<dyn Constraint<U, E>> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        Rc::new(LessThanOrEqualFdConstraint { u, v })
    }
}

impl<U, E> Constraint<U, E> for LessThanOrEqualFdConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn run(self: Rc<Self>, state: State<U, E>) -> SResult<U, E> {
        let smap = state.get_smap();
        let dstore = state.get_dstore();

        let uwalk = smap.walk(&self.u);
        let maybe_udomain = dstore.get(uwalk);

        let vwalk = smap.walk(&self.v);
        let maybe_vdomain = dstore.get(vwalk);

        match (maybe_udomain, maybe_vdomain) {
            (Some(udomain), Some(vdomain)) => {
                // Both variables of the constraints have assigned domains, we can evaluate
                // the constraint. The constraint implies that min(u) <= max(v).
                let vmax = vdomain.max();
                let umin = udomain.min();
                Ok(state
                    .process_domain(
                        &uwalk,
                        Rc::new(udomain.copy_before(|u| vmax < *u).ok_or(())?),
                    )?
                    .process_domain(
                        &vwalk,
                        Rc::new(vdomain.drop_before(|v| umin <= *v).ok_or(())?),
                    )?
                    .with_constraint(self))
            }
            (Some(udomain), None) if vwalk.is_number() => {
                // The variable `u` has an assigned domain, and variable `v` has been bound
                // to a number. After the number constraint has been applied to the domain,
                // the constraint is dropped.
                let v = vwalk.get_number().unwrap();
                Ok(state
                    .process_domain(&uwalk, Rc::new(udomain.copy_before(|u| v < *u).ok_or(())?))?)
            }
            (None, Some(vdomain)) if uwalk.is_number() => {
                // The variable `v` has an assigned domain, and variable `u` has been bound
                // to a number. After the number constraint has been applied to the domain,
                // the constraint is dropped.
                let u = uwalk.get_number().unwrap();
                Ok(state
                    .process_domain(&vwalk, Rc::new(vdomain.drop_before(|v| u <= *v).ok_or(())?))?)
            }
            (None, None) if uwalk.is_number() && vwalk.is_number() => {
                // Both variables are bound to numbers. Constraint is no longer needed if it
                // is not broken.
                let u = uwalk.get_number().unwrap();
                let v = vwalk.get_number().unwrap();
                if u <= v {
                    // Constraint was successful
                    Ok(state)
                } else {
                    // Constraint failed
                    Err(())
                }
            }
            _ => {
                // The variables do not yet have assigned domains, add constraint back to
                // the store waiting for the domains to be assigned later.
                Ok(state.with_constraint(self))
            }
        }
    }

    fn operands(&self) -> Vec<LTerm<U, E>> {
        vec![self.u.clone(), self.v.clone()]
    }
}

impl<U, E> std::fmt::Display for LessThanOrEqualFdConstraint<U, E>
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
    use super::ltefd;
    use crate::prelude::*;
    use crate::relation::infd::{infd, infdrange};

    #[test]
    fn test_ltefd_1() {
        let query = proto_vulcan_query!(|q| {
            infdrange(q, &(0..=10)),
            ltefd(q, 5),
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 0);
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 4);
        assert_eq!(iter.next().unwrap().q, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltefd_2() {
        let query = proto_vulcan_query!(|q| {
            |x| {
                infdrange([x, q], &(0..=10)),
                ltefd(x, 5),
                q == x,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 0);
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 4);
        assert_eq!(iter.next().unwrap().q, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltefd_3() {
        let query = proto_vulcan_query!(|q| {
            |x| {
                ltefd(x, 5),
                infdrange([x, q], &(0..=10)),
                q == x,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 0);
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 4);
        assert_eq!(iter.next().unwrap().q, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltefd_4() {
        let query = proto_vulcan_query!(|q| {
            |x, y| {
                q == [x, y],
                infd(x, &[1, 2, 3]),
                infd(y, &[0, 1, 2, 3, 4]),
                ltefd(x, y),
            }
        });
        let iter = query.run();
        let mut expected = vec![
            lterm!([1, 1]),
            lterm!([1, 2]),
            lterm!([1, 3]),
            lterm!([2, 2]),
            lterm!([1, 4]),
            lterm!([3, 3]),
            lterm!([3, 4]),
            lterm!([2, 3]),
            lterm!([2, 4]),
        ];
        iter.for_each(|x| {
            let n = x.q.clone();
            assert!(expected.contains(&n));
            expected.retain(|y| &n != y);
        });
        assert_eq!(expected.len(), 0);
    }
}
