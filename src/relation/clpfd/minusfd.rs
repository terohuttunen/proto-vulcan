/// Constrains u - v = w finite domains
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::solver::{Solve, Solver};
use crate::state::{Constraint, FiniteDomain, SResult, State};
use crate::stream::Stream;
use std::rc::Rc;

#[derive(Debug)]
pub struct MinusFd {
    u: LTerm,
    v: LTerm,
    w: LTerm,
}

impl MinusFd {
    pub fn new<G: AnyGoal>(u: LTerm, v: LTerm, w: LTerm) -> InferredGoal<G> {
        InferredGoal::new(G::dynamic(Rc::new(MinusFd { u, v, w })))
    }
}

impl Solve for MinusFd {
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        match MinusFdConstraint::new(self.u.clone(), self.v.clone(), self.w.clone()).run(state) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

pub fn minusfd<G>(u: LTerm, v: LTerm, w: LTerm) -> InferredGoal<G>
where
    G: AnyGoal,
{
    MinusFd::new(u, v, w)
}

#[derive(Debug, Clone)]
pub struct MinusFdConstraint {
    u: LTerm,
    v: LTerm,
    w: LTerm,
}

impl MinusFdConstraint {
    pub fn new(u: LTerm, v: LTerm, w: LTerm) -> Rc<dyn Constraint> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        Rc::new(MinusFdConstraint { u, v, w })
    }
}

impl Constraint for MinusFdConstraint {
    fn run(self: Rc<Self>, state: State) -> SResult {
        let smap = state.get_smap();
        let dstore = state.get_dstore();

        let uwalk = smap.walk(&self.u);
        let singleton_udomain;
        let maybe_udomain = match uwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(uwalk),
            LTermInner::Val(LValue::Number(u)) => {
                singleton_udomain = FiniteDomain::from(*u);
                Some(&singleton_udomain as &dyn crate::state::DomainValue)
            }
            _ => None,
        };

        let vwalk = smap.walk(&self.v);
        let singleton_vdomain;
        let maybe_vdomain = match vwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(vwalk),
            LTermInner::Val(LValue::Number(v)) => {
                singleton_vdomain = FiniteDomain::from(*v);
                Some(&singleton_vdomain as &dyn crate::state::DomainValue)
            }
            _ => None,
        };

        let wwalk = smap.walk(&self.w);
        let singleton_wdomain;
        let maybe_wdomain = match wwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(wwalk),
            LTermInner::Val(LValue::Number(w)) => {
                singleton_wdomain = FiniteDomain::from(*w);
                Some(&singleton_wdomain as &dyn crate::state::DomainValue)
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
                // Extract finite domains - error if not finite domains
                let udomain_fd = crate::state::dstore::as_finite_domain(udomain).ok_or(())?;
                let vdomain_fd = crate::state::dstore::as_finite_domain(vdomain).ok_or(())?;
                let wdomain_fd = crate::state::dstore::as_finite_domain(wdomain).ok_or(())?;

                let umin = udomain_fd.min();
                let umax = udomain_fd.max();
                let vmin = vdomain_fd.min();
                let vmax = vdomain_fd.max();
                let wmin = wdomain_fd.min();
                let wmax = wdomain_fd.max();
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

    fn operands(&self) -> Vec<LTerm> {
        vec![self.u.clone(), self.v.clone(), self.w.clone()]
    }
}

impl std::fmt::Display for MinusFdConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}
