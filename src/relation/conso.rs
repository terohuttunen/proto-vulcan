use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::UserState;
use std::rc::Rc;

/// A relation such that the `out` parameter is equal to `rest` parameter appended to `first`
/// parameter. The `first` parameter is the head of the list `out` and the `rest` is the tail.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::*;
/// use proto_vulcan::relation::conso;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         conso(1, [2, 3], q)
///     });
///     assert!(query.run().next().unwrap().q == lterm!([1, 2, 3]));
/// }
/// ```
pub fn conso<U: UserState>(
    first: &Rc<LTerm>,
    rest: &Rc<LTerm>,
    out: &Rc<LTerm>,
) -> Rc<dyn Goal<U>> {
    proto_vulcan!([first | rest] == out)
}

#[cfg(test)]
mod test {
    use super::conso;
    use crate::*;

    #[test]
    fn test_conso_1() {
        let query = proto_vulcan_query!(|q| { conso(1, [2, 3], q) });
        assert!(query.run().next().unwrap().q == lterm!([1, 2, 3]));
    }

    #[test]
    fn test_conso_2() {
        let query = proto_vulcan_query!(|q| { conso([1, 2], [3, 4], q) });
        assert!(query.run().next().unwrap().q == lterm!([[1, 2], 3, 4]));
    }

    #[test]
    fn test_conso_3() {
        let query = proto_vulcan_query!(|q| { conso(1, [2], q) });
        assert!(query.run().next().unwrap().q == lterm!([1, 2]));
    }

    #[test]
    fn test_conso_4() {
        let query = proto_vulcan_query!(|q| { conso(q, [2], [1, 2]) });
        assert!(query.run().next().unwrap().q == 1);
    }

    #[test]
    fn test_conso_5() {
        let query = proto_vulcan_query!(|q| { conso(1, [q, 3], [1, 2, 3]) });
        assert!(query.run().next().unwrap().q == 2);
    }
}
