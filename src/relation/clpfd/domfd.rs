
/// Constrains x in domain
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::solver::{Solve, Solver};
use crate::state::FiniteDomain;
use crate::state::State;
use crate::stream::Stream;

use std::rc::Rc;

#[derive(Debug)]
pub struct DomFd
{
    x: LTerm,
    domain: Rc<FiniteDomain>,
}

impl DomFd
{
    pub fn new<G: AnyGoal>(x: LTerm, domain: FiniteDomain) -> InferredGoal<G> {
        InferredGoal::new(G::dynamic(Rc::new(DomFd {
            x,
            domain: Rc::new(domain),
        })))
    }
}

impl Solve for DomFd
{
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        let xwalk = state.smap_ref().walk(&self.x).clone();
        match state.process_domain(&xwalk, Rc::clone(&self.domain) as Rc<FiniteDomain>) {
            Ok(state) => Stream::unit(Box::new(state)),
            Err(_) => Stream::empty(),
        }
    }
}
