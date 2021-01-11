/// Constrain disequality in finite domains
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::DiseqFdConstraint;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;

#[derive(Debug)]
pub struct DiseqFd<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
}

impl<U: User> DiseqFd<U> {
    pub fn new(u: LTerm<U>, v: LTerm<U>) -> Goal<U> {
        Goal::new(DiseqFd { u, v })
    }
}

impl<U: User> Solve<U> for DiseqFd<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let u = self.u.clone();
        let v = self.v.clone();
        let c = DiseqFdConstraint::new(u, v);
        Stream::from(c.run(state))
    }
}

/// Disequality relation for finite domains.
///
/// Note: The built-in syntax `x != y` does not work with finite domains.
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::*;
/// use proto_vulcan::relation::diseqfd;
/// use proto_vulcan::relation::infd;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         |x, y| {
///             infd(x, #&[1, 2]),
///             infd(y, #&[2, 3]),
///             diseqfd(x, y),
///             q == [x, y],
///         }
///     });
///     let mut iter = query.run();
///     assert!(iter.next().unwrap().q == lterm!([2, 3]));
///     assert!(iter.next().unwrap().q == lterm!([1, 2]));
///     assert!(iter.next().unwrap().q == lterm!([1, 3]));
///     assert!(iter.next().is_none())
/// }
/// ```
pub fn diseqfd<U: User>(u: LTerm<U>, v: LTerm<U>) -> Goal<U> {
    DiseqFd::new(u, v)
}

#[cfg(test)]
mod tests {
    use super::diseqfd;
    use crate::relation::infd::infd;
    use crate::*;

    #[test]
    fn test_diseqfd_1() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                infd(x, #&[1, 2]),
                infd(y, #&[2, 3]),
                infd([z, q], #&[2, 4]),
                x == y,
                diseqfd(x, z),
                q == z,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 4);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_diseqfd_2() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                x == y,
                infd(y, #&[2, 3]),
                diseqfd(x, z),
                infd([z, q], #&[2, 4]),
                q == z,
                infd(x, #&[1, 2]),
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 4);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_diseqfd_3() {
        let query = proto_vulcan_query!(|x, y| {
            infd(x, #&[1, 2]),
            infd(y, #&[2, 3]),
            x == y,
        });
        let mut iter = query.run();
        let result = iter.next().unwrap();
        assert_eq!(result.x, 2);
        assert_eq!(result.y, 2);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_diseqfd_4() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                infd([x, y, z], #&[1, 2]),
                diseqfd(x, y),
                diseqfd(x, z),
                diseqfd(y, z),
            }
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }
}
