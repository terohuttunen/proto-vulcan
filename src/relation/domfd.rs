use crate::engine::Engine;
/// Constrains x in domain
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::state::FiniteDomain;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug(bound="U: User"))]
pub struct DomFd<U, E>
where
    U: User,
    E: Engine<U>,
{
    x: LTerm<U, E>,
    domain: Rc<FiniteDomain>,
}

impl<U, E> DomFd<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(x: LTerm<U, E>, domain: FiniteDomain) -> Goal<U, E> {
        Goal::new(DomFd {
            x,
            domain: Rc::new(domain),
        })
    }
}

impl<U, E> Solve<U, E> for DomFd<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, _engine: &E, state: State<U, E>) -> Stream<U, E> {
        let xwalk = state.smap_ref().walk(&self.x).clone();
        match state.process_domain(&xwalk, Rc::clone(&self.domain) as Rc<FiniteDomain>) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}
