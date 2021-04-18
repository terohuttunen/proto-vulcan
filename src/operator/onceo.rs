use crate::engine::Engine;
use crate::goal::Goal;
use crate::operator::condu;
use crate::operator::OperatorParam;
use crate::user::User;
use proto_vulcan::prelude::*;

/// Once operator
///
/// Guarantees that the conjunction of body goals generates at most one answer.
pub fn onceo<U, E>(param: OperatorParam<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    let g = crate::operator::all::All::from_conjunctions(param.body);
    proto_vulcan!(condu { g })
}
