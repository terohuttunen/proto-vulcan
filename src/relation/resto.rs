use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::relation::conso;
use crate::user::User;

/// A relation such that `rest` is `list` without its first element.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::*;
/// use proto_vulcan::relation::resto;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         resto([1, 2, 3], q)
///     });
///     assert!(query.run().next().unwrap().q == lterm!([2, 3]));
/// }
/// ```
pub fn resto<U: User>(list: LTerm, rest: LTerm) -> Goal<U> {
    proto_vulcan!(|first| { conso(first, rest, list) })
}

#[cfg(test)]
mod test {
    use super::resto;
    use crate::*;

    #[test]
    fn test_resto_1() {
        let query = proto_vulcan_query!(|q| { resto([1], q) });
        assert!(query.run().next().unwrap().q == lterm!([]));
    }

    #[test]
    fn test_resto_2() {
        let query = proto_vulcan_query!(|q| { resto([1, 2], q) });
        assert!(query.run().next().unwrap().q == lterm!([2]));
    }

    #[test]
    fn test_resto_3() {
        let query = proto_vulcan_query!(|q| { resto([1, 2, 3], q) });
        assert!(query.run().next().unwrap().q == lterm!([2, 3]));
    }

    #[test]
    fn test_resto_4() {
        let query = proto_vulcan_query!(|q| { resto([1, [2, 3]], q) });
        assert!(query.run().next().unwrap().q == lterm!([[2, 3]]));
    }
}
