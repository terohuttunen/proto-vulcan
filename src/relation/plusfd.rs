/// Constrains u + v = w finite domains
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::state::State;
use crate::state::{BaseConstraint, PlusFdConstraint};
use crate::stream::Stream;
use crate::user::UserState;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct PlusFd<U: UserState> {
    u: Rc<LTerm>,
    v: Rc<LTerm>,
    w: Rc<LTerm>,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: UserState> PlusFd<U> {
    pub fn new(u: Rc<LTerm>, v: Rc<LTerm>, w: Rc<LTerm>) -> Rc<dyn Goal<U>> {
        Rc::new(PlusFd {
            u,
            v,
            w,
            _phantom: PhantomData,
        })
    }
}

impl<U: UserState> Goal<U> for PlusFd<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        let c = Rc::new(PlusFdConstraint::new(
            self.u.clone(),
            self.v.clone(),
            self.w.clone(),
        ));
        Stream::from(c.run(state))
    }
}

pub fn plusfd<U: UserState>(u: Rc<LTerm>, v: Rc<LTerm>, w: Rc<LTerm>) -> Rc<dyn Goal<U>> {
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
