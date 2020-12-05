use crate::goal::Goal;
use crate::operator::condu::Condu;
use crate::operator::PatternMatchOperatorParam;
use crate::user::User;

pub fn matchu<U: User>(param: PatternMatchOperatorParam<U>) -> Goal<U> {
    Condu::from_conjunctions(param.arms)
}
