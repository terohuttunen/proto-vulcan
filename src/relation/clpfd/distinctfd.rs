/// distinctfd finite domain constraint
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::solver::{Solve, Solver};
use crate::state::{Constraint, FiniteDomain, SResult, State};
use crate::stream::Stream;

use std::rc::Rc;

#[derive(Debug)]
pub struct DistinctFd {
    u: LTerm,
}

impl DistinctFd {
    pub fn new<G: AnyGoal>(u: LTerm) -> InferredGoal<G> {
        InferredGoal::new(G::dynamic(Rc::new(DistinctFd { u })))
    }
}

impl Solve for DistinctFd {
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        let u = self.u.clone();
        match DistinctFdConstraint::new(u).run(state) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

pub fn distinctfd<G>(u: LTerm) -> InferredGoal<G>
where
    G: AnyGoal,
{
    DistinctFd::new(u)
}

#[derive(Debug)]
pub struct DistinctFdConstraint {
    u: LTerm,
}

impl DistinctFdConstraint {
    pub fn new(u: LTerm) -> Rc<dyn Constraint> {
        assert!(u.is_list());
        Rc::new(DistinctFdConstraint { u })
    }
}

impl Constraint for DistinctFdConstraint {
    fn run(self: Rc<Self>, state: State) -> SResult {
        let smap = state.get_smap();

        let v = smap.walk(&self.u);
        match v.as_ref() {
            LTermInner::Var(_, _) => {
                // The term has not yet been associated with a list of terms that we want
                // to constrain, keep the constraint for later.
                Ok(state.with_constraint(self))
            }
            LTermInner::Empty | LTermInner::Cons(_, _) => {
                // Partition the list of terms to unresolved variables in `x` and constants in `n`.
                let (x, n): (LTerm, LTerm) = v.iter().cloned().partition(|v| v.is_var());

                // Convert list of LTerm constants to Vec<usize>
                let mut n = n
                    .iter()
                    .map(|t| match t.as_ref() {
                        LTermInner::Val(LValue::Number(u)) => *u,
                        _ => panic!("Invalid constant constraint {:?}", t),
                    })
                    .collect::<Vec<isize>>();

                // Sort the array so that we can find duplicates with a simple scan
                n.sort_unstable();

                // See if there are any duplicate values in the sorted array.
                let mut it = n.iter();
                let no_duplicates = match it.next() {
                    Some(first) => it
                        .scan(first, |previous, current| {
                            let cmp = *previous < current;
                            *previous = current;
                            Some(cmp)
                        })
                        .all(|cmp| cmp),
                    None => true,
                };

                if no_duplicates {
                    // There are no duplicate constant constraints. Create a new constraint
                    // to follow the fulfillment of the variable domain constraints.
                    let c = DistinctFd2Constraint::new(self.u.clone(), x, n);
                    Ok(state.with_constraint(c))
                } else {
                    // If there are duplicate constants in the array, then the constraint is
                    // already violated.
                    Err(())
                }
            }
            _ => panic!(
                "Cannot constrain {:?}. The variable must be grounded to a list of terms.",
                v
            ),
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.u.clone()]
    }
}

impl std::fmt::Display for DistinctFdConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

#[derive(Clone, Debug)]
pub struct DistinctFd2Constraint {
    u: LTerm,
    y: LTerm,
    n: Vec<isize>,
}

impl DistinctFd2Constraint {
    pub fn new(u: LTerm, y: LTerm, n: Vec<isize>) -> Rc<dyn Constraint> {
        assert!(u.is_list());
        assert!(y.is_list());
        Rc::new(DistinctFd2Constraint { u, y, n })
    }
}

impl Constraint for DistinctFd2Constraint {
    fn run(mut self: Rc<Self>, state: State) -> SResult {
        let smap = state.get_smap();

        let mut x = LTerm::empty_list();
        let mself = Rc::make_mut(&mut self);
        for y in mself.y.into_iter() {
            let ywalk = smap.walk(&y);
            match ywalk.as_ref() {
                LTermInner::Var(_, _) => {
                    // Terms that walk to variables cannot be resolved to values yet. Such terms
                    // are moved from y to x, where they will become the new y on next run of
                    // constraints.
                    x.extend(Some(y.clone()));
                }
                LTermInner::Val(val) => {
                    // A variable has been associated with a value and can be moved from y to n.
                    match val {
                        LValue::Number(u) => {
                            match mself.n.binary_search(u) {
                                Ok(_) => {
                                    // Duplicate invalidates the constraint
                                    return Err(());
                                }
                                Err(pos) => {
                                    // Add the previously unseen value to the list of constant
                                    // constraints.
                                    mself.n.insert(pos, *u);
                                }
                            }
                        }
                        _ => panic!("Invalid value {:?} in constraint", val),
                    }
                }
                _ => panic!("Invalid LTerm  {:?} in constraint", ywalk),
            }
        }

        // Create a new all-diff constraint with (hopefully) less unassociated variables in y and
        // more constants in n.
        mself.y = x.clone();
        if mself.n.is_empty() {
            Ok(state.with_constraint(self))
        } else {
            let ndomain = Rc::new(FiniteDomain::from(mself.n.clone()));
            state.with_constraint(self).exclude_from_domain(&x, ndomain)
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        self.u.iter().cloned().collect()
    }
}

impl std::fmt::Display for DistinctFd2Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

#[cfg(test)]
mod tests {
    use super::distinctfd;
    use crate::prelude::*;
    use crate::relation::clpfd::diseqfd::diseqfd;
    use crate::relation::clpfd::infd::{infd, infdrange};
    use crate::relation::clpfd::ltefd::ltefd;

    #[test]
    fn test_distinctfd_1() {
        let query = proto_vulcan_query!(|q| { distinctfd([1, 2, 3, 4, 5]) });
        let mut iter = query.run();
        let result = iter.next().unwrap();
        assert!(result.q.is_any() && !result.q.is_constrained());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_distinctfd_2() {
        let query = proto_vulcan_query!(|q| { distinctfd([1, 2, 3, 4, 4, 5]) });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_distinctfd_3() {
        let query = proto_vulcan_query!(|q| {
            infdrange(q, &(0..=2)),
            distinctfd([q])
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 0);
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_distinctfd_4() {
        let query = proto_vulcan_query!(|q| {
            infdrange(q, &(0..=2)),
            distinctfd([q, q])
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_distinctfd_5() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                infdrange([x, y, z], &(0..=2)),
                distinctfd([x, y, z]),
                q == [x, y, z],
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([0, 1, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([0, 2, 1]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 0, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 0, 1]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 2, 0]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 1, 0]));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_distinctfd_6() {
        let query = proto_vulcan_query!(|q| {
            |a, b, c, x| {
                infdrange([a, b, c], &(1..=3)),
                distinctfd([a, b, c]),
                diseqfd(c, x),
                ltefd(b, 2),
                x == 3,
                q == [a, b, c],
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([3, 1, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([3, 2, 1]));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_distinctfd_7() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                infd([x, y, z], &[1, 2]),
                distinctfd([x, y, z]),
            }
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }
}
