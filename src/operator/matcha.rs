use crate::goal::Goal;
use crate::operator::conda::Conda;
use crate::operator::PatternMatchOperatorParam;
use crate::user::UserState;
use std::rc::Rc;

pub fn matcha<U: UserState>(param: PatternMatchOperatorParam<U>) -> Rc<dyn Goal<U>> {
    Conda::from_conjunctions(param.arms)
}
