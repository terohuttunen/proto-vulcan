use crate::goal::Goal;
use crate::state::State;
use crate::stream::Stream;
use crate::operator::ClosureOperatorParam;
use crate::user::UserState;
use std::fmt;
use std::rc::Rc;

pub struct Closure<U: UserState> {
    f: Box<dyn Fn() -> Rc<dyn Goal<U>>>,
}

impl<U: UserState> Closure<U> {
    pub fn new(param: ClosureOperatorParam<U>) -> Rc<dyn Goal<U>> {
        Rc::new(Closure { f: param.f })
    }
}

impl<U: UserState> Goal<U> for Closure<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        (*self.f)().apply(state)
    }
}

impl<U: UserState> fmt::Debug for Closure<U> {
    fn fmt(&self, fm: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Goals that are put into closure are typically recursive; therefore, evaluating
        // the goal here and trying to print it will end up in infinite recursion.
        write!(fm, "Closure(...)")
    }
}
