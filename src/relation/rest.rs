use crate::engine::Engine;
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::relation::cons;
use crate::user::User;

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
pub fn rest<U, E, G>(list: LTerm<U, E>, rest: LTerm<U, E>) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
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
