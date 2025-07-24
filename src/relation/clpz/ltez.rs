/// Constrains u <= v for integers
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::solver::{Solve, Solver};
use crate::state::{Constraint, SResult, State};
use crate::stream::Stream;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

/// CLPZ Less-than-or-equal constraint: u <= v for integers
#[derive(Debug, Clone)]
pub struct LessEqualZConstraint {
    u: LTerm,
    v: LTerm,
}

impl LessEqualZConstraint {
    pub fn new(u: LTerm, v: LTerm) -> Rc<Self> {
        Rc::new(LessEqualZConstraint { u, v })
    }
}

impl Constraint for LessEqualZConstraint {
    fn run(self: Rc<Self>, state: State) -> SResult {
        // Walk the terms to get their current values
        let u_walk = state.smap_ref().walk(&self.u).clone();
        let v_walk = state.smap_ref().walk(&self.v).clone();

        // Pattern match like other CLPZ constraints to handle all cases
        match (u_walk.as_ref(), v_walk.as_ref()) {
            (LTermInner::Val(LValue::Number(u_num)), LTermInner::Val(LValue::Number(v_num))) => {
                // Both are ground integers - check constraint immediately
                if u_num <= v_num {
                    Ok(state) // Constraint satisfied, drop it
                } else {
                    Err(()) // Constraint violated, fail
                }
            }
            (LTermInner::Val(LValue::Number(_)), LTermInner::Var(_, _))
            | (LTermInner::Var(_, _), LTermInner::Val(LValue::Number(_))) => {
                // One ground, one variable - keep constraint for re-evaluation when variable is bound
                let walked_constraint = LessEqualZConstraint::new(u_walk, v_walk);
                Ok(state.with_constraint(walked_constraint))
            }
            (LTermInner::Var(_, _), LTermInner::Var(_, _)) => {
                // Both are variables - keep constraint for later evaluation
                let walked_constraint = LessEqualZConstraint::new(u_walk, v_walk);
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

impl Display for LessEqualZConstraint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} <= {}", self.u, self.v)
    }
}

/// CLPZ Less-than-or-equal goal (the relation interface)
#[derive(Debug)]
pub struct LessEqualZ {
    u: LTerm,
    v: LTerm,
}

impl LessEqualZ {
    pub fn new<G: AnyGoal>(u: LTerm, v: LTerm) -> InferredGoal<G> {
        InferredGoal::new(G::dynamic(Rc::new(LessEqualZ { u, v })))
    }
}

impl Solve for LessEqualZ {
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        match LessEqualZConstraint::new(self.u.clone(), self.v.clone()).run(state) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

/// Public interface function for less-than-or-equal constraint
pub fn ltez<G>(u: LTerm, v: LTerm) -> InferredGoal<G>
where
    G: AnyGoal,
{
    LessEqualZ::new(u, v)
}

#[cfg(test)]
mod test {
    use super::ltez;
    use crate::prelude::*;

    #[test]
    fn test_ltez_ground_true() {
        // Test u <= v where 5 <= 10 (should succeed)
        let query = proto_vulcan_query!(|result| {
            result == 5,
            ltez(result, 10)
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().result, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltez_ground_false() {
        // Test u <= v where 10 <= 5 (should fail)
        let query = proto_vulcan_query!(|result| {
            result == 10,
            ltez(result, 5)
        });

        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltez_equal_boundary() {
        // Test u <= v where 5 <= 5 (should succeed)
        let query = proto_vulcan_query!(|result| {
            result == 5,
            ltez(result, 5)
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().result, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltez_constraint_generation() {
        // Test constraint propagation: x <= y, y = 10, should constrain x
        let query = proto_vulcan_query!(|x| {
            |y| {
                ltez(x, y),
                y == 10,
                x == 8  // This should satisfy x <= 10
            }
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().x, 8);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltez_constraint_generation_fail() {
        // Test constraint propagation failure: x <= y, y = 5, x = 10 should fail
        let query = proto_vulcan_query!(|x| {
            |y| {
                ltez(x, y),
                y == 5,
                x == 10  // This should fail since 10 > 5
            }
        });

        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltez_negative_numbers() {
        // Test with negative numbers: -5 <= -2 (should succeed)
        let query = proto_vulcan_query!(|result| {
            result == -5,
            ltez(result, -2)
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().result, -5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltez_zero_boundary() {
        // Test zero boundary: 0 <= 1 (should succeed)
        let query = proto_vulcan_query!(|result| {
            result == 0,
            ltez(result, 1)
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().result, 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltez_chained_constraints() {
        // Test multiple constraints: x <= y <= z
        let query = proto_vulcan_query!(|x| {
            |y, z| {
                ltez(x, y),
                ltez(y, z),
                x == 3,
                z == 7,
                y == 5  // Should satisfy 3 <= 5 <= 7
            }
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().x, 3);
        assert!(iter.next().is_none());
    }
}
