use crate::goal::Goal;
use crate::operator::condu;
use crate::operator::OperatorParam;
use crate::user::UserState;
use std::rc::Rc;

/// Once operator
///
/// Guarantees that the conjunction of body goals generates at most one answer.
pub fn onceo<U: UserState>(param: OperatorParam<U>) -> Rc<dyn Goal<U>> {
    let g = crate::operator::all::All::from_conjunctions(param.body);
    proto_vulcan!(condu { g })
}
