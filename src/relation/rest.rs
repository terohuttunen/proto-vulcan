
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::relation::cons;


/// A relation such that `rest` is `list` without its first element.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// use proto_vulcan::relation::rest;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         rest([1, 2, 3], q)
///     });
///     assert!(query.run().next().unwrap().q == lterm!([2, 3]));
/// }
/// ```
pub fn rest<G>(list: LTerm, rest: LTerm) -> InferredGoal<G>
where
    G: AnyGoal,
{
    proto_vulcan!(|first| { cons(first, rest, list) })
}

#[cfg(test)]
mod test {
    use super::rest;
    use crate::prelude::*;

    #[test]
    fn test_rest_1() {
        let query = proto_vulcan_query!(|q| { rest([1], q) });
        assert!(query.run().next().unwrap().q == lterm!([]));
    }

    #[test]
    fn test_rest_2() {
        let query = proto_vulcan_query!(|q| { rest([1, 2], q) });
        assert!(query.run().next().unwrap().q == lterm!([2]));
    }

    #[test]
    fn test_rest_3() {
        let query = proto_vulcan_query!(|q| { rest([1, 2, 3], q) });
        assert!(query.run().next().unwrap().q == lterm!([2, 3]));
    }

    #[test]
    fn test_rest_4() {
        let query = proto_vulcan_query!(|q| { rest([1, [2, 3]], q) });
        assert!(query.run().next().unwrap().q == lterm!([[2, 3]]));
    }
}
