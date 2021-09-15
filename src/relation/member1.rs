use crate::engine::Engine;
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::user::User;

/// A relation that succeeds once if `x` is in list `l`.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// use proto_vulcan::relation::member1;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         member1(q, [1, 1, 1, 1, 1])
///     });
///     let mut iter = query.run();
///     assert_eq!(iter.next().unwrap().q, 1);
///     assert!(iter.next().is_none());
/// }
/// ```
pub fn member1<U, E, G>(x: LTerm<U, E>, l: LTerm<U, E>) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    proto_vulcan_closure!(match l {
        [head | _] => head == x,
        [head | rest] => [head != x, member1(x, rest)],
    })
}

#[cfg(test)]
mod tests {
    use super::member1;
    use crate::prelude::*;

    #[test]
    fn test_member1_1() {
        let query = proto_vulcan_query!(|q| { member1(q, [1, 1, 1, 1, 1]) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1_2() {
        let query = proto_vulcan_query!(|q| { member1(q, [1, 2, 3, 4, 5]) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 4);
        assert_eq!(iter.next().unwrap().q, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1_3() {
        let query = proto_vulcan_query!(|q| { member1(q, [1, 0, 0, 0, 1]) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1_4() {
        let query = proto_vulcan_query!(|q| { member1(q, [1, 1, 1, 1, 0]) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1_5() {
        let query = proto_vulcan_query!(|q| { member1(q, [1, 1, 1, 0, 0]) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1_6() {
        let query = proto_vulcan_query!(|q| { member1(q, []) });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1_7() {
        let query = proto_vulcan_query!(|q| { member1(q, [5]) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1_8() {
        let query = proto_vulcan_query!(|q| {
            |a, b, c| {
                member1(q, [a, b, c]),
                a == 5,
                b == 3,
                c == 9,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!(5));
        assert_eq!(iter.next().unwrap().q, lterm!(3));
        assert_eq!(iter.next().unwrap().q, lterm!(9));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1_9() {
        let query = proto_vulcan_query!(|q| {
            |a, b, c| {
                member1(q, [a, a, b, b]),
                a == 5,
                b == 3,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!(5));
        assert_eq!(iter.next().unwrap().q, lterm!(3));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1_10() {
        let query = proto_vulcan_query!(|q| {
            |a, b, c| {
                member1(q, [a, a, b, b, c, c]),
                a == 5,
                b == 5,
                c == 3,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!(5));
        assert_eq!(iter.next().unwrap().q, lterm!(3));
        assert!(iter.next().is_none());
    }
}
