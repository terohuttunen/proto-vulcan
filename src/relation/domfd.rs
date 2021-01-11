/// Constrains x in domain
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::FiniteDomain;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::rc::Rc;

#[derive(Debug)]
pub struct DomFd<U: User> {
    x: LTerm<U>,
    domain: Rc<FiniteDomain>,
}

impl<U: User> DomFd<U> {
    pub fn new(x: LTerm<U>, domain: FiniteDomain) -> Goal<U> {
        Goal::new(DomFd {
            x,
            domain: Rc::new(domain),
        })
    }
}

impl<U: User> Solve<U> for DomFd<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let xwalk = state.smap_ref().walk(&self.x).clone();
        Stream::from(state.process_domain(&xwalk, Rc::clone(&self.domain) as Rc<FiniteDomain>))
    }
}
