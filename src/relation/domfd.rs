/// Constrains x in domain
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::state::FiniteDomain;
use crate::state::{State, UserState};
use crate::stream::Stream;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DomFd<U: UserState> {
    x: Rc<LTerm>,
    domain: Rc<FiniteDomain>,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: UserState> DomFd<U> {
    pub fn new(x: Rc<LTerm>, domain: FiniteDomain) -> Rc<dyn Goal<U>> {
        Rc::new(DomFd {
            x,
            domain: Rc::new(domain),
            _phantom: PhantomData,
        })
    }
}

impl<U: UserState> Goal<U> for DomFd<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        let xwalk = Rc::clone(state.smap_ref().walk(&self.x));
        Stream::from(state.process_domain(&xwalk, Rc::clone(&self.domain) as Rc<FiniteDomain>))
    }
}
