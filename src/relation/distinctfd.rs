/// distinctfd finite domain constraint
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::DistinctFdConstraint;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;

#[derive(Debug)]
pub struct DistinctFd<U: User> {
    u: LTerm<U>,
}

impl<U: User> DistinctFd<U> {
    pub fn new(u: LTerm<U>) -> Goal<U> {
        Goal::new(DistinctFd { u })
    }
}

impl<U: User> Solve<U> for DistinctFd<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let c = DistinctFdConstraint::new(self.u.clone());
        Stream::from(c.run(state))
    }
}

pub fn distinctfd<U: User>(u: LTerm<U>) -> Goal<U> {
    DistinctFd::new(u)
}

#[cfg(test)]
mod tests {
    use super::distinctfd;
    use crate::relation::diseqfd::diseqfd;
    use crate::relation::infd::{infd, infdrange};
    use crate::relation::ltefd::ltefd;
    use crate::*;

    #[test]
    fn test_distinctfd_1() {
        let query = proto_vulcan_query!(|q| { distinctfd([1, 2, 3, 4, 5]) });
        let mut iter = query.run();
        let result = iter.next().unwrap();
        assert!(result.q.is_any() && !result.q.is_constrained());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_distinctfd_2() {
        let query = proto_vulcan_query!(|q| { distinctfd([1, 2, 3, 4, 4, 5]) });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_distinctfd_3() {
        let query = proto_vulcan_query!(|q| {
            infdrange(q, #&(0..=2)),
            distinctfd([q])
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 0);
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_distinctfd_4() {
        let query = proto_vulcan_query!(|q| {
            infdrange(q, #&(0..=2)),
            distinctfd([q, q])
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_distinctfd_5() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                infdrange([x, y, z], #&(0..=2)),
                distinctfd([x, y, z]),
                q == [x, y, z],
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([0, 1, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([0, 2, 1]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 0, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 0, 1]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 2, 0]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 1, 0]));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_distinctfd_6() {
        let query = proto_vulcan_query!(|q| {
            |a, b, c, x| {
                infdrange([a, b, c], #&(1..=3)),
                distinctfd([a, b, c]),
                diseqfd(c, x),
                ltefd(b, 2),
                x == 3,
                q == [a, b, c],
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([3, 1, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([3, 2, 1]));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_distinctfd_7() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                infd([x, y, z], #&[1, 2]),
                distinctfd([x, y, z]),
            }
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }
}
