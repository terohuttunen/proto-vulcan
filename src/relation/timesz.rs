use crate::engine::Engine;
/// Constrains u * v = w
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::solver::{Solve, Solver};
use crate::state::{Constraint, SResult, State};
use crate::stream::Stream;
use crate::user::User;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct TimesZ<U, E>
where
    U: User,
    E: Engine<U>,
{
    u: LTerm<U, E>,
    v: LTerm<U, E>,
    w: LTerm<U, E>,
}

impl<U, E> TimesZ<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new<G: AnyGoal<U, E>>(
        u: LTerm<U, E>,
        v: LTerm<U, E>,
        w: LTerm<U, E>,
    ) -> InferredGoal<U, E, G> {
        InferredGoal::new(G::dynamic(TimesZ { u, v, w }))
    }
}

impl<U, E> Solve<U, E> for TimesZ<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        match TimesZConstraint::new(self.u.clone(), self.v.clone(), self.w.clone()).run(state) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

pub fn timesz<U, E, G>(u: LTerm<U, E>, v: LTerm<U, E>, w: LTerm<U, E>) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    TimesZ::new(u, v, w)
}

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"), Clone(bound = "U: User"))]
pub struct TimesZConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    u: LTerm<U, E>,
    v: LTerm<U, E>,
    w: LTerm<U, E>,
}

impl<U, E> TimesZConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(u: LTerm<U, E>, v: LTerm<U, E>, w: LTerm<U, E>) -> Rc<dyn Constraint<U, E>> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        Rc::new(TimesZConstraint { u, v, w })
    }
}

impl<U, E> Constraint<U, E> for TimesZConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn run(self: Rc<Self>, mut state: State<U, E>) -> SResult<U, E> {
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
                if u * v == *w {
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
                    .extend(wwalk.clone(), LTerm::from(u * v));
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
                    .extend(vwalk.clone(), LTerm::from(w / u));
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
                    .extend(uwalk.clone(), LTerm::from(w / v));
                state.run_constraints()
            }
            (LTermInner::Var(_, _), LTermInner::Var(_, _), LTermInner::Val(LValue::Number(_)))
            | (LTermInner::Var(_, _), LTermInner::Val(LValue::Number(_)), LTermInner::Var(_, _))
            | (LTermInner::Val(LValue::Number(_)), LTermInner::Var(_, _), LTermInner::Var(_, _)) => {
                /* Not enough terms grounded to verify constraint. */
                Ok(state.with_constraint(self))
            }
            _ => {
                /* Some operands grounded to terms of invalid type. */
                Err(())
            }
        }
    }

    fn operands(&self) -> Vec<LTerm<U, E>> {
        vec![self.u.clone(), self.v.clone(), self.w.clone()]
    }
}

impl<U, E> std::fmt::Display for TimesZConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

#[cfg(test)]
mod test {
    use super::timesz;
    use crate::prelude::*;

    #[test]
    fn test_timesz_1() {
        let query = proto_vulcan_query!(|q| { timesz(4, 2, q) });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 8);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_timesz_2() {
        let query = proto_vulcan_query!(|q| {
            |r, p| {
                timesz(2, r, q),
                timesz(r, 10, p),
                p == 20,
            }
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 4);
        assert!(iter.next().is_none());
    }
}
