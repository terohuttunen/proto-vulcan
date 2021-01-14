/// Constrains u - v = w finite domains
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::{MinusFdConstraint, State};
use crate::stream::Stream;
use crate::user::User;

#[derive(Debug)]
pub struct MinusFd<U: User> {
    u: LTerm<U>,
    v: LTerm<U>,
    w: LTerm<U>,
}

impl<U: User> MinusFd<U> {
    pub fn new(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Goal<U> {
        Goal::new(MinusFd { u, v, w })
    }
}

impl<U: User> Solve<U> for MinusFd<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let c = MinusFdConstraint::new(self.u.clone(), self.v.clone(), self.w.clone());
        Stream::from(c.run(state))
    }
}

pub fn minusfd<U: User>(u: LTerm<U>, v: LTerm<U>, w: LTerm<U>) -> Goal<U> {
    MinusFd::new(u, v, w)
}
