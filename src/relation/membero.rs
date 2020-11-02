use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::UserState;
use std::rc::Rc;

/// A relation that succeeds for each occurrence of `x` in list `l`.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::*;
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
pub fn membero<U: UserState>(x: &Rc<LTerm>, l: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    let x = Rc::clone(x);
    let l = Rc::clone(l);
    proto_vulcan!(
        closure {
            match l {
                [head | _] => head == x,
                [_ | rest] => membero(x, rest),
            }
        }
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

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
