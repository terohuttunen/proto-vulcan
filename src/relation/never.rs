
use crate::goal::Goal;


/// A relation that fails an unbounded number of times.
///
/// This may easily lead to divergence, and never return.
pub fn never() -> Goal
{
    proto_vulcan!(loop {
        false
    })
}
