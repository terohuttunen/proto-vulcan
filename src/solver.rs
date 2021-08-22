use crate::engine::Engine;
use crate::goal::Goal;
use crate::state::State;
use crate::stream::{Lazy, Stream};
use crate::user::User;
use std::fmt;

pub struct Solver<U, E>
where
    U: User,
    E: Engine<U>,
{
    engine: E,
    context: U::UserContext,
    debug: bool,
}

impl<U, E> Solver<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(context: U::UserContext, debug: bool) -> Solver<U, E> {
        let engine = E::new();
        Solver {
            engine,
            context,
            debug,
        }
    }

    pub fn start(&self, state: Box<State<U, E>>, goal: Goal<U, E>) -> Stream<U, E> {
        if self.debug {
            // TODO: debugger hook
        }

        self.engine.start(self, state, goal)
    }

    pub fn step(&self, lazy: Lazy<U, E>) -> Stream<U, E> {
        if self.debug {
            // TODO: debugger hook
        }
        self.engine.step(self, lazy)
    }

    pub fn context(&self) -> &U::UserContext {
        &self.context
    }
}

// A goal is a function which, given an input state, will give an output state (or infinite stream
// of output states). It encapsulates a logic query that is evaluated as infinite stream of
// states that solve the query at any given time.
pub trait Solve<U, E>: fmt::Debug
where
    U: User,
    E: Engine<U>,
{
    /// Generate a stream of solutions to the goal by applying it to some initial state.
    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E>;
}
