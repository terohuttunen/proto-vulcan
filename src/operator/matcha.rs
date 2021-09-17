use crate::engine::Engine;
use crate::goal::Goal;
use crate::operator::conda::Conda;
use crate::operator::PatternMatchOperatorParam;
use crate::user::User;

pub fn matcha<U, E>(param: PatternMatchOperatorParam<U, E, Goal<U, E>>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Conda::from_conjunctions(param.arms)
}
