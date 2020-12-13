/// Constrains u * v = w finite domains
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::State;
use crate::state::TimesFdConstraint;
use crate::stream::Stream;
use crate::user::User;
use std::marker::PhantomData;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct TimesFd<U: User> {
    u: LTerm,
    v: LTerm,
    w: LTerm,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: User> TimesFd<U> {
    pub fn new(u: LTerm, v: LTerm, w: LTerm) -> Goal<U> {
        Goal::new(TimesFd {
            u,
            v,
            w,
            _phantom: PhantomData,
        })
    }
}

impl<U: User> Solve<U> for TimesFd<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let c = TimesFdConstraint::new(
            self.u.clone(),
            self.v.clone(),
            self.w.clone(),
        );
        Stream::from(c.run(state))
    }
}

pub fn timesfd<U: User>(u: LTerm, v: LTerm, w: LTerm) -> Goal<U> {
    TimesFd::new(u, v, w)
}

#[cfg(test)]
mod tests {
    use super::timesfd;
    use crate::relation::infd::infdrange;
    use crate::*;

    #[test]
    fn test_timesfd_1() {
        let query = proto_vulcan_query!(|q| {
            |x, y| {
                infdrange([x, y], #&(0..=6)),
                timesfd(x, y, 6),
                q == [x, y],
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([1, 6]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 3]));
        assert_eq!(iter.next().unwrap().q, lterm!([3, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([6, 1]));
        assert!(iter.next().is_none());
    }
}
