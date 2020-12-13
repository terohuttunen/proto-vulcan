/// Less than or equal FD
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::State;
use crate::state::LessThanOrEqualFdConstraint;
use crate::stream::Stream;
use crate::user::User;
use std::marker::PhantomData;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct LessThanOrEqualFd<U: User> {
    u: LTerm,
    v: LTerm,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: User> LessThanOrEqualFd<U> {
    pub fn new(u: LTerm, v: LTerm) -> Goal<U> {
        Goal::new(LessThanOrEqualFd {
            u,
            v,
            _phantom: PhantomData,
        })
    }
}

impl<U: User> Solve<U> for LessThanOrEqualFd<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let c = LessThanOrEqualFdConstraint::new(
            self.u.clone(),
            self.v.clone(),
        );
        Stream::from(c.run(state))
    }
}

pub fn ltefd<U: User>(u: LTerm, v: LTerm) -> Goal<U> {
    LessThanOrEqualFd::new(u, v)
}

#[cfg(test)]
mod tests {
    use super::ltefd;
    use crate::relation::infd::{infd, infdrange};
    use crate::*;

    #[test]
    fn test_ltefd_1() {
        let query = proto_vulcan_query!(|q| {
            infdrange(q, #&(0..=10)),
            ltefd(q, 5),
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 0);
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 4);
        assert_eq!(iter.next().unwrap().q, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltefd_2() {
        let query = proto_vulcan_query!(|q| {
            |x| {
                infdrange([x, q], #&(0..=10)),
                ltefd(x, 5),
                q == x,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 0);
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 4);
        assert_eq!(iter.next().unwrap().q, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltefd_3() {
        let query = proto_vulcan_query!(|q| {
            |x| {
                ltefd(x, 5),
                infdrange([x, q], #&(0..=10)),
                q == x,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 0);
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 4);
        assert_eq!(iter.next().unwrap().q, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ltefd_4() {
        let query = proto_vulcan_query!(|q| {
            |x, y| {
                q == [x, y],
                infd(x, #&[1, 2, 3]),
                infd(y, #&[0, 1, 2, 3, 4]),
                ltefd(x, y),
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([1, 1]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 3]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 4]));
        assert_eq!(iter.next().unwrap().q, lterm!([3, 3]));
        assert_eq!(iter.next().unwrap().q, lterm!([3, 4]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 3]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 4]));
        assert!(iter.next().is_none());
    }
}
