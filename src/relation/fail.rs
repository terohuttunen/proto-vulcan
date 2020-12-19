use crate::goal::Goal;
use crate::user::User;

/// A relation that fails.
///
/// Proto-vulcan provides a built-in syntax `false` to avoid the use-clause.
///
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::*;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         conde {
///             [true, q == 1],
///             [false, q == 2],
///         }
///     });
///     let mut iter = query.run();
///     assert!(iter.next().unwrap().q == 1);
///     assert!(iter.next().is_none());
/// }
/// ```
pub fn fail<U: User>() -> Goal<U> {
    Goal::fail()
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn test_fail() {
        let query = proto_vulcan_query!(|q| {false});
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }
}
