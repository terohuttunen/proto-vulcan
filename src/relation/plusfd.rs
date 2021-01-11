/// Constrains u + v = w finite domains
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::PlusFdConstraint;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;

#[derive(Debug)]
pub struct PlusFd<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
    w: LTerm<U>,
}

impl<U: User> PlusFd<U> {
    pub fn new(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Goal<U> {
        Goal::new(PlusFd { u, v, w })
    }
}

impl<U: User> Solve<U> for PlusFd<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let c = PlusFdConstraint::new(self.u.clone(), self.v.clone(), self.w.clone());
        Stream::from(c.run(state))
    }
}

pub fn plusfd<U: User>(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Goal<U> {
    PlusFd::new(u, v, w)
}

#[cfg(test)]
mod tests {
    use super::plusfd;
    use crate::relation::diseqfd::diseqfd;
    use crate::relation::infd::infdrange;
    use crate::*;

    #[test]
    fn test_plusfd_1() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                infdrange([x, y, z, q], #&(0..=9)),
                diseqfd(x, y),
                diseqfd(y, z),
                diseqfd(x, z),
                x == 2,
                q == 3,
                plusfd(y, 3, z),
            }
        });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 3);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_plusfd_2() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                q == [x, y, z],
                infdrange([x, y, z], #&(0..=3)),
                plusfd(x, y, z),
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([0, 0, 0]));
        assert_eq!(iter.next().unwrap().q, lterm!([0, 1, 1]));
        assert_eq!(iter.next().unwrap().q, lterm!([0, 2, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 0, 1]));
        assert_eq!(iter.next().unwrap().q, lterm!([0, 3, 3]));
        assert_eq!(iter.next().unwrap().q, lterm!([3, 0, 3]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 1, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 2, 3]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 0, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 1, 3]));
        assert!(iter.next().is_none());
    }
}
