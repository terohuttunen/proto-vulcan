/// Constrains u - v = w finite domains
use crate::goal::{Goal, Solve};
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::state::{Constraint, FiniteDomain, SResult, State};
use crate::stream::Stream;
use crate::user::User;
use std::rc::Rc;

#[derive(Debug)]
pub struct MinusFd<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
    w: LTerm<U>,
}

impl<U: User> MinusFd<U> {
    pub fn new(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Goal<U> {
        Goal::new(MinusFd { u, v, w })
    }
}

impl<U: User> Solve<U> for MinusFd<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let c = MinusFdConstraint::new(self.u.clone(), self.v.clone(), self.w.clone());
        Stream::from(c.run(state))
    }
}

pub fn minusfd<U: User>(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Goal<U> {
    MinusFd::new(u, v, w)
}

#[derive(Debug)]
pub struct MinusFdConstraint<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
    w: LTerm<U>,
}

impl<U: User> MinusFdConstraint<U> {
    pub fn new(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Rc<dyn Constraint<U>> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        Rc::new(MinusFdConstraint { u, v, w })
    }
}

impl<U: User> Constraint<U> for MinusFdConstraint<U> {
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
            if uwalk.get_number().unwrap() - vwalk.get_number().unwrap()
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
                // The constraint is: u - v = w  <=>  u = w + v  <=>  v = u - w
                //
                // Given domains for u and v, we can then deduce that the domain of w must be
                // in range [umin - vmax .. umax + vmin]. The constraining domain is built and
                // intersected with the current domain of w in .process_domain()-call.
                //
                // Same application of constraining domain is done for the other two variables.
                //   w = u - v  =>  [umin - vmax .. umax - vmin]
                //   u = w + v  =>  [wmin + vmin .. wmax + vmax]
                //   v = u - w  =>  [umin - wmax .. umax - wmin]
                //
                // The constraint is not dropped until all variables converge into numbers.
                Ok(state
                    .process_domain(
                        &wwalk,
                        Rc::new(FiniteDomain::from(
                            umin.saturating_sub(vmax)..=umax.saturating_sub(vmin),
                        )),
                    )?
                    .process_domain(
                        &uwalk,
                        Rc::new(FiniteDomain::from(
                            wmin.saturating_add(vmin)..=wmax.saturating_add(vmax),
                        )),
                    )?
                    .process_domain(
                        &vwalk,
                        Rc::new(FiniteDomain::from(
                            umin.saturating_sub(wmax)..=umax.saturating_sub(wmin),
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

impl<U: User> std::fmt::Display for MinusFdConstraint<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}
