/// Constrains u + v = w
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::state::State;
use crate::state::{BaseConstraint, PlusZConstraint};
use crate::stream::Stream;
use crate::user::UserState;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct PlusZ<U: UserState> {
    u: Rc<LTerm>,
    v: Rc<LTerm>,
    w: Rc<LTerm>,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: UserState> PlusZ<U> {
    pub fn new(u: Rc<LTerm>, v: Rc<LTerm>, w: Rc<LTerm>) -> Rc<dyn Goal<U>> {
        Rc::new(PlusZ {
            u,
            v,
            w,
            _phantom: PhantomData,
        })
    }
}

impl<U: UserState> Goal<U> for PlusZ<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        let c = Rc::new(PlusZConstraint::new(
            self.u.clone(),
            self.v.clone(),
            self.w.clone(),
        ));
        Stream::from(c.run(state))
    }
}

pub fn plusz<U: UserState>(u: &Rc<LTerm>, v: &Rc<LTerm>, w: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    PlusZ::new(Rc::clone(u), Rc::clone(v), Rc::clone(w))
}

#[cfg(test)]
mod test {
    use super::plusz;
    use crate::*;

    #[test]
    fn test_plusz_1() {
        let query = proto_vulcan_query!(|q| { plusz(0, 1, q) });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_plusz_2() {
        let query = proto_vulcan_query!(|q| {
            |r, p| {
                plusz(1, r, q),
                plusz(r, 10, p),
                p == 15,
            }
        });

        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 6);
        assert!(iter.next().is_none());
    }
}
