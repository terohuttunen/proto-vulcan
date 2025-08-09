
/// Constrains u + v = w
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::solver::{Solve, Solver};
use crate::state::{Constraint, SResult, State};
use crate::stream::Stream;

use std::rc::Rc;

#[derive(Debug)]
pub struct PlusZ
{
    u: LTerm,
    v: LTerm,
    w: LTerm,
}

impl PlusZ
{
    pub fn new<G: AnyGoal>(
        u: LTerm,
        v: LTerm,
        w: LTerm,
    ) -> InferredGoal<G> {
        InferredGoal::new(G::dynamic(Rc::new(PlusZ { u, v, w })))
    }
}

impl Solve for PlusZ
{
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        match PlusZConstraint::new(self.u.clone(), self.v.clone(), self.w.clone()).run(state) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

pub fn plusz<G>(u: LTerm, v: LTerm, w: LTerm) -> InferredGoal<G>
where
    G: AnyGoal,
{
    PlusZ::new(u, v, w)
}

/// Sum
#[derive(Debug)]
pub struct PlusZConstraint
{
    u: LTerm,
    v: LTerm,
    w: LTerm,
}

impl PlusZConstraint
{
    pub fn new(u: LTerm, v: LTerm, w: LTerm) -> Rc<dyn Constraint> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        Rc::new(PlusZConstraint { u, v, w })
    }
}

impl Constraint for PlusZConstraint {
    fn run(self: Rc<Self>, mut state: State) -> SResult {
        let uwalk = state.smap_ref().walk(&self.u).clone();
        let vwalk = state.smap_ref().walk(&self.v).clone();
        let wwalk = state.smap_ref().walk(&self.w).clone();

        match (uwalk.as_ref(), vwalk.as_ref(), wwalk.as_ref()) {
            (
                LTermInner::Val(LValue::Number(u)),
                LTermInner::Val(LValue::Number(v)),
                LTermInner::Val(LValue::Number(w)),
            ) => {
                /* All operands grounded. */
                if u + v == *w {
                    Ok(state)
                } else {
                    Err(())
                }
            }
            (
                LTermInner::Val(LValue::Number(u)),
                LTermInner::Val(LValue::Number(v)),
                LTermInner::Var(_, _),
            ) => {
                /* u and v grounded */
                state
                    .smap_to_mut()
                    .extend(wwalk.clone(), LTerm::from(u + v));
                state.run_constraints()
            }
            (
                LTermInner::Val(LValue::Number(u)),
                LTermInner::Var(_, _),
                LTermInner::Val(LValue::Number(w)),
            ) => {
                /* u and w grounded */
                state
                    .smap_to_mut()
                    .extend(vwalk.clone(), LTerm::from(w - u));
                state.run_constraints()
            }
            (
                LTermInner::Var(_, _),
                LTermInner::Val(LValue::Number(v)),
                LTermInner::Val(LValue::Number(w)),
            ) => {
                /* v and w grounded */
                state
                    .smap_to_mut()
                    .extend(uwalk.clone(), LTerm::from(w - v));
                state.run_constraints()
            }
            (LTermInner::Var(_, _), LTermInner::Var(_, _), LTermInner::Val(LValue::Number(_)))
            | (LTermInner::Var(_, _), LTermInner::Val(LValue::Number(_)), LTermInner::Var(_, _))
            | (LTermInner::Val(LValue::Number(_)), LTermInner::Var(_, _), LTermInner::Var(_, _))
            | (LTermInner::Var(_, _), LTermInner::Var(_, _), LTermInner::Var(_, _)) => {
                /* Not enough terms grounded to verify constraint. */
                Ok(state.with_constraint(self))
            }
            _ => {
                /* Some operands grounded to terms of invalid type. */
                Err(())
            }
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.u.clone(), self.v.clone(), self.w.clone()]
    }
}

impl std::fmt::Display for PlusZConstraint
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

#[cfg(test)]
mod test {
    use super::plusz;
    use crate::prelude::*;

    #[test]
    fn test_plusz_1() {
        let query = proto_vulcan_query!(|q| { plusz(0, 1, q) });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_plusz_2() {
        let query = proto_vulcan_query!(|q| {
            |r, p| {
                plusz(1, r, q),
                plusz(r, 10, p),
                p == 15,
            }
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 6);
        assert!(iter.next().is_none());
    }
}
