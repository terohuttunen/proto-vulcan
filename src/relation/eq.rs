use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::state::State;
use crate::stream::Stream;
use crate::user::UserState;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Eq<U: UserState> {
    u: Rc<LTerm>,
    v: Rc<LTerm>,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: UserState> Eq<U> {
    pub fn new(u: Rc<LTerm>, v: Rc<LTerm>) -> Rc<dyn Goal<U>> {
        Rc::new(Eq {
            u,
            v,
            _phantom: PhantomData,
        })
    }
}

impl<U: UserState> Goal<U> for Eq<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        Stream::from(state.unify(&self.u, &self.v))
    }
}

/// Equality relation.
///
/// Equality is one of the three core operations in miniKanren. Proto-vulcan provides a built-in
/// syntax `u == v` that avoids the use-clause: `use proto_vulcan::relation::eq`. Unlike `diseq`,
/// `eq` works also for finite-domain constraints.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::*;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         q == 5,
///     });
///     let mut iter = query.run();
///     assert!(iter.next().unwrap().q == 5);
///     assert!(iter.next().is_none());
/// }
/// ```
pub fn eq<U: UserState>(u: Rc<LTerm>, v: Rc<LTerm>) -> Rc<dyn Goal<U>> {
    Eq::new(u, v)
}
