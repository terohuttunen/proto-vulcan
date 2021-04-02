use crate::engine::Engine;
use crate::goal::Goal;
use crate::operator::condu::Condu;
use crate::operator::PatternMatchOperatorParam;
use crate::user::User;

pub fn matchu<U, E>(param: PatternMatchOperatorParam<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Condu::from_conjunctions(param.arms)
}
