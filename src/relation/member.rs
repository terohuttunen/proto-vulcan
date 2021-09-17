use crate::engine::Engine;
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::user::User;

/// A relation that succeeds for each occurrence of `x` in list `l`.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// use proto_vulcan::relation::member;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         member(q, [1, 2, 3])
///     });
///     let mut iter = query.run();
///     assert!(iter.next().unwrap().q == 1);
///     assert!(iter.next().unwrap().q == 2);
///     assert!(iter.next().unwrap().q == 3);
///     assert!(iter.next().is_none());
/// }
/// ```
pub fn member<U, E, G>(x: LTerm<U, E>, l: LTerm<U, E>) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
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
    fn test_member_1() {
        let query = proto_vulcan_query!(|q| { member(q, [1, 2, 3]) });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().unwrap().q == 2);
        assert!(iter.next().unwrap().q == 3);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member_2() {
        let query = proto_vulcan_query!(|q| { member(q, [1, 1, 1]) });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member_3() {
        let query = proto_vulcan_query!(|q| { member(q, []) });
        assert!(query.run().next().is_none());
    }
}
