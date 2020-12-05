use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::UserState;

/// A relation which guarantees that all elements of `l` are distinct from each other.
pub fn distincto<U: UserState>(l: LTerm) -> Goal<U> {
    proto_vulcan_closure!(
        match l {
            [] | [_] => ,
            [first, second | rest] => {
                first != second,
                distincto([first | rest]),
                distincto([second | rest]),
            }
        }
    )
}

#[cfg(test)]
mod test {
    use super::distincto;
    use crate::*;

    #[test]
    fn test_distincto_1() {
        let query = proto_vulcan_query!(|q| {
            |x, y, a, b| {
                distincto(q),
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
