/// Constrains u * v = w
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::State;
use crate::state::{BaseConstraint, TimesZConstraint};
use crate::stream::Stream;
use crate::user::User;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct TimesZ<U: User> {
    u: LTerm,
    v: LTerm,
    w: LTerm,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: User> TimesZ<U> {
    pub fn new(u: LTerm, v: LTerm, w: LTerm) -> Goal<U> {
        Goal::new(TimesZ {
            u,
            v,
            w,
            _phantom: PhantomData,
        })
    }
}

impl<U: User> Solve<U> for TimesZ<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let c = Rc::new(TimesZConstraint::new(
            self.u.clone(),
            self.v.clone(),
            self.w.clone(),
        ));
        Stream::from(c.run(state))
    }
}

pub fn timesz<U: User>(u: LTerm, v: LTerm, w: LTerm) -> Goal<U> {
    TimesZ::new(u, v, w)
}

#[cfg(test)]
mod test {
    use super::timesz;
    use crate::*;

    #[test]
    fn test_timesz_1() {
        let query = proto_vulcan_query!(|q| { timesz(4, 2, q) });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 8);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_timesz_2() {
        let query = proto_vulcan_query!(|q| {
            |r, p| {
                timesz(2, r, q),
                timesz(r, 10, p),
                p == 20,
            }
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 4);
        assert!(iter.next().is_none());
    }
}
