use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::operator::conde;
use crate::relation::firsto;
use crate::relation::resto;
use crate::state::UserState;
use std::rc::Rc;

/// A relation that succeeds for each occurrence of `x` in list `l`.
///
/// # Example
/// ```rust
/// # #![recursion_limit = "512"]
/// use proto_vulcan::*;
/// use proto_vulcan::relation::membero;
/// let query = proto_vulcan_query!(|q| {
///     membero(q, [1, 2, 3])
/// });
/// let mut iter = query.run();
/// assert!(iter.next().unwrap().q == 1);
/// assert!(iter.next().unwrap().q == 2);
/// assert!(iter.next().unwrap().q == 3);
/// assert!(iter.next().is_none());
/// ```
pub fn membero<U: UserState>(x: &Rc<LTerm>, l: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    let x = Rc::clone(x);
    let l = Rc::clone(l);
    proto_vulcan!(
        closure {
            conde {
                |a| {
                    firsto(l, a),
                    a == x
                },
                |d| {
                    resto(l, d),
                    membero(x, d)
                }
            }
    })
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
