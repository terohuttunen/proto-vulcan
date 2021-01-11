use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::User;

/// A relation that succeeds when `s` is an empty list. This is equivalent to `s == []`.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::*;
/// use proto_vulcan::relation::emptyo;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         conde {
///             [q == [], emptyo(q)]
///         }
///     });
///     assert!(query.run().next().unwrap().q == lterm!([]));
/// }
/// ```
pub fn emptyo<U: User>(s: LTerm<U>) -> Goal<U> {
    proto_vulcan!([] == s)
}

#[cfg(test)]
mod test {
    use super::emptyo;
    use crate::operator::conde::conde;
    use crate::*;

    #[test]
    fn test_emptyo_1() {
        let query = proto_vulcan_query!(|q| {
            conde {
                [q == [], emptyo(q)]
            }
        });
        assert!(query.run().next().unwrap().q == lterm!([]));
    }

    #[test]
    fn test_emptyo_2() {
        let query = proto_vulcan_query!(|q| {
            conde {
                [q == [1, 2, 3], emptyo(q)],
            }
        });
        assert!(query.run().next().is_none());
    }
}
