use crate::goal::Goal;
use crate::operator::condu;
use crate::state::UserState;
use std::rc::Rc;

/// Once operator
///
/// Guarantees that the conjunction of body goals generates at most one answer.
pub fn onceo<U: UserState>(goals: &[&[Rc<dyn Goal<U>>]]) -> Rc<dyn Goal<U>> {
    let g = crate::operator::all::All::from_conjunctions(goals);
    proto_vulcan!(condu { g })
}
