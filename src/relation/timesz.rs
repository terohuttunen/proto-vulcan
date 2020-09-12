/// Constrains u * v = w
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::state::State;
use crate::state::{BaseConstraint, TimesZConstraint};
use crate::stream::Stream;
use crate::user::UserState;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct TimesZ<U: UserState> {
    u: Rc<LTerm>,
    v: Rc<LTerm>,
    w: Rc<LTerm>,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: UserState> TimesZ<U> {
    pub fn new(u: Rc<LTerm>, v: Rc<LTerm>, w: Rc<LTerm>) -> Rc<dyn Goal<U>> {
        Rc::new(TimesZ {
            u,
            v,
            w,
            _phantom: PhantomData,
        })
    }
}

impl<U: UserState> Goal<U> for TimesZ<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        let c = Rc::new(TimesZConstraint::new(
            self.u.clone(),
            self.v.clone(),
            self.w.clone(),
        ));
        Stream::from(c.run(state))
    }
}

pub fn timesz<U: UserState>(u: &Rc<LTerm>, v: &Rc<LTerm>, w: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    TimesZ::new(Rc::clone(u), Rc::clone(v), Rc::clone(w))
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
