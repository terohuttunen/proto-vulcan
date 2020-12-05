use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::operator::all::All;
use crate::relation::domfd::DomFd;
use crate::state::FiniteDomain;
use crate::user::User;
use std::ops::RangeInclusive;

/// Associates the same domain to multiple variables
pub fn infd<U: User>(u: LTerm, domain: &[isize]) -> Goal<U> {
    if u.is_list() {
        let goals = u
            .iter()
            .map(|v| DomFd::new(v.clone(), FiniteDomain::from(domain)))
            .collect();
        All::from_vec(goals)
    } else {
        DomFd::new(u, FiniteDomain::from(domain))
    }
}

pub fn infdrange<U: User>(u: LTerm, domain: &RangeInclusive<isize>) -> Goal<U> {
    if u.is_list() {
        let goals = u
            .iter()
            .map(|v| DomFd::new(v.clone(), FiniteDomain::from(domain)))
            .collect();
        All::from_vec(goals)
    } else {
        DomFd::new(u, FiniteDomain::from(domain))
    }
}

#[cfg(test)]
mod tests {
    use super::infd;
    use crate::*;

    #[test]
    fn test_infd_1() {
        let query = proto_vulcan_query!(|q| {
            |x, y| {
                q == [x, y],
                infd([x, y], #&[1]),
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([1, 1]));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_infd_2() {
        let query = proto_vulcan_query!(|q| {
            infd(q, #&[1, 2, 3, 4]),
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 4);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_infd_3() {
        let query = proto_vulcan_query!(|q| {
            infd(q, #&[1, 2]),
            q != 1,
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 2);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_infd_4() {
        let query = proto_vulcan_query!(|q| {
            |x| {
                infd([x, q], #&[1, 2]),
                q != 1,
                x == q,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 2);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_infd_5() {
        let query = proto_vulcan_query!(|x, y, z| {
            infd(x, #&[1, 2, 3]),
            infd(y, #&[3, 4, 5]),
            x == y,
            infd(z, #&[1, 3, 5, 7, 8]),
            infd(z, #&[5, 6]),
            z == 5,
        });
        let mut iter = query.run();
        let result = iter.next().unwrap();
        assert_eq!(result.x, 3);
        assert_eq!(result.y, 3);
        assert_eq!(result.z, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_infd_6() {
        let query = proto_vulcan_query!(|x, y, z| {
            infd(x, #&[1, 2, 3]),
            infd(y, #&[3, 4, 5]),
            x == y,
            infd(z, #&[1, 3, 5, 7, 8]),
            infd(z, #&[5, 6]),
            z == x,
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_infd_7() {
        let query = proto_vulcan_query!(|q| {
            |x| {
                infd(x, #&[1, 2]),
                infd(q, #&[5])
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_infd_8() {
        let query = proto_vulcan_query!(|q| {
            infd(q, #&[1, 2]),
            q == true
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_infd_9() {
        let query = proto_vulcan_query!(|q| {
            q == true,
            infd(q, #&[1, 2]),
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }
}
