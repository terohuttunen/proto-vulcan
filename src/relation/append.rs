use crate::engine::Engine;
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::user::User;

/// A relation where `l`, `s`, and `ls` are proper lists, such that `ls` is `s` appended to `l`.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// use proto_vulcan::relation::append;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         append([1, 2, 3], [4, 5], q)
///     });
///     assert!(query.run().next().unwrap().q == lterm!([1, 2, 3, 4, 5]));
/// }
pub fn append<U, E, G>(l: LTerm<U, E>, s: LTerm<U, E>, ls: LTerm<U, E>) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    proto_vulcan_closure!(
        match [l, s, ls] {
            [[], x, x] => ,
            [[x | l1], l2, [x | l3]] => append(l1, l2, l3),
        }
    )
}

#[cfg(test)]
mod test {
    use super::append;
    use crate::prelude::*;

    #[test]
    fn test_append_1() {
        let query = proto_vulcan_query!(|q| { append([1, 2, 3], [4, 5], q) });
        assert!(query.run().next().unwrap().q == lterm!([1, 2, 3, 4, 5]));
    }
}
