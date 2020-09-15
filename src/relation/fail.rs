use crate::goal::Goal;
use crate::state::State;
use crate::stream::Stream;
use crate::user::UserState;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Fail<U: UserState> {
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: UserState> Fail<U> {
    pub fn new() -> Rc<dyn Goal<U>> {
        Rc::new(Fail {
            _phantom: PhantomData,
        })
    }
}

impl<U: UserState> Goal<U> for Fail<U> {
    fn apply(&self, _state: State<U>) -> Stream<U> {
        Stream::empty()
    }

    fn is_fail(&self) -> bool {
        true
    }
}

/// A relation that fails.
///
/// Proto-vulcan provides a built-in syntax `false` to avoid the use-clause.
///
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::*;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         conde {
///             [true, q == 1],
///             [false, q == 2],
///         }
///     });
///     let mut iter = query.run();
///     assert!(iter.next().unwrap().q == 1);
///     assert!(iter.next().is_none());
/// }
/// ```
pub fn fail<U: UserState>() -> Rc<dyn Goal<U>> {
    Fail::new()
}
