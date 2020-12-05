use crate::goal::Goal;
use crate::operator::condu::Condu;
use crate::operator::PatternMatchOperatorParam;
use crate::user::UserState;

pub fn matchu<U: UserState>(param: PatternMatchOperatorParam<U>) -> Goal<U> {
    Condu::from_conjunctions(param.arms)
}
