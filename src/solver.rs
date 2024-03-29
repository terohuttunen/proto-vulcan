use crate::engine::Engine;
use crate::goal::{DFSGoal, Goal};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;
use std::any::{Any, TypeId};
use std::fmt;

#[cfg(feature = "debugger")]
use crate::debugger::Debugger;

pub struct Solver<U, E>
where
    U: User,
    E: Engine<U>,
{
    engine: E,
    context: U::UserContext,
    #[cfg(feature = "debugger")]
    debugger: Debugger<U, E>,
    debug_enabled: bool,
}

impl<U, E> Solver<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(context: U::UserContext, debug_enabled: bool) -> Solver<U, E> {
        let engine = E::new();
        #[cfg(feature = "debugger")]
        let debugger = Debugger::new();
        Solver {
            engine,
            context,
            #[cfg(feature = "debugger")]
            debugger,
            debug_enabled,
        }
    }

    pub fn start(&self, goal: &Goal<U, E>, state: State<U, E>) -> Stream<U, E> {
        match goal {
            Goal::Succeed => Stream::unit(Box::new(state)),
            Goal::Fail => Stream::empty(),
            Goal::Breakpoint(_id) => {
                if self.debug_enabled {
                    // TODO: self.debugger.breakpoint(goal, &state, *id)
                }
                Stream::unit(Box::new(state))
            }
            Goal::Dynamic(dynamic) => {
                if self.debug_enabled {
                    // TODO: self.debugger.start(goal, &state)
                }
                dynamic.solve(self, state)
            }
        }
    }

    pub fn start_dfs(&self, goal: &DFSGoal<U, E>, state: State<U, E>) -> Stream<U, E> {
        match goal {
            DFSGoal::Succeed => Stream::unit(Box::new(state)),
            DFSGoal::Fail => Stream::empty(),
            DFSGoal::Breakpoint(_id) => {
                if self.debug_enabled {
                    // TODO: self.debugger.breakpoint(goal, &state, *id)
                }
                Stream::unit(Box::new(state))
            }
            DFSGoal::Dynamic(dynamic) => {
                if self.debug_enabled {
                    // TODO: self.debugger.start(goal, &state)
                }
                dynamic.solve(self, state)
            }
        }
    }

    pub fn next(&mut self, stream: &mut Stream<U, E>) -> Option<Box<State<U, E>>> {
        loop {
            #[cfg(feature = "debugger")]
            if self.debug_enabled {
                self.debugger.next_step(stream);
            }
            match std::mem::replace(stream, Stream::Empty) {
                Stream::Empty => {
                    #[cfg(feature = "debugger")]
                    if self.debug_enabled {
                        self.debugger.program_exit();
                    }
                    return None;
                }
                Stream::Unit(state) => {
                    #[cfg(feature = "debugger")]
                    if self.debug_enabled {
                        self.debugger.new_solution(stream, &state);
                    }
                    return Some(state);
                }
                Stream::Lazy(LazyStream(lazy)) => *stream = self.engine.step(self, *lazy),
                Stream::Cons(state, lazy_stream) => {
                    *stream = Stream::Lazy(lazy_stream);
                    #[cfg(feature = "debugger")]
                    if self.debug_enabled {
                        self.debugger.new_solution(stream, &state);
                    }
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
