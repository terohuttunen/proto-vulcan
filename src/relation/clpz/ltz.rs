use crate::engine::Engine;
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::solver::{Solve, Solver};
use crate::state::{Constraint, SResult, State};
use crate::stream::Stream;
use crate::user::User;
use derivative::Derivative;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

/// CLPZ Less-than constraint: u < v for integers  
#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct LessZConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    u: LTerm<U, E>,
    v: LTerm<U, E>,
}

impl<U, E> LessZConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(u: LTerm<U, E>, v: LTerm<U, E>) -> Rc<Self> {
        Rc::new(LessZConstraint { u, v })
    }
}

impl<U, E> Constraint<U, E> for LessZConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn run(self: Rc<Self>, state: State<U, E>) -> SResult<U, E> {
        // Walk the terms to get their current values
        let u_walk = state.smap_ref().walk(&self.u).clone();
        let v_walk = state.smap_ref().walk(&self.v).clone();

        // Pattern match like other CLPZ constraints to handle all cases
        match (u_walk.as_ref(), v_walk.as_ref()) {
            (LTermInner::Val(LValue::Number(u_num)), LTermInner::Val(LValue::Number(v_num))) => {
                // Both are ground integers - check constraint immediately
                if u_num < v_num {
                    Ok(state) // Constraint satisfied, drop it
                } else {
                    Err(()) // Constraint violated, fail
                }
            }
            (LTermInner::Val(LValue::Number(_)), LTermInner::Var(_, _))
            | (LTermInner::Var(_, _), LTermInner::Val(LValue::Number(_))) => {
                // One ground, one variable - keep constraint for re-evaluation when variable is bound
                let walked_constraint = LessZConstraint::new(u_walk, v_walk);
                Ok(state.with_constraint(walked_constraint))
            }
            (LTermInner::Var(_, _), LTermInner::Var(_, _)) => {
                // Both are variables - keep constraint for later evaluation
                let walked_constraint = LessZConstraint::new(u_walk, v_walk);
                Ok(state.with_constraint(walked_constraint))
            }
            _ => {
                // Some operands grounded to terms of invalid type
                Err(())
            }
        }
    }

    fn operands(&self) -> Vec<LTerm<U, E>> {
        vec![self.u.clone(), self.v.clone()]
    }
}

impl<U, E> Display for LessZConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} < {}", self.u, self.v)
    }
}

/// CLPZ Less-than goal (the relation interface)
#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct LessZ<U, E>
where
    U: User,
    E: Engine<U>,
{
    u: LTerm<U, E>,
    v: LTerm<U, E>,
}

impl<U, E> LessZ<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new<G: AnyGoal<U, E>>(u: LTerm<U, E>, v: LTerm<U, E>) -> InferredGoal<U, E, G> {
        InferredGoal::new(G::dynamic(Rc::new(LessZ { u, v })))
    }
}

impl<U, E> Solve<U, E> for LessZ<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        match LessZConstraint::new(self.u.clone(), self.v.clone()).run(state) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

/// Public interface function for less-than constraint
pub fn ltz<U, E, G>(u: LTerm<U, E>, v: LTerm<U, E>) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    LessZ::new(u, v)
}

#[cfg(test)]
mod test {
    use super::ltz;
    use crate::prelude::*;
    use crate::relation::clpz::ltez::ltez;

    #[test]
    fn test_ltz_ground_true() {
        // Test u < v where 5 < 10 (should succeed)
        let query = proto_vulcan_query!(|result| {
            result == 5,
            ltz(result, 10)
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().result, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltz_ground_false() {
        // Test u < v where 10 < 5 (should fail)
        let query = proto_vulcan_query!(|result| {
            result == 10,
            ltz(result, 5)
        });

        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltz_equal_boundary_false() {
        // Test u < v where 5 < 5 (should fail - strict inequality)
        let query = proto_vulcan_query!(|result| {
            result == 5,
            ltz(result, 5)
        });

        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltz_constraint_generation() {
        // Test constraint propagation: x < y, y = 10, should constrain x
        let query = proto_vulcan_query!(|x| {
            |y| {
                ltz(x, y),
                y == 10,
                x == 8  // This should satisfy x < 10
            }
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().x, 8);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltz_constraint_generation_boundary_fail() {
        // Test constraint propagation boundary: x < y, y = 5, x = 5 should fail
        let query = proto_vulcan_query!(|x| {
            |y| {
                ltz(x, y),
                y == 5,
                x == 5  // This should fail since 5 is not < 5
            }
        });

        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltz_negative_numbers() {
        // Test with negative numbers: -5 < -2 (should succeed)
        let query = proto_vulcan_query!(|result| {
            result == -5,
            ltz(result, -2)
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().result, -5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltz_zero_boundary() {
        // Test zero boundary: -1 < 0 (should succeed)
        let query = proto_vulcan_query!(|result| {
            result == -1,
            ltz(result, 0)
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().result, -1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltz_chained_constraints() {
        // Test multiple constraints: x < y < z
        let query = proto_vulcan_query!(|x| {
            |y, z| {
                ltz(x, y),
                ltz(y, z),
                x == 3,
                z == 7,
                y == 5  // Should satisfy 3 < 5 < 7
            }
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().x, 3);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltz_vs_ltez_difference() {
        // Test that < and <= behave differently at boundary
        let query_lt = proto_vulcan_query!(|result| {
            result == 5,
            ltz(result, 5)  // 5 < 5 should fail
        });

        let query_lte = proto_vulcan_query!(|result| {
            result == 5,
            ltez(result, 5)  // 5 <= 5 should succeed
        });

        // Less-than should fail
        let mut iter_lt = query_lt.run();
        assert!(iter_lt.next().is_none());

        // Less-than-or-equal should succeed
        let mut iter_lte = query_lte.run();
        assert_eq!(iter_lte.next().unwrap().result, 5);
        assert!(iter_lte.next().is_none());
    }
}
