/// Constrains x in domain
use crate::goal::{Goal, Solver};
use crate::lterm::LTerm;
use crate::state::FiniteDomain;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DomFd<U: User> {
    x: LTerm,
    domain: Rc<FiniteDomain>,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: User> DomFd<U> {
    pub fn new(x: LTerm, domain: FiniteDomain) -> Goal<U> {
        Goal::new(DomFd {
            x,
            domain: Rc::new(domain),
            _phantom: PhantomData,
        })
    }
}

impl<U: User> Solver<U> for DomFd<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let xwalk = state.smap_ref().walk(&self.x).clone();
        Stream::from(state.process_domain(&xwalk, Rc::clone(&self.domain) as Rc<FiniteDomain>))
    }
}
