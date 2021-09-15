use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::User;

/// A relation that succeeds when `s` is an empty list. This is equivalent to `s == []`.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// use proto_vulcan::relation::empty;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         conde {
///             [q == [], empty(q)]
///         }
///     });
///     assert!(query.run().next().unwrap().q == lterm!([]));
/// }
/// ```
pub fn empty<U, E>(s: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    proto_vulcan!([] == s)
}

#[cfg(test)]
mod test {
    use super::empty;
    use crate::operator::conde::conde;
    use crate::prelude::*;

    #[test]
    fn test_empty_1() {
        let query = proto_vulcan_query!(|q| {
            conde {
                [q == [], empty(q)]
            }
        });
        assert!(query.run().next().unwrap().q == lterm!([]));
    }

    #[test]
    fn test_empty_2() {
        let query = proto_vulcan_query!(|q| {
            conde {
                [q == [1, 2, 3], empty(q)],
            }
        });
        assert!(query.run().next().is_none());
    }
}
