use crate::engine::Engine;
use crate::goal::Goal;
use crate::solver::Solver;
use crate::state::State;
use crate::stream::{Lazy, Stream};
use crate::user::User;
use std::marker::PhantomData;
use std::rc::Rc;

mod ui;

pub struct Debugger<U, E>
where
    U: User,
    E: Engine<U>,
{
    _phantom: PhantomData<U>,
    _phantom2: PhantomData<E>,
}

impl<U, E> Debugger<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new() -> Debugger<U, E> {
        Debugger {
            _phantom: PhantomData,
            _phantom2: PhantomData,
        }
    }

    pub fn pre_start(&self, solver: &Solver<U, E>, state: &State<U, E>, goal: &Goal<U, E>) {
        if goal.is_succeed() {
        } else if goal.is_fail() {
        } else if goal.is_breakpoint() {
        }
    }

    pub fn post_start(&self, solver: &Solver<U, E>, stream: &Stream<U, E>) {}

    pub fn pre_step(&self, solver: &Solver<U, E>, lazy: &Lazy<U, E>) {}

    pub fn post_step(&self, solver: &Solver<U, E>, stream: &Stream<U, E>) {}
}
