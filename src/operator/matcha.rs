use crate::goal::Goal;
use crate::operator::conda::Conda;
use crate::operator::PatternMatchOperatorParam;
use crate::user::UserState;

pub fn matcha<U: UserState>(param: PatternMatchOperatorParam<U>) -> Goal<U> {
    Conda::from_conjunctions(param.arms)
}
