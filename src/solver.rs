//use crate::debugger::Debugger;
use crate::engine::Engine;
use crate::goal::Goal;
use crate::state::State;
use crate::stream::{LazyStream, Stream};
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
    //debugger: Option<Debugger<U, E>>,
}

impl<U, E> Solver<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(context: U::UserContext, debug: bool) -> Solver<U, E> {
        let engine = E::new();
        //let debugger = debug.then_some(Debugger::new());
        //let debugger = if debug { Some(Debugger::new()) } else { None };
        Solver {
            engine,
            context,
            //debugger,
        }
    }

    pub fn start(&self, goal: &Goal<U, E>, initial_state: State<U, E>) -> Stream<U, E> {
        self.engine.start(self, Box::new(initial_state), goal)
    }

    pub fn next(&self, stream: &mut Stream<U, E>) -> Option<Box<State<U, E>>> {
        loop {
            // TODO: Debugger step hook
            match std::mem::replace(stream, Stream::Empty) {
                Stream::Empty => {
                    // TODO: Debugger program exit hook
                    return None;
                }
                Stream::Unit(state) => {
                    // TODO: Debugger new solution hook
                    return Some(state);
                }
                Stream::Lazy(LazyStream(lazy)) => *stream = self.engine.step(self, *lazy),
                Stream::Cons(state, lazy_stream) => {
                    *stream = Stream::Lazy(lazy_stream);
                    // TODO: Debugger new solution hook
                    return Some(state);
                }
            }
        }
    }

    /// Returns a reference to next element in the stream, if any.
    pub fn peek<'a>(&self, stream: &'a mut Stream<U, E>) -> Option<&'a Box<State<U, E>>> {
        loop {
            match stream {
                Stream::Lazy(_) => {
                    if let Stream::Lazy(LazyStream(lazy)) = std::mem::replace(stream, Stream::Empty)
                    {
                        *stream = self.engine.step(self, *lazy);
                    }
                }
                _ => return stream.head(),
            }
        }
    }

    /// Truncates the stream leaving at most one element, and returns a reference to
    /// the remaining element if any.
    pub fn trunc<'a>(&self, stream: &'a mut Stream<U, E>) -> Option<&'a Box<State<U, E>>> {
        loop {
            match std::mem::replace(stream, Stream::Empty) {
                Stream::Empty => return None,
                Stream::Lazy(LazyStream(lazy)) => {
                    *stream = self.engine.step(self, *lazy);
                }
                Stream::Unit(a) | Stream::Cons(a, _) => {
                    *stream = Stream::Unit(a);
                    return stream.head();
                }
            }
        }
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
