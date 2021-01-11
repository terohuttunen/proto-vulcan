/// CLP(Z)
use super::{BaseConstraint, Constraint, ZConstraint};
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::state::{SResult, State, User};
use std::rc::Rc;

/// Product
#[derive(Debug, Clone)]
pub struct TimesZConstraint<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
    w: LTerm<U>,
}

impl<U: User> TimesZConstraint<U> {
    pub fn new(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Constraint<U> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        Constraint::Z(Rc::new(TimesZConstraint { u, v, w }))
    }
}

impl<U: User> BaseConstraint<U> for TimesZConstraint<U> {
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

    fn operands(&self) -> Vec<LTerm<U>> {
        vec![self.u.clone(), self.v.clone(), self.w.clone()]
    }
}

impl<U: User> std::fmt::Display for TimesZConstraint<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

impl<U: User> ZConstraint<U> for TimesZConstraint<U> {}

impl<U: User> From<Rc<TimesZConstraint<U>>> for Constraint<U> {
    fn from(c: Rc<TimesZConstraint<U>>) -> Constraint<U> {
        Constraint::Z(c as Rc<dyn ZConstraint<U>>)
    }
}

/// Sum
#[derive(Debug, Clone)]
pub struct PlusZConstraint<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
    w: LTerm<U>,
}

impl<U: User> PlusZConstraint<U> {
    pub fn new(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Constraint<U> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        Constraint::Z(Rc::new(PlusZConstraint { u, v, w }))
    }
}

impl<U: User> BaseConstraint<U> for PlusZConstraint<U> {
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

impl<U: User> ZConstraint<U> for PlusZConstraint<U> {}

impl<U: User> From<Rc<PlusZConstraint<U>>> for Constraint<U> {
    fn from(c: Rc<PlusZConstraint<U>>) -> Constraint<U> {
        Constraint::Z(c as Rc<dyn ZConstraint<U>>)
    }
}
