use crate::engine::Engine;
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
    pub fn new<E: Engine<U>>(x: LTerm<U>, domain: FiniteDomain) -> Goal<U, E> {
        Goal::new(DomFd {
            x,
            domain: Rc::new(domain),
        })
    }
}

impl<U, E> Solve<U, E> for DomFd<U>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, engine: &E, state: State<U>) -> Stream<U, E> {
        let xwalk = state.smap_ref().walk(&self.x).clone();
        match state.process_domain(&xwalk, Rc::clone(&self.domain) as Rc<FiniteDomain>) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}
