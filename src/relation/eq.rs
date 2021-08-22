use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct Eq<U, E>
where
    U: User,
    E: Engine<U>,
{
    u: LTerm<U, E>,
    v: LTerm<U, E>,
}

impl<U, E> Eq<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(u: LTerm<U, E>, v: LTerm<U, E>) -> Goal<U, E> {
        Goal::new(Eq { u, v })
    }
}

impl<U, E> Solve<U, E> for Eq<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
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
pub fn eq<U, E>(u: LTerm<U, E>, v: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
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
