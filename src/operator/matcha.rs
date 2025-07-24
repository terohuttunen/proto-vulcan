use crate::goal::Goal;
use crate::operator::conda::Conda;
use crate::operator::PatternMatchOperatorParam;

pub fn matcha(param: PatternMatchOperatorParam<Goal>) -> Goal {
    Conda::from_conjunctions(param.arms)
}
