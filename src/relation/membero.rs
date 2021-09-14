use crate::engine::Engine;
use crate::goal::{AnyGoal, DFSGoal, Goal};
use crate::lterm::LTerm;
use crate::user::User;

/// A relation that succeeds for each occurrence of `x` in list `l`.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// use proto_vulcan::relation::membero;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         membero(q, [1, 2, 3])
///     });
///     let mut iter = query.run();
///     assert!(iter.next().unwrap().q == 1);
///     assert!(iter.next().unwrap().q == 2);
///     assert!(iter.next().unwrap().q == 3);
///     assert!(iter.next().is_none());
/// }
/// ```
pub fn membero<U, E>(x: LTerm<U, E>, l: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    proto_vulcan_closure!(match l {
        [head | _] => head == x,
        [_ | rest] => membero(x, rest),
    })
}

pub fn member<U, E>(x: LTerm<U, E>, l: LTerm<U, E>) -> DFSGoal<U, E>
where
    U: User,
    E: Engine<U>,
{
    proto_vulcan_closure!(match l {
        [head | _] => head == x,
        [_ | rest] => member(x, rest),
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_membero_1() {
        let query = proto_vulcan_query!(|q| { membero(q, [1, 2, 3]) });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().unwrap().q == 2);
        assert!(iter.next().unwrap().q == 3);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_membero_2() {
        let query = proto_vulcan_query!(|q| { membero(q, [1, 1, 1]) });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_membero_3() {
        let query = proto_vulcan_query!(|q| { membero(q, []) });
        assert!(query.run().next().is_none());
    }
}
