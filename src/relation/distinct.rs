use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::User;

/// A relation which guarantees that all elements of `l` are distinct from each other.
pub fn distinct<U, E>(l: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
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
