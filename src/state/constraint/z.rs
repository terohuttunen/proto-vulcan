/// CLP(Z)
use super::{BaseConstraint, Constraint, ZConstraint};
use crate::lterm::LTerm;
use crate::lvalue::LValue;
use crate::state::{SResult, State, UserState};
use std::rc::Rc;

/// Product
#[derive(Debug, Clone)]
pub struct TimesZConstraint {
    u: Rc<LTerm>,
    v: Rc<LTerm>,
    w: Rc<LTerm>,
}

impl TimesZConstraint {
    pub fn new(u: Rc<LTerm>, v: Rc<LTerm>, w: Rc<LTerm>) -> TimesZConstraint {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        TimesZConstraint { u, v, w }
    }
}

impl<U: UserState> BaseConstraint<U> for TimesZConstraint {
    fn run(self: Rc<Self>, mut state: State<U>) -> SResult<U> {
        let uwalk = Rc::clone(state.smap_ref().walk(&self.u));
        let vwalk = Rc::clone(state.smap_ref().walk(&self.v));
        let wwalk = Rc::clone(state.smap_ref().walk(&self.w));

        match (uwalk.as_ref(), vwalk.as_ref(), wwalk.as_ref()) {
            (
                LTerm::Val(LValue::Number(u)),
                LTerm::Val(LValue::Number(v)),
                LTerm::Val(LValue::Number(w)),
            ) => {
                /* All operands grounded. */
                if u * v == *w {
                    Ok(state)
                } else {
                    Err(())
                }
            }
            (LTerm::Val(LValue::Number(u)), LTerm::Val(LValue::Number(v)), LTerm::Var(_, _)) => {
                /* u and v grounded */
                state
                    .smap_to_mut()
                    .extend(Rc::clone(&wwalk), Rc::new(LTerm::from(u * v)));
                state.run_constraints()
            }
            (LTerm::Val(LValue::Number(u)), LTerm::Var(_, _), LTerm::Val(LValue::Number(w))) => {
                /* u and w grounded */
                state
                    .smap_to_mut()
                    .extend(Rc::clone(&vwalk), Rc::new(LTerm::from(w / u)));
                state.run_constraints()
            }
            (LTerm::Var(_, _), LTerm::Val(LValue::Number(v)), LTerm::Val(LValue::Number(w))) => {
                /* v and w grounded */
                state
                    .smap_to_mut()
                    .extend(Rc::clone(&uwalk), Rc::new(LTerm::from(w / v)));
                state.run_constraints()
            }
            (LTerm::Var(_, _), LTerm::Var(_, _), LTerm::Val(LValue::Number(_)))
            | (LTerm::Var(_, _), LTerm::Val(LValue::Number(_)), LTerm::Var(_, _))
            | (LTerm::Val(LValue::Number(_)), LTerm::Var(_, _), LTerm::Var(_, _)) => {
                /* Not enough terms grounded to verify constraint. */
                Ok(state.with_constraint(self))
            }
            _ => {
                /* Some operands grounded to terms of invalid type. */
                Err(())
            }
        }
    }

    fn operands(&self) -> Vec<&Rc<LTerm>> {
        vec![&self.u, &self.v, &self.w]
    }
}

impl std::fmt::Display for TimesZConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

impl<U: UserState> ZConstraint<U> for TimesZConstraint {}

impl<U: UserState> From<Rc<TimesZConstraint>> for Constraint<U> {
    fn from(c: Rc<TimesZConstraint>) -> Constraint<U> {
        Constraint::Z(c as Rc<dyn ZConstraint<U>>)
    }
}

/// Sum
#[derive(Debug, Clone)]
pub struct PlusZConstraint {
    u: Rc<LTerm>,
    v: Rc<LTerm>,
    w: Rc<LTerm>,
}

impl PlusZConstraint {
    pub fn new(u: Rc<LTerm>, v: Rc<LTerm>, w: Rc<LTerm>) -> PlusZConstraint {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        PlusZConstraint { u, v, w }
    }
}

impl<U: UserState> BaseConstraint<U> for PlusZConstraint {
    fn run(self: Rc<Self>, mut state: State<U>) -> SResult<U> {
        let uwalk = Rc::clone(state.smap_ref().walk(&self.u));
        let vwalk = Rc::clone(state.smap_ref().walk(&self.v));
        let wwalk = Rc::clone(state.smap_ref().walk(&self.w));

        match (uwalk.as_ref(), vwalk.as_ref(), wwalk.as_ref()) {
            (
                LTerm::Val(LValue::Number(u)),
                LTerm::Val(LValue::Number(v)),
                LTerm::Val(LValue::Number(w)),
            ) => {
                /* All operands grounded. */
                if u * v == *w {
                    Ok(state)
                } else {
                    Err(())
                }
            }
            (LTerm::Val(LValue::Number(u)), LTerm::Val(LValue::Number(v)), LTerm::Var(_, _)) => {
                /* u and v grounded */
                state
                    .smap_to_mut()
                    .extend(Rc::clone(&wwalk), Rc::new(LTerm::from(u + v)));
                state.run_constraints()
            }
            (LTerm::Val(LValue::Number(u)), LTerm::Var(_, _), LTerm::Val(LValue::Number(w))) => {
                /* u and w grounded */
                state
                    .smap_to_mut()
                    .extend(Rc::clone(&vwalk), Rc::new(LTerm::from(w - u)));
                state.run_constraints()
            }
            (LTerm::Var(_, _), LTerm::Val(LValue::Number(v)), LTerm::Val(LValue::Number(w))) => {
                /* v and w grounded */
                state
                    .smap_to_mut()
                    .extend(Rc::clone(&uwalk), Rc::new(LTerm::from(w - v)));
                state.run_constraints()
            }
            (LTerm::Var(_, _), LTerm::Var(_, _), LTerm::Val(LValue::Number(_)))
            | (LTerm::Var(_, _), LTerm::Val(LValue::Number(_)), LTerm::Var(_, _))
            | (LTerm::Val(LValue::Number(_)), LTerm::Var(_, _), LTerm::Var(_, _)) => {
                /* Not enough terms grounded to verify constraint. */
                Ok(state.with_constraint(self))
            }
            _ => {
                /* Some operands grounded to terms of invalid type. */
                Err(())
            }
        }
    }

    fn operands(&self) -> Vec<&Rc<LTerm>> {
        vec![&self.u, &self.v, &self.w]
    }
}

impl std::fmt::Display for PlusZConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

impl<U: UserState> ZConstraint<U> for PlusZConstraint {}

impl<U: UserState> From<Rc<PlusZConstraint>> for Constraint<U> {
    fn from(c: Rc<PlusZConstraint>) -> Constraint<U> {
        Constraint::Z(c as Rc<dyn ZConstraint<U>>)
    }
}
