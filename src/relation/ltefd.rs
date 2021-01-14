/// Less than or equal FD
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::{Constraint, SResult, State};
use crate::stream::Stream;
use crate::user::User;
use std::rc::Rc;

#[derive(Debug)]
pub struct LessThanOrEqualFd<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
}

impl<U: User> LessThanOrEqualFd<U> {
    pub fn new(u: LTerm<U>, v: LTerm<U>) -> Goal<U> {
        Goal::new(LessThanOrEqualFd { u, v })
    }
}

impl<U: User> Solve<U> for LessThanOrEqualFd<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let c = LessThanOrEqualFdConstraint::new(self.u.clone(), self.v.clone());
        Stream::from(c.run(state))
    }
}

pub fn ltefd<U: User>(u: LTerm<U>, v: LTerm<U>) -> Goal<U> {
    LessThanOrEqualFd::new(u, v)
}

// Finite Domain Constraints
#[derive(Debug, Clone)]
pub struct LessThanOrEqualFdConstraint<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
}

impl<U: User> LessThanOrEqualFdConstraint<U> {
    pub fn new(u: LTerm<U>, v: LTerm<U>) -> Rc<dyn Constraint<U>> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        Rc::new(LessThanOrEqualFdConstraint { u, v })
    }
}

impl<U: User> Constraint<U> for LessThanOrEqualFdConstraint<U> {
    fn run(self: Rc<Self>, state: State<U>) -> SResult<U> {
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

    fn operands(&self) -> Vec<LTerm<U>> {
        vec![self.u.clone(), self.v.clone()]
    }
}

impl<U: User> std::fmt::Display for LessThanOrEqualFdConstraint<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

#[cfg(test)]
mod tests {
    use super::ltefd;
    use crate::relation::infd::{infd, infdrange};
    use crate::*;

    #[test]
    fn test_ltefd_1() {
        let query = proto_vulcan_query!(|q| {
            infdrange(q, #&(0..=10)),
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
                infdrange([x, q], #&(0..=10)),
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
                infdrange([x, q], #&(0..=10)),
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
                infd(x, #&[1, 2, 3]),
                infd(y, #&[0, 1, 2, 3, 4]),
                ltefd(x, y),
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([1, 1]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 3]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 4]));
        assert_eq!(iter.next().unwrap().q, lterm!([3, 3]));
        assert_eq!(iter.next().unwrap().q, lterm!([3, 4]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 3]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 4]));
        assert!(iter.next().is_none());
    }
}
