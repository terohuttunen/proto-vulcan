use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::solver::{Solve, Solver};
use crate::state::{Constraint, SResult, State};
use crate::stream::Stream;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

/// CLPZ Less-than constraint: u < v for integers  
#[derive(Debug, Clone)]
pub struct LessZConstraint {
    u: LTerm,
    v: LTerm,
}

impl LessZConstraint {
    pub fn new(u: LTerm, v: LTerm) -> Rc<Self> {
        Rc::new(LessZConstraint { u, v })
    }
}

impl Constraint for LessZConstraint {
    fn run(self: Rc<Self>, state: State) -> SResult {
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

    fn operands(&self) -> Vec<LTerm> {
        vec![self.u.clone(), self.v.clone()]
    }
}

impl Display for LessZConstraint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} < {}", self.u, self.v)
    }
}

/// CLPZ Less-than goal (the relation interface)
#[derive(Debug)]
pub struct LessZ {
    u: LTerm,
    v: LTerm,
}

impl LessZ {
    pub fn new<G: AnyGoal>(u: LTerm, v: LTerm) -> InferredGoal<G> {
        InferredGoal::new(G::dynamic(Rc::new(LessZ { u, v })))
    }
}

impl Solve for LessZ {
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        match LessZConstraint::new(self.u.clone(), self.v.clone()).run(state) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

/// Public interface function for less-than constraint
pub fn ltz<G>(u: LTerm, v: LTerm) -> InferredGoal<G>
where
    G: AnyGoal,
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
