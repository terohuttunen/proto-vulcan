use crate::engine::Engine;
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::State;
use crate::user::User;

#[derive(Debug)]
pub struct Eq<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
}

impl<U: User> Eq<U> {
    pub fn new<E: Engine<U>>(u: LTerm<U>, v: LTerm<U>) -> Goal<U, E> {
        Goal::new(Eq { u, v })
    }
}

impl<U, E> Solve<U, E> for Eq<U>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, engine: &E, state: State<U>) -> E::Stream {
        match state.unify(&self.u, &self.v) {
            Ok(state) => engine.munit(state),
            Err(_) => engine.mzero(),
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
/// use proto_vulcan::*;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         q == 5,
///     });
///     let mut iter = query.run();
///     assert!(iter.next().unwrap().q == 5);
///     assert!(iter.next().is_none());
/// }
/// ```
pub fn eq<U, E>(u: LTerm<U>, v: LTerm<U>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Eq::new(u, v)
}

#[cfg(test)]
mod test {
    use crate::*;

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
