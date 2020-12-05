/// Constrains u * v = w finite domains
use crate::goal::{Goal, Solver};
use crate::lterm::LTerm;
use crate::state::State;
use crate::state::{BaseConstraint, TimesFdConstraint};
use crate::stream::Stream;
use crate::user::UserState;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct TimesFd<U: UserState> {
    u: LTerm,
    v: LTerm,
    w: LTerm,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: UserState> TimesFd<U> {
    pub fn new(u: LTerm, v: LTerm, w: LTerm) -> Goal<U> {
        Rc::new(TimesFd {
            u,
            v,
            w,
            _phantom: PhantomData,
        })
    }
}

impl<U: UserState> Solver<U> for TimesFd<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        let c = Rc::new(TimesFdConstraint::new(
            self.u.clone(),
            self.v.clone(),
            self.w.clone(),
        ));
        Stream::from(c.run(state))
    }
}

pub fn timesfd<U: UserState>(u: LTerm, v: LTerm, w: LTerm) -> Goal<U> {
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
