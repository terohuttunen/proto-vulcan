use crate::goal::Goal;
use crate::operator::conde::Conde;
use crate::operator::PatternMatchOperatorParam;
use crate::user::UserState;
use std::rc::Rc;

pub fn matche<U: UserState>(param: PatternMatchOperatorParam<U>) -> Rc<dyn Goal<U>> {
    Conde::from_conjunctions(param.arms)
}
