use crate::goal::Goal;
use crate::user::UserState;

/// A relation that fails an unbounded number of times.
///
/// This may easily lead to divergence, and never return.
pub fn nevero<U: UserState>() -> Goal<U> {
    proto_vulcan!(loop {
        false
    })
}
