use crate::goal::Goal;
use crate::state::State;
use crate::stream::Stream;
use crate::user::UserState;
use std::fmt;
use std::rc::Rc;

pub struct FnGoal<U: UserState> {
    f: Box<dyn Fn(State<U>) -> Stream<U>>,
}

impl<U: UserState> FnGoal<U> {
    pub fn new(f: Box<dyn Fn(State<U>) -> Stream<U>>) -> Rc<dyn Goal<U>> {
        Rc::new(FnGoal { f })
    }
}

impl<U: UserState> Goal<U> for FnGoal<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        (*self.f)(state)
    }
}

impl<U: UserState> fmt::Debug for FnGoal<U> {
    fn fmt(&self, fm: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fm, "FnGoal()")
    }
}
