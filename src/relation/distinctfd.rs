/// distinctfd finite domain constraint
use crate::goal::{Goal, Solver};
use crate::lterm::LTerm;
use crate::state::State;
use crate::state::{BaseConstraint, DistinctFdConstraint};
use crate::stream::Stream;
use crate::user::User;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DistinctFd<U: User> {
    u: LTerm,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: User> DistinctFd<U> {
    pub fn new(u: LTerm) -> Goal<U> {
        Rc::new(DistinctFd {
            u,
            _phantom: PhantomData,
        })
    }
}

impl<U: User> Solver<U> for DistinctFd<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let c = Rc::new(DistinctFdConstraint::new(self.u.clone()));
        Stream::from(c.run(state))
    }
}

pub fn distinctfd<U: User>(u: LTerm) -> Goal<U> {
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
