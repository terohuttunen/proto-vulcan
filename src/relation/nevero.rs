use crate::goal::Goal;
use crate::state::UserState;
use std::rc::Rc;

/// A relation that fails an unbounded number of times.
///
/// This may easily lead to divergence, and never return.
pub fn nevero<U: UserState>() -> Rc<dyn Goal<U>> {
    proto_vulcan!(loop {
        false
    })
}
