/// Constrains u - v = w finite domains
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::state::{BaseConstraint, MinusFdConstraint};
use crate::state::{State, UserState};
use crate::stream::Stream;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct MinusFd<U: UserState> {
    u: Rc<LTerm>,
    v: Rc<LTerm>,
    w: Rc<LTerm>,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: UserState> MinusFd<U> {
    pub fn new(u: Rc<LTerm>, v: Rc<LTerm>, w: Rc<LTerm>) -> Rc<dyn Goal<U>> {
        Rc::new(MinusFd {
            u,
            v,
            w,
            _phantom: PhantomData,
        })
    }
}

impl<U: UserState> Goal<U> for MinusFd<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        let c = Rc::new(MinusFdConstraint::new(
            self.u.clone(),
            self.v.clone(),
            self.w.clone(),
        ));
        Stream::from(c.run(state))
    }
}

pub fn minusfd<U: UserState>(u: &Rc<LTerm>, v: &Rc<LTerm>, w: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    MinusFd::new(Rc::clone(u), Rc::clone(v), Rc::clone(w))
}
