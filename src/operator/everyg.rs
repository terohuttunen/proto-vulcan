use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::operator::all::All;
use crate::operator::ForOperatorParam;
use crate::state::State;
use crate::stream::Stream;
use crate::user::UserState;
use std::fmt::Debug;
use std::rc::Rc;

pub struct Everyg<U, T>
where
    U: UserState,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a Rc<LTerm>>,
{
    coll: T,
    g: Box<dyn Fn(Rc<LTerm>) -> Rc<dyn Goal<U>>>,
}

impl<U, T> Debug for Everyg<U, T>
where
    U: UserState,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a Rc<LTerm>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Everyg()")
    }
}

impl<U, T> Everyg<U, T>
where
    U: UserState,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a Rc<LTerm>>,
{
    fn new(coll: T, g: Box<dyn Fn(Rc<LTerm>) -> Rc<dyn Goal<U>>>) -> Rc<dyn Goal<U>> {
        Rc::new(Everyg { coll, g })
    }
}

impl<U, T> Goal<U> for Everyg<U, T>
where
    U: UserState,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a Rc<LTerm>>,
{
    fn apply(&self, state: State<U>) -> Stream<U> {
        let term_iter = IntoIterator::into_iter(&self.coll);
        let goal_iter = term_iter.map(|term| (*self.g)(Rc::clone(term)));
        All::from_iter(goal_iter).apply(state)
    }
}

pub fn everyg<U, T>(param: ForOperatorParam<U, T>) -> Rc<dyn Goal<U>>
where
    U: UserState,
    T: Debug + 'static,
    for<'a> &'a T: IntoIterator<Item = &'a Rc<LTerm>>,
{
    Everyg::new(param.coll, param.g)
}
