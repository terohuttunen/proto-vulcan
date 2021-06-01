use crate::engine::Engine;
/// Constrains u + v = w
use crate::goal::{Goal, Solve};
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::state::{Constraint, SResult, State};
use crate::stream::Stream;
use crate::user::User;
use std::rc::Rc;

#[derive(Debug)]
pub struct PlusZ<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
    w: LTerm<U>,
}

impl<U: User> PlusZ<U> {
    pub fn new<E: Engine<U>>(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Goal<U, E> {
        Goal::new(PlusZ { u, v, w })
    }
}

impl<U, E> Solve<U, E> for PlusZ<U>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _engine: &E, state: State<U>) -> Stream<U, E> {
        match PlusZConstraint::new(self.u.clone(), self.v.clone(), self.w.clone()).run(state) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

pub fn plusz<U, E>(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    PlusZ::new(u, v, w)
}

/// Sum
#[derive(Debug, Clone)]
pub struct PlusZConstraint<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
    w: LTerm<U>,
}

impl<U: User> PlusZConstraint<U> {
    pub fn new(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Rc<dyn Constraint<U>> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        Rc::new(PlusZConstraint { u, v, w })
    }
}

impl<U: User> Constraint<U> for PlusZConstraint<U> {
    fn run(self: Rc<Self>, mut state: State<U>) -> SResult<U> {
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

    fn operands(&self) -> Vec<LTerm<U>> {
        vec![self.u.clone(), self.v.clone(), self.w.clone()]
    }
}

impl<U: User> std::fmt::Display for PlusZConstraint<U> {
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
