use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::operator::conde;
use crate::relation::emptyo;
use crate::user::UserState;
use std::rc::Rc;

/// A relation which guarantees that all elements of `l` are distinct from each other.
pub fn distincto<U: UserState>(l: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    let l = Rc::clone(l);
    proto_vulcan!(
        closure {
            conde {
                emptyo(l),
                |single| {
                    [single] == l
                },
                |first, second, rest| {
                    (first, second, rest) == l,
                    first != second,
                    distincto((first, rest)),
                    distincto((second, rest)),
                }
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
