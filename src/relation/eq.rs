use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::marker::PhantomData;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Eq<U: User> {
    u: LTerm,
    v: LTerm,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: User> Eq<U> {
    pub fn new(u: LTerm, v: LTerm) -> Goal<U> {
        Goal::new(Eq {
            u,
            v,
            _phantom: PhantomData,
        })
    }
}

impl<U: User> Solve<U> for Eq<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        Stream::from(state.unify(&self.u, &self.v))
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
pub fn eq<U: User>(u: LTerm, v: LTerm) -> Goal<U> {
    Eq::new(u, v)
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn test_eq_1() {
        let query = proto_vulcan_query!(|q| {q == 1234});
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1234);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_eq_2() {
        let query = proto_vulcan_query!(|q| {
            q == [1, 2, 3]
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([1, 2, 3]));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_eq_3() {
        // Occurs-check 1
        let query = proto_vulcan_query!(|q| {
            q == [1, 2, 3, q]
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());


    }

    #[test]
    fn test_eq_4() {
        // Occurs-check 2
        let query = proto_vulcan_query!(|q| {
            [1, 2, 3, q] == q
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }
}
