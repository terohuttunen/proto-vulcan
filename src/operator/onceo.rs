use crate::goal::Goal;
use crate::operator::condu;
use crate::operator::OperatorParam;
use crate::user::User;

/// Once operator
///
/// Guarantees that the conjunction of body goals generates at most one answer.
pub fn onceo<U: User>(param: OperatorParam<U>) -> Goal<U> {
    let g = crate::operator::all::All::from_conjunctions(param.body);
    proto_vulcan!(condu { g })
}
