use crate::goal::{Goal, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Succeed<U: User> {
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: User> Succeed<U> {
    pub fn new() -> Goal<U> {
        Rc::new(Succeed {
            _phantom: PhantomData,
        })
    }
}

impl<U: User> Solver<U> for Succeed<U> {
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
pub fn succeed<U: User>() -> Goal<U> {
    Succeed::new()
}
