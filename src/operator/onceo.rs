use crate::goal::Goal;
use crate::operator::condu;
use crate::operator::OperatorParam;
use proto_vulcan::prelude::*;

/// Once operator
///
/// Guarantees that the conjunction of body goals generates at most one answer.
pub fn onceo(param: OperatorParam<Goal>) -> Goal {
    let g = crate::operator::conj::Conj::from_conjunctions(param.body);
    proto_vulcan!(condu { g })
}
