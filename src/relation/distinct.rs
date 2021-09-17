use crate::engine::Engine;
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::user::User;

/// A relation which guarantees that all elements of `l` are distinct from each other.
pub fn distinct<U, E, G>(l: LTerm<U, E>) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    proto_vulcan_closure!(
        match l {
            [] | [_] => ,
            [first, second | rest] => {
                first != second,
                distinct([first | rest]),
                distinct([second | rest]),
            }
        }
    )
}

#[cfg(test)]
mod test {
    use super::distinct;
    use crate::prelude::*;

    #[test]
    fn test_distinct_1() {
        let query = proto_vulcan_query!(|q| {
            |x, y, a, b| {
                distinct(q),
                [x, y] == [a, b],
                q == [a, b],
                x == 1,
                y == 2,
            }
        });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == lterm!([1, 2]));
    }
}
