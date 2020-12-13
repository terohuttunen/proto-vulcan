/// Constrains u + v = w
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::State;
use crate::state::PlusZConstraint;
use crate::stream::Stream;
use crate::user::User;
use std::marker::PhantomData;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct PlusZ<U: User> {
    u: LTerm,
    v: LTerm,
    w: LTerm,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: User> PlusZ<U> {
    pub fn new(u: LTerm, v: LTerm, w: LTerm) -> Goal<U> {
        Goal::new(PlusZ {
            u,
            v,
            w,
            _phantom: PhantomData,
        })
    }
}

impl<U: User> Solve<U> for PlusZ<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let c = PlusZConstraint::new(
            self.u.clone(),
            self.v.clone(),
            self.w.clone(),
        );
        Stream::from(c.run(state))
    }
}

pub fn plusz<U: User>(u: LTerm, v: LTerm, w: LTerm) -> Goal<U> {
    PlusZ::new(u, v, w)
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
