// Less-than finite domain constraint
use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::relation::diseqfd::diseqfd;
use crate::relation::ltefd::ltefd;
use crate::user::User;

pub fn ltfd<U, E>(u: LTerm<U, E>, v: LTerm<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    proto_vulcan!([diseqfd(u, v), ltefd(u, v)])
}

#[cfg(test)]
mod tests {
    use super::ltfd;
    use crate::prelude::*;
    use crate::relation::diseqfd::diseqfd;
    use crate::relation::infd::infd;

    #[test]
    fn test_ltfd_1() {
        let query = proto_vulcan_query!(|q| {
            |x, y| {
                q == [x, y],
                infd(x, &[1, 2, 3]),
                infd(y, &[0, 1, 2, 3, 4]),
                ltfd(x, y),
            }
        });
        let iter = query.run();
        let mut expected = vec![
            lterm!([1, 2]),
            lterm!([1, 3]),
            lterm!([1, 4]),
            lterm!([3, 4]),
            lterm!([2, 3]),
            lterm!([2, 4]),
        ];
        iter.for_each(|x| {
            let n = x.q.clone();
            assert!(expected.contains(&n));
            expected.retain(|y| &n != y);
        });
        assert_eq!(expected.len(), 0);
    }

    #[test]
    fn test_ltfd_2() {
        let query = proto_vulcan_query!(|q| {
            |x, y| {
                q == [x, y],
                infd(x, &[1, 2, 3]),
                infd(y, &[0, 1, 2, 3, 4]),
                ltfd(x, y),
                diseqfd(x, 1),
                y == 3,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([2, 3]));
        assert!(iter.next().is_none());
    }
}
