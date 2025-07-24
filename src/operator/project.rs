//! # Projection
//!
//! For projecting variables there is a built-in operator `project |x, y, z| { <body> }`, where
//! variables already declared earlier, can be projected within the operator body as specified
//! by the projection list `|x, y, z|`.
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use derivative::Derivative;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Project<G>
where
    G: AnyGoal,
{
    variables: Vec<LTerm>,
    body: G,
}

impl<G> Project<G>
where
    G: AnyGoal,
{
    pub fn new(variables: Vec<LTerm>, body: G) -> InferredGoal<G> {
        InferredGoal::new(G::dynamic(Rc::new(Project { variables, body })))
    }
}

impl<G> Solve for Project<G>
where
    G: AnyGoal,
{
    fn solve(&self, solver: &Solver, state: State) -> Stream {
        // Walk* each projected variable with the current substitution
        for v in self.variables.iter() {
            v.project(|x| state.smap_ref().walk_star(x));
        }
        self.body.solve(solver, state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lterm::LTermInner;
    use crate::prelude::*;
    use crate::solver::{Solve, Solver};
    use derivative::Derivative;
    use std::rc::Rc;

    #[derive(Derivative)]
    #[derivative(Debug)]
    pub struct SqEq {
        u: LTerm,
        v: LTerm,
    }

    impl SqEq {
        pub fn new(u: LTerm, v: LTerm) -> Goal {
            Goal::dynamic(Rc::new(SqEq { u, v }))
        }
    }

    impl Solve for SqEq {
        fn solve(&self, solver: &Solver, state: State) -> Stream {
            let u = self.u.clone();
            let v = self.v.clone();
            let g: Goal = proto_vulcan!(fngoal move |_solver, state| {
                match u.as_ref() {
                    // sqeq is non-relational operator and requires `u` to be associated with
                    // integer value to succeed.
                    LTermInner::Val(LValue::Number(u)) => {
                        let sq = LTerm::from(u * u);
                        Stream::unit(Box::new(state.unify(&sq, &v).unwrap()))
                    }
                    _ => Stream::empty(),
                }
            });
            g.solve(solver, state)
        }
    }

    fn sqeq(u: LTerm, v: LTerm) -> Goal {
        SqEq::new(u, v)
    }

    #[test]
    fn test_project_1() {
        // Project helps non-relational goal sqeq! to succeed
        let query = proto_vulcan_query!(|q| {
            |x| {
                5 == x,
                project |x| {
                    sqeq(x, q)
                }
            }
        });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 25);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_project_2() {
        // Does not succeed without project!
        let query = proto_vulcan_query!(|q| {
            |x| {
                5 == x,
                sqeq(x, q)
            }
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_project_3() {
        // project! itself is non-relational, and fails if the variable is not grounded.
        let query = proto_vulcan_query!(|q| {
            |x| {
                project |x| {
                    sqeq(x, q),
                    5 == x,
                }
            }
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }
}
