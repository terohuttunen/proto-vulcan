use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::operator::conde;
use crate::relation::conso;
use crate::user::UserState;
use std::rc::Rc;

/// A relation where `out` is equal to `ls` with first occurrence of `x` removed.
///
/// # Example
/// ```rust
/// # #![recursion_limit = "512"]
/// use proto_vulcan::*;
/// use proto_vulcan::relation::rembero;
/// let query = proto_vulcan_query!(|q| {
///     rembero(2, [1, 2, 3, 2, 4], q)
/// });
/// assert!(query.run().next().unwrap().q == lterm!([1, 3, 2, 4]))
/// ```
pub fn rembero<U: UserState>(x: &Rc<LTerm>, ls: &Rc<LTerm>, out: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    let x = Rc::clone(x);
    proto_vulcan!(
        conde {
            [ls == [], out == []],
            |a, d| {
                conso(a, d, ls),
                a == x,
                d == out,
            },
            |a, d, res| {
                conso(a, d, ls),
                a != x,
                conso(a, res, out),
                closure { rembero(x, d, res) },
            }
        }
    )
}

#[cfg(test)]
mod test {
    use super::rembero;
    use crate::*;

    #[test]
    fn test_rembero_1() {
        let query = proto_vulcan_query!(|q| { rembero(2, [1, 2, 3, 2, 4], q) });
        assert!(query.run().next().unwrap().q == lterm!([1, 3, 2, 4]))
    }
}
