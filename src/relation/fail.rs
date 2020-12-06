use crate::goal::{Goal, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Fail<U: User> {
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: User> Fail<U> {
    pub fn new() -> Goal<U> {
        Rc::new(Fail {
            _phantom: PhantomData,
        })
    }
}

impl<U: User> Solver<U> for Fail<U> {
    fn solve(&self, _state: State<U>) -> Stream<U> {
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
pub fn fail<U: User>() -> Goal<U> {
    Fail::new()
}
