use crate::debugger::Debugger;
use crate::engine::Engine;
use crate::goal::Goal;
use crate::state::State;
use crate::stream::{Lazy, Stream};
use crate::user::User;
use std::any::{Any, TypeId};
use std::fmt;

pub struct Solver<U, E>
where
    U: User,
    E: Engine<U>,
{
    engine: E,
    context: U::UserContext,
    debugger: Option<Debugger<U, E>>,
}

impl<U, E> Solver<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(context: U::UserContext, debug: bool) -> Solver<U, E> {
        let engine = E::new();
        //let debugger = debug.then_some(Debugger::new());
        let debugger = if debug { Some(Debugger::new()) } else { None };
        Solver {
            engine,
            context,
            debugger,
        }
    }

    pub fn start(&self, state: Box<State<U, E>>, goal: Goal<U, E>) -> Stream<U, E> {
        match &self.debugger {
            Some(debugger) => debugger.pre_start(self, &state, &goal),
            None => (),
        }

        let stream = self.engine.start(self, state, goal);

        match &self.debugger {
            Some(debugger) => debugger.post_start(self, &stream),
            None => (),
        }

        stream
    }

    pub fn step(&self, lazy: Lazy<U, E>) -> Stream<U, E> {
        match &self.debugger {
            Some(debugger) => debugger.pre_step(self, &lazy),
            None => (),
        }

        let stream = self.engine.step(self, lazy);

        match &self.debugger {
            Some(debugger) => debugger.post_step(self, &stream),
            None => (),
        }

        stream
    }

    pub fn context(&self) -> &U::UserContext {
        &self.context
    }

    pub fn engine(&self) -> &E {
        &self.engine
    }
}

pub trait Solve<U, E>: fmt::Debug + AnySolve<U, E>
where
    U: User,
    E: Engine<U>,
{
    /// Generate a stream of solutions to the goal by applying it to some initial state.
    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E>;
}

pub trait AnySolve<U, E>: Any
where
    U: User,
    E: Engine<U>,
{
    fn as_any(&self) -> &dyn Any;
}

impl<U, E, T> AnySolve<U, E> for T
where
    U: User,
    E: Engine<U>,
    T: Solve<U, E>,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<U, E> dyn Solve<U, E>
where
    U: User,
    E: Engine<U>,
{
    #[inline]
    pub fn is<T: Solve<U, E>>(&self) -> bool {
        TypeId::of::<T>() == self.type_id()
    }

    #[inline]
    pub fn downcast_ref<T: Any + Solve<U, E>>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }
}
