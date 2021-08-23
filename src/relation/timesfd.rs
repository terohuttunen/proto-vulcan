use crate::engine::Engine;
/// Constrains u * v = w finite domains
use crate::goal::Goal;
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::solver::{Solve, Solver};
use crate::state::{Constraint, FiniteDomain, SResult, State};
use crate::stream::Stream;
use crate::user::User;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct TimesFd<U, E>
where
    U: User,
    E: Engine<U>,
{
    u: LTerm<U, E>,
    v: LTerm<U, E>,
    w: LTerm<U, E>,
}

impl<U, E> TimesFd<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(u: LTerm<U, E>, v: LTerm<U, E>, w: LTerm<U, E>) -> Goal<U, E> {
        Goal::dynamic(TimesFd { u, v, w })
    }
}

impl<U, E> Solve<U, E> for TimesFd<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        match TimesFdConstraint::new(self.u.clone(), self.v.clone(), self.w.clone()).run(state) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

pub fn timesfd<U, E>(u: LTerm<U, E>, v: LTerm<U, E>, w: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    TimesFd::new(u, v, w)
}

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct TimesFdConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    u: LTerm<U, E>,
    v: LTerm<U, E>,
    w: LTerm<U, E>,
}

impl<U, E> TimesFdConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(u: LTerm<U, E>, v: LTerm<U, E>, w: LTerm<U, E>) -> Rc<dyn Constraint<U, E>> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        Rc::new(TimesFdConstraint { u, v, w })
    }
}

impl<U, E> Constraint<U, E> for TimesFdConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn run(self: Rc<Self>, state: State<U, E>) -> SResult<U, E> {
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
            if uwalk.get_number().unwrap() * vwalk.get_number().unwrap()
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
                // The constraint is: u * v = w  <=>  u = w / v  <=>  v = w / u
                //
                // Given domains for u and v, we can then deduce that the domain of w must be
                // in range [umin - vmax .. umax + vmin]. The constraining domain is built and
                // intersected with the current domain of w in .process_domain()-call.
                //
                // Same application of constraining domain is done for the other two variables.
                //   w = u * v  =>  [umin * vmin .. umax * vmax]
                //   u = w / v  =>  [wmin / vmax .. wmax / vmin]
                //   v = w / u  =>  [wmin / umax .. wmax / umin]
                //
                // The constraint is not dropped until all variables converge into numbers.
                Ok(state
                    .process_domain(
                        &wwalk,
                        Rc::new(FiniteDomain::from(
                            umin.saturating_mul(vmin)..=umax.saturating_mul(vmax),
                        )),
                    )?
                    .process_domain(
                        &uwalk,
                        Rc::new(FiniteDomain::from(
                            wmin.checked_div(vmax).unwrap_or(umin)
                                ..=wmax.checked_div(vmin).unwrap_or(umax),
                        )),
                    )?
                    .process_domain(
                        &vwalk,
                        Rc::new(FiniteDomain::from(
                            wmin.checked_div(umax).unwrap_or(vmin)
                                ..=wmax.checked_div(umin).unwrap_or(vmax),
                        )),
                    )?
                    .with_constraint(self))
            }
            // If all operators do not yet have domains, then keep the constraint until it can
            // be used to constrain some domains.
            _ => Ok(state.with_constraint(self)),
        }
    }

    fn operands(&self) -> Vec<LTerm<U, E>> {
        vec![self.u.clone(), self.v.clone(), self.w.clone()]
    }
}

impl<U, E> std::fmt::Display for TimesFdConstraint<U, E>
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
    use super::timesfd;
    use crate::prelude::*;
    use crate::relation::infd::infdrange;

    #[test]
    fn test_timesfd_1() {
        let query = proto_vulcan_query!(|q| {
            |x, y| {
                infdrange([x, y], &(0..=6)),
                timesfd(x, y, 6),
                q == [x, y],
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([1, 6]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 3]));
        assert_eq!(iter.next().unwrap().q, lterm!([3, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([6, 1]));
        assert!(iter.next().is_none());
    }
}
