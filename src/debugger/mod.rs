use crate::user::User;
use crate::engine::Engine;
use crate::state::State;
use crate::goal::Goal;
use crate::stream::{Stream, Lazy};
use std::marker::PhantomData;

pub struct Debugger<U: User, E: Engine<U>> {
    engine: E,
    _phantom: PhantomData<U>,
}

impl<U, E> Engine<U> for Debugger<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn new(context: U::UserContext) -> Self {
        let engine = E::new(context);
        Debugger {
            engine,
            _phantom: PhantomData,
        }
    }

    fn start(&self, state: Box<State<U, Self>>, goal: Goal<U, Self>) -> Stream<U, Self> {
        let stream = self.engine(state, goal);
        stream
    }

    fn step(&self, lazy: Lazy<U, Self>) -> Stream<U, Self> {
        let stream = self.engine.step(lazy);
        stream
    }

    fn context(&self) -> &U::UserContext {
        self.engine.context()
    }
}