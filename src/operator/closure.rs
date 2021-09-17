use crate::engine::Engine;
use crate::goal::{AnyGoal, InferredGoal};
use crate::operator::ClosureOperatorParam;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;

pub struct Closure<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    f: Box<dyn Fn() -> G>,
    _phantom: PhantomData<U>,
    _phantom2: PhantomData<E>,
}

impl<U, E, G> Closure<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E> + 'static,
{
    pub fn new(param: ClosureOperatorParam<U, E, G>) -> InferredGoal<U, E, G> {
        InferredGoal::new(G::dynamic(Rc::new(Closure {
            f: param.f,
            _phantom: PhantomData,
            _phantom2: PhantomData,
        })))
    }
}

impl<U, E, G> Solve<U, E> for Closure<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E> + 'static,
{
    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        (*self.f)().solve(solver, state)
    }
}

impl<U, E, G> fmt::Debug for Closure<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    fn fmt(&self, fm: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Goals that are put into closure are typically recursive; therefore, evaluating
        // the goal here and trying to print it will end up in infinite recursion.
        write!(fm, "Closure(...)")
    }
}
