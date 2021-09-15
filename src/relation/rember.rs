use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::User;

/// A relation where `out` is equal to `ls` with first occurrence of `x` removed.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// use proto_vulcan::relation::rember;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         rember(2, [1, 2, 3, 2, 4], q)
///     });
///     assert!(query.run().next().unwrap().q == lterm!([1, 3, 2, 4]));
/// }
/// ```
pub fn rember<U, E>(x: LTerm<U, E>, ls: LTerm<U, E>, out: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    proto_vulcan_closure!(
        match [ls, out] {
            [[], []] => ,
            [[a | d], d] => a == x,
            [[y | ys], [y | zs]] => {
                y != x,
                rember(x, ys, zs)
            }
        }
    )
}

#[cfg(test)]
mod test {
    use super::rember;
    use crate::prelude::*;

    #[test]
    fn test_rember_1() {
        let query = proto_vulcan_query!(|q| { rember(2, [1, 2, 3, 2, 4], q) });
        assert!(query.run().next().unwrap().q == lterm!([1, 3, 2, 4]))
    }
}
