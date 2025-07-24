
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;

use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Eq
{
    u: LTerm,
    v: LTerm,
}

impl Eq
{
    pub fn new<G: AnyGoal>(u: LTerm, v: LTerm) -> InferredGoal<G> {
        InferredGoal::new(G::dynamic(Rc::new(Eq { u, v })))
    }
}

impl Solve for Eq
{
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        match state.unify(&self.u, &self.v) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}

/// Equality relation.
///
/// Equality is one of the three core operations in miniKanren. Proto-vulcan provides a built-in
/// syntax `u == v` that avoids the use-clause: `use proto_vulcan::relation::eq`. Unlike `diseq`,
/// `eq` works also for finite-domain constraints.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         q == 5,
///     });
///     let mut iter = query.run();
///     assert!(iter.next().unwrap().q == 5);
///     assert!(iter.next().is_none());
/// }
/// ```
pub fn eq<G>(u: LTerm, v: LTerm) -> InferredGoal<G>
where
    G: AnyGoal,
{
    Eq::new(u, v)
}

#[cfg(test)]
mod test {
    use crate::prelude::*;

    #[test]
    fn test_eq_1() {
        let query = proto_vulcan_query!(|q| { q == 1234 });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1234);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_eq_2() {
        let query = proto_vulcan_query!(|q| { q == [1, 2, 3] });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([1, 2, 3]));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_eq_3() {
        // Occurs-check 1
        let query = proto_vulcan_query!(|q| { q == [1, 2, 3, q] });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_eq_4() {
        // Occurs-check 2
        let query = proto_vulcan_query!(|q| { [1, 2, 3, q] == q });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }
}
