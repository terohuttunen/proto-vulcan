use crate::goal::Goal;
use crate::operator::conde::Conde;
use crate::operator::PatternMatchOperatorParam;
use crate::user::User;

pub fn matche<U: User>(param: PatternMatchOperatorParam<U>) -> Goal<U> {
    Conde::from_conjunctions(param.arms)
}
