use crate::goal::Goal;
use crate::operator::conda::Conda;
use crate::operator::PatternMatchOperatorParam;
use crate::user::User;

pub fn matcha<U: User>(param: PatternMatchOperatorParam<U>) -> Goal<U> {
    Conda::from_conjunctions(param.arms)
}
