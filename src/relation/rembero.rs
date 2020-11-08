use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::UserState;
use std::rc::Rc;

/// A relation where `out` is equal to `ls` with first occurrence of `x` removed.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::*;
/// use proto_vulcan::relation::rembero;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         rembero(2, [1, 2, 3, 2, 4], q)
///     });
///     assert!(query.run().next().unwrap().q == lterm!([1, 3, 2, 4]));
/// }
/// ```
pub fn rembero<U: UserState>(x: Rc<LTerm>, ls: Rc<LTerm>, out: Rc<LTerm>) -> Rc<dyn Goal<U>> {
    proto_vulcan_closure!(
        match [ls, out] {
            [[], []] => ,
            [[a | d], d] => a == x,
            [[y | ys], [y | zs]] => {
                y != x,
                rembero(x, ys, zs)
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
