use crate::engine::Engine;
use crate::goal::Goal;
use crate::operator::conde::Conde;
use crate::operator::PatternMatchOperatorParam;
use crate::user::User;

pub fn matche<U, E>(param: PatternMatchOperatorParam<U, E, Goal<U, E>>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Conde::from_conjunctions(param.arms)
}
