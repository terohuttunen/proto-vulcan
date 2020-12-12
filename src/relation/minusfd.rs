/// Constrains u - v = w finite domains
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::State;
use crate::state::{BaseConstraint, MinusFdConstraint};
use crate::stream::Stream;
use crate::user::User;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct MinusFd<U: User> {
    u: LTerm,
    v: LTerm,
    w: LTerm,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: User> MinusFd<U> {
    pub fn new(u: LTerm, v: LTerm, w: LTerm) -> Goal<U> {
        Goal::new(MinusFd {
            u,
            v,
            w,
            _phantom: PhantomData,
        })
    }
}

impl<U: User> Solve<U> for MinusFd<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let c = Rc::new(MinusFdConstraint::new(
            self.u.clone(),
            self.v.clone(),
            self.w.clone(),
        ));
        Stream::from(c.run(state))
    }
}

pub fn minusfd<U: User>(u: LTerm, v: LTerm, w: LTerm) -> Goal<U> {
    MinusFd::new(u, v, w)
}
