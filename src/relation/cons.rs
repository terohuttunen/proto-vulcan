use crate::engine::Engine;
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::user::User;

/// A relation such that the `out` parameter is equal to `rest` parameter appended to `first`
/// parameter. The `first` parameter is the head of the list `out` and the `rest` is the tail.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// use proto_vulcan::relation::cons;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         cons(1, [2, 3], q)
///     });
///     assert!(query.run().next().unwrap().q == lterm!([1, 2, 3]));
/// }
/// ```

pub fn cons<U, E, G>(
    first: LTerm<U, E>,
    rest: LTerm<U, E>,
    out: LTerm<U, E>,
) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    proto_vulcan!([first | rest] == out)
}

#[cfg(test)]
mod test {
    use super::cons;
    use crate::prelude::*;

    #[test]
    fn test_cons_1() {
        let query = proto_vulcan_query!(|q| { cons(1, [2, 3], q) });
        assert!(query.run().next().unwrap().q == lterm!([1, 2, 3]));
    }

    #[test]
    fn test_cons_2() {
        let query = proto_vulcan_query!(|q| { cons([1, 2], [3, 4], q) });
        assert!(query.run().next().unwrap().q == lterm!([[1, 2], 3, 4]));
    }

    #[test]
    fn test_cons_3() {
        let query = proto_vulcan_query!(|q| { cons(1, [2], q) });
        assert!(query.run().next().unwrap().q == lterm!([1, 2]));
    }

    #[test]
    fn test_cons_4() {
        let query = proto_vulcan_query!(|q| { cons(q, [2], [1, 2]) });
        assert!(query.run().next().unwrap().q == 1);
    }

    #[test]
    fn test_cons_5() {
        let query = proto_vulcan_query!(|q| { cons(1, [q, 3], [1, 2, 3]) });
        assert!(query.run().next().unwrap().q == 2);
    }
}
