use crate::goal::Goal;
use crate::operator::conde::Conde;
use crate::user::UserState;
use crate::operator::PatternMatchOperatorParam;
use std::rc::Rc;

pub fn matche<U: UserState>(param: PatternMatchOperatorParam<U>) -> Rc<dyn Goal<U>> {
    Conde::from_conjunctions(param.arms)
}
