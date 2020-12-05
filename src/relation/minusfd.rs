/// Constrains u - v = w finite domains
use crate::goal::{Goal, Solver};
use crate::lterm::LTerm;
use crate::state::State;
use crate::state::{BaseConstraint, MinusFdConstraint};
use crate::stream::Stream;
use crate::user::UserState;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct MinusFd<U: UserState> {
    u: LTerm,
    v: LTerm,
    w: LTerm,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: UserState> MinusFd<U> {
    pub fn new(u: LTerm, v: LTerm, w: LTerm) -> Goal<U> {
        Rc::new(MinusFd {
            u,
            v,
            w,
            _phantom: PhantomData,
        })
    }
}

impl<U: UserState> Solver<U> for MinusFd<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        let c = Rc::new(MinusFdConstraint::new(
            self.u.clone(),
            self.v.clone(),
            self.w.clone(),
        ));
        Stream::from(c.run(state))
    }
}

pub fn minusfd<U: UserState>(u: LTerm, v: LTerm, w: LTerm) -> Goal<U> {
    MinusFd::new(u, v, w)
}
