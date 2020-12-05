use crate::goal::Goal;
use crate::operator::conde::Conde;
use crate::operator::PatternMatchOperatorParam;
use crate::user::UserState;

pub fn matche<U: UserState>(param: PatternMatchOperatorParam<U>) -> Goal<U> {
    Conde::from_conjunctions(param.arms)
}
