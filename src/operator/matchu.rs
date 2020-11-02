use crate::goal::Goal;
use crate::operator::condu::Condu;
use crate::user::UserState;
use crate::operator::PatternMatchOperatorParam;
use std::rc::Rc;

pub fn matchu<U: UserState>(param: PatternMatchOperatorParam<U>) -> Rc<dyn Goal<U>> {
    Condu::from_conjunctions(param.arms)
}
