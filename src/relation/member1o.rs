use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::UserState;
use std::rc::Rc;

/// A relation that succeeds once if `x` is in list `l`.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::*;
/// use proto_vulcan::relation::member1o;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         member1o(q, [1, 1, 1, 1, 1])
///     });
///     let mut iter = query.run();
///     assert_eq!(iter.next().unwrap().q, 1);
///     assert!(iter.next().is_none());
/// }
/// ```
pub fn member1o<U: UserState>(x: &Rc<LTerm>, l: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    let x = Rc::clone(x);
    let l = Rc::clone(l);
    proto_vulcan!(
        closure {
            match l {
                [head | _] => head == x,
                [head | rest] => [head != x, member1o(x, rest)],
            }
        }
    )
}

#[cfg(test)]
mod tests {
    use super::member1o;
    use crate::*;

    #[test]
    fn test_member1o_1() {
        let query = proto_vulcan_query!(|q| { member1o(q, [1, 1, 1, 1, 1]) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1o_2() {
        let query = proto_vulcan_query!(|q| { member1o(q, [1, 2, 3, 4, 5]) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 4);
        assert_eq!(iter.next().unwrap().q, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1o_3() {
        let query = proto_vulcan_query!(|q| { member1o(q, [1, 0, 0, 0, 1]) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1o_4() {
        let query = proto_vulcan_query!(|q| { member1o(q, [1, 1, 1, 1, 0]) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1o_5() {
        let query = proto_vulcan_query!(|q| { member1o(q, [1, 1, 1, 0, 0]) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1o_6() {
        let query = proto_vulcan_query!(|q| { member1o(q, []) });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1o_7() {
        let query = proto_vulcan_query!(|q| { member1o(q, [5]) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_member1o_8() {
        let query = proto_vulcan_query!(|q| {
            |a, b, c| {
                member1o(q, [a, b, c]),
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
    fn test_member1o_9() {
        let query = proto_vulcan_query!(|q| {
            |a, b, c| {
                member1o(q, [a, a, b, b]),
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
    fn test_member1o_10() {
        let query = proto_vulcan_query!(|q| {
            |a, b, c| {
                member1o(q, [a, a, b, b, c, c]),
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
