use crate::engine::Engine;
/// Constrains u + v = w finite domains
use crate::goal::{Goal, Solve};
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::state::{Constraint, FiniteDomain, SResult, State};
use crate::stream::Stream;
use crate::user::User;
use std::rc::Rc;

#[derive(Debug)]
pub struct PlusFd<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
    w: LTerm<U>,
}

impl<U: User> PlusFd<U> {
    pub fn new<E: Engine<U>>(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Goal<U, E> {
        Goal::new(PlusFd { u, v, w })
    }
}

impl<U, E> Solve<U, E> for PlusFd<U>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _engine: &E, state: State<U>) -> Stream<U, E> {
        match PlusFdConstraint::new(self.u.clone(), self.v.clone(), self.w.clone()).run(state) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

pub fn plusfd<U, E>(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    PlusFd::new(u, v, w)
}

#[derive(Debug)]
pub struct PlusFdConstraint<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
    w: LTerm<U>,
}

impl<U: User> PlusFdConstraint<U> {
    pub fn new(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Rc<dyn Constraint<U>> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        Rc::new(PlusFdConstraint { u, v, w })
    }
}

impl<U: User> Constraint<U> for PlusFdConstraint<U> {
    fn run(self: Rc<Self>, state: State<U>) -> SResult<U> {
        let smap = state.get_smap();
        let dstore = state.get_dstore();

        let uwalk = smap.walk(&self.u);
        let singleton_udomain;
        let maybe_udomain = match uwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(uwalk),
            LTermInner::Val(LValue::Number(u)) => {
                singleton_udomain = Rc::new(FiniteDomain::from(*u));
                Some(&singleton_udomain)
            }
            _ => None,
        };

        let vwalk = smap.walk(&self.v);
        let singleton_vdomain;
        let maybe_vdomain = match vwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(vwalk),
            LTermInner::Val(LValue::Number(v)) => {
                singleton_vdomain = Rc::new(FiniteDomain::from(*v));
                Some(&singleton_vdomain)
            }
            _ => None,
        };

        let wwalk = smap.walk(&self.w);
        let singleton_wdomain;
        let maybe_wdomain = match wwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(wwalk),
            LTermInner::Val(LValue::Number(w)) => {
                singleton_wdomain = Rc::new(FiniteDomain::from(*w));
                Some(&singleton_wdomain)
            }
            _ => None,
        };

        // If all operators are bound to numbers, then we can drop the constraint or fail if
        // constraint is not fulfilled.
        if uwalk.is_number() && vwalk.is_number() && wwalk.is_number() {
            if uwalk.get_number().unwrap() + vwalk.get_number().unwrap()
                == wwalk.get_number().unwrap()
            {
                return Ok(state);
            } else {
                return Err(());
            }
        }

        match (maybe_udomain, maybe_vdomain, maybe_wdomain) {
            (Some(udomain), Some(vdomain), Some(wdomain)) => {
                let umin = udomain.min();
                let umax = udomain.max();
                let vmin = vdomain.min();
                let vmax = vdomain.max();
                let wmin = wdomain.min();
                let wmax = wdomain.max();
                // The constraint is: u + v = w
                //
                // Given domains for u and v, we can then deduce that the domain of w must be
                // in range [umin + vmin .. umax + vmax]. The constraining domain is built and
                // intersected with the current domain of w in .process_domain()-call.
                //
                // Same application of constraining domain is done for the other two variables.
                //
                // The constraint is not dropped until all variables converge into numbers.
                Ok(state
                    .process_domain(
                        &wwalk,
                        Rc::new(FiniteDomain::from(
                            umin.saturating_add(vmin)..=umax.saturating_add(vmax),
                        )),
                    )?
                    .process_domain(
                        &uwalk,
                        Rc::new(FiniteDomain::from(
                            wmin.saturating_sub(vmax)..=wmax.saturating_sub(vmin),
                        )),
                    )?
                    .process_domain(
                        &vwalk,
                        Rc::new(FiniteDomain::from(
                            wmin.saturating_sub(umax)..=wmax.saturating_sub(umin),
                        )),
                    )?
                    .with_constraint(self))
            }
            // If all operators do not yet have domains, then keep the constraint until it can
            // be used to constrain some domains.
            _ => Ok(state.with_constraint(self)),
        }
    }

    fn operands(&self) -> Vec<LTerm<U>> {
        vec![self.u.clone(), self.v.clone(), self.w.clone()]
    }
}

impl<U: User> std::fmt::Display for PlusFdConstraint<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

#[cfg(test)]
mod tests {
    use super::plusfd;
    use crate::prelude::*;
    use crate::relation::diseqfd::diseqfd;
    use crate::relation::infd::infdrange;

    #[test]
    fn test_plusfd_1() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                infdrange([x, y, z, q], #&(0..=9)),
                diseqfd(x, y),
                diseqfd(y, z),
                diseqfd(x, z),
                x == 2,
                q == 3,
                plusfd(y, 3, z),
            }
        });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 3);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_plusfd_2() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                q == [x, y, z],
                infdrange([x, y, z], #&(0..=3)),
                plusfd(x, y, z),
            }
        });
        let iter = query.run();
        let mut expected = vec![
            lterm!([0, 0, 0]),
            lterm!([0, 1, 1]),
            lterm!([0, 2, 2]),
            lterm!([1, 0, 1]),
            lterm!([0, 3, 3]),
            lterm!([3, 0, 3]),
            lterm!([1, 1, 2]),
            lterm!([1, 2, 3]),
            lterm!([2, 0, 2]),
            lterm!([2, 1, 3]),
        ];
        iter.for_each(|x| {
            let n = x.q.clone();
            assert!(expected.contains(&n));
            expected.retain(|y| &n != y);
        });
        assert_eq!(expected.len(), 0);
    }
}
