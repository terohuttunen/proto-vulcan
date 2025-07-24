use crate::goal::Goal;
use crate::operator::condu::Condu;
use crate::operator::PatternMatchOperatorParam;

pub fn matchu(param: PatternMatchOperatorParam<Goal>) -> Goal {
    Condu::from_conjunctions(param.arms)
}
