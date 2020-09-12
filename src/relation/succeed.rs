use crate::goal::Goal;
use crate::state::State;
use crate::stream::Stream;
use crate::user::UserState;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Succeed<U: UserState> {
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: UserState> Succeed<U> {
    pub fn new() -> Rc<dyn Goal<U>> {
        Rc::new(Succeed {
            _phantom: PhantomData,
        })
    }
}

impl<U: UserState> Goal<U> for Succeed<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        Stream::from(state)
    }

    fn is_succeed(&self) -> bool {
        true
    }
}

/// A relation that succeeds.
///
/// Proto-vulcan provides a built-in syntax `true` to avoid the use-clause.
///
/// ```rust
/// # #![recursion_limit = "512"]
/// use proto_vulcan::*;
/// let query = proto_vulcan_query!(|q| {
///     conde {
///         [true, q == 1],
///         [false, q == 2],
///     }
/// });
/// let mut iter = query.run();
/// assert!(iter.next().unwrap().q == 1);
/// assert!(iter.next().is_none());
/// ```
pub fn succeed<U: UserState>() -> Rc<dyn Goal<U>> {
    Succeed::new()
}
