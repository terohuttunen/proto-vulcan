use crate::engine::Engine;
use crate::goal::Goal;
use crate::state::State;
use crate::user::User;
use std::fmt;

pub enum Thunk<U: User> {
    /// A delayed stream.
    Stream(Stream<U>),

    /// The goal is applied to the state to generate a delayed stream.
    Goal(Goal<U, StreamEngine<U>>, State<U>),

    /// Interleaving operations
    MPlus(LazyStream<U>, LazyStream<U>),
    Bind(LazyStream<U>, Goal<U, StreamEngine<U>>),

    /// Generic closure. This cannot be serialized.
    Closure(Box<dyn FnOnce() -> Stream<U>>),
}

impl<U: User> Thunk<U> {
    /// Evaluates the thunk.
    pub fn call(self, engine: &StreamEngine<U>) -> Stream<U> {
        match self {
            Thunk::Stream(stream) => stream,
            Thunk::Goal(goal, state) => goal.solve(engine, state),
            Thunk::MPlus(lazy, lazy_hat) => engine.mplus(engine.force(lazy), lazy_hat),
            Thunk::Bind(lazy, goal) => engine.mbind(engine.force(lazy), goal),
            Thunk::Closure(f) => f(),
        }
    }

    /// Returns a reference to next element in the stream, if any. The thunk is
    /// evaluated if necessary.
    pub fn peek<'a>(&mut self, engine: &'a StreamEngine<U>) -> Option<&Box<State<U>>> {
        let thunk = std::mem::replace(self, Thunk::Stream(engine.mzero()));
        let stream = thunk.call(engine);
        let _ = std::mem::replace(self, Thunk::Stream(stream));
        if let Thunk::Stream(stream) = self {
            engine.peek(stream)
        } else {
            unreachable!();
        }
    }

    /// Truncates the stream leaving at most one element, and returns a reference to
    /// the remaining element if any. The thunk is evaluated if necessary.
    pub fn trunc<'a>(&mut self, engine: &'a StreamEngine<U>) -> Option<&Box<State<U>>> {
        let thunk = std::mem::replace(self, Thunk::Stream(engine.mzero()));
        let stream = thunk.call(engine);
        let _ = std::mem::replace(self, Thunk::Stream(stream));
        if let Thunk::Stream(stream) = self {
            engine.trunc(stream)
        } else {
            unreachable!();
        }
    }
}

impl<U: User> fmt::Debug for Thunk<U> {
    fn fmt(&self, fm: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Thunk::Stream(stream) => fm.debug_tuple("Stream").field(stream).finish(),
            Thunk::Goal(goal, state) => fm.debug_tuple("Goal").field(goal).field(state).finish(),
            Thunk::MPlus(lazy, lazy_hat) => {
                fm.debug_tuple("MPlus").field(lazy).field(lazy_hat).finish()
            }
            Thunk::Bind(lazy, goal) => fm.debug_tuple("Bind").field(lazy).field(goal).finish(),
            Thunk::Closure(_) => fm.debug_tuple("Closure").finish(),
        }
    }
}

#[derive(Debug)]
pub enum LazyStream<U: User> {
    Empty,
    Thunk { delay: usize, thunk: Box<Thunk<U>> },
}

impl<U: User> LazyStream<U> {
    pub fn is_empty(&self) -> bool {
        match self {
            LazyStream::Empty => true,
            _ => false,
        }
    }

    pub fn from_closure(f: Box<dyn FnOnce() -> Stream<U>>) -> LazyStream<U> {
        LazyStream::Thunk {
            delay: 0,
            thunk: Box::new(Thunk::Closure(f)),
        }
    }

    pub fn from_goal(goal: Goal<U, StreamEngine<U>>, state: State<U>) -> LazyStream<U> {
        if goal.is_fail() {
            LazyStream::Empty
        } else {
            LazyStream::Thunk {
                delay: 0,
                thunk: Box::new(Thunk::Goal(goal, state)),
            }
        }
    }

    pub fn from_stream(stream: Stream<U>) -> LazyStream<U> {
        if stream.is_empty() {
            LazyStream::Empty
        } else {
            LazyStream::Thunk {
                delay: 0,
                thunk: Box::new(Thunk::Stream(stream)),
            }
        }
    }

    pub fn bind(lazy: LazyStream<U>, goal: Goal<U, StreamEngine<U>>) -> LazyStream<U> {
        if goal.is_succeed() {
            lazy
        } else if goal.is_fail() {
            LazyStream::Empty
        } else {
            LazyStream::Thunk {
                delay: 0,
                thunk: Box::new(Thunk::Bind(lazy, goal)),
            }
        }
    }

    pub fn mplus(lazy: LazyStream<U>, lazy_hat: LazyStream<U>) -> LazyStream<U> {
        LazyStream::Thunk {
            delay: 0,
            thunk: Box::new(Thunk::MPlus(lazy, lazy_hat)),
        }
    }

    pub fn with_delay(self, delay: usize) -> LazyStream<U> {
        match self {
            LazyStream::Empty => LazyStream::Empty,
            LazyStream::Thunk { delay: d, thunk } => LazyStream::Thunk {
                delay: d + delay,
                thunk,
            },
        }
    }

    /// Evaluates a stream from lazy stream.
    pub fn eval(mut self, engine: &StreamEngine<U>) -> Stream<U> {
        match self {
            LazyStream::Empty => Stream::Empty,
            LazyStream::Thunk {
                ref mut delay,
                thunk,
            } => {
                if *delay > 0 {
                    *delay = *delay - 1;
                    return Stream::Lazy(LazyStream::Thunk {
                        delay: *delay,
                        thunk,
                    });
                }
                thunk.call(engine)
            }
        }
    }

    /// Returns a reference to next element in the stream, if any.
    pub fn peek<'a>(&mut self, engine: &'a StreamEngine<U>) -> Option<&Box<State<U>>> {
        match self {
            LazyStream::Empty => None,
            LazyStream::Thunk { delay: _, thunk } => thunk.peek(engine),
        }
    }

    /// Truncates the stream leaving at most one element, and returns a reference to
    /// the remaining element if any.
    pub fn trunc<'a>(&mut self, engine: &'a StreamEngine<U>) -> Option<&Box<State<U>>> {
        match self {
            LazyStream::Empty => None,
            LazyStream::Thunk { delay: _, thunk } => thunk.trunc(engine),
        }
    }
}

#[derive(Debug)]
pub enum Stream<U: User> {
    Empty,
    Lazy(LazyStream<U>),
    Unit(Box<State<U>>),
    Cons(Box<State<U>>, LazyStream<U>),
}

impl<U: User> Stream<U> {
    pub fn is_empty(&self) -> bool {
        match self {
            Stream::Empty => true,
            Stream::Lazy(lazy) => lazy.is_empty(),
            _ => false,
        }
    }

    pub fn with_delay(self, n: usize) -> Stream<U> {
        Stream::Lazy(LazyStream::from_stream(self).with_delay(n))
    }

    pub fn cons(mut a: Box<State<U>>, lazy: LazyStream<U>) -> Stream<U> {
        U::finalize(a.as_mut());
        Stream::Cons(a, lazy)
    }

    pub fn from_goal(
        engine: &StreamEngine<U>,
        goal: Goal<U, StreamEngine<U>>,
        mut state: State<U>,
    ) -> Stream<U> {
        U::finalize(&mut state);
        goal.solve(engine, state)
    }

    pub fn lazy_mplus(lazy: LazyStream<U>, lazy_hat: LazyStream<U>) -> Stream<U> {
        Stream::Lazy(LazyStream::mplus(lazy, lazy_hat))
    }

    pub fn lazy_bind(lazy: LazyStream<U>, goal: Goal<U, StreamEngine<U>>) -> Stream<U> {
        if goal.is_succeed() {
            Stream::Lazy(lazy)
        } else if goal.is_fail() {
            Stream::Empty
        } else {
            Stream::Lazy(LazyStream::bind(lazy, goal))
        }
    }
}

#[derive(Debug)]
pub struct StreamEngine<U: User> {
    _phantom: std::marker::PhantomData<U>,
}

impl<U: User> Engine<U> for StreamEngine<U> {
    type LazyStream = LazyStream<U>;
    type Stream = Stream<U>;

    fn new() -> Self {
        StreamEngine {
            _phantom: std::marker::PhantomData,
        }
    }

    // Identity for mplus, i.e. empty stream
    fn mzero(&self) -> Self::Stream {
        Stream::Empty
    }

    // Stream with single element
    fn munit(&self, state: State<U>) -> Self::Stream {
        Stream::Unit(Box::new(state))
    }

    fn is_empty(&self, stream: &Self::Stream) -> bool {
        match stream {
            Stream::Empty => true,
            Stream::Lazy(lazy) => lazy.is_empty(),
            _ => false,
        }
    }

    // Returns a stream of solutions from solving `goal` in each solution of `stream`.
    // Conjunction. Each application of the `goal` to states of the stream may result in
    // failure with zero solutions, or success with any non-zero number of solutions.
    fn mbind(&self, stream: Self::Stream, goal: Goal<U, Self>) -> Self::Stream {
        if goal.is_succeed() {
            stream
        } else if goal.is_fail() {
            Stream::Empty
        } else {
            match stream {
                Stream::Empty => Stream::Empty,
                Stream::Lazy(lazy) => Stream::lazy_bind(lazy, goal),
                Stream::Unit(a) => goal.solve(self, *a),
                Stream::Cons(head, lazy) => {
                    self.mplus(goal.solve(self, *head), LazyStream::bind(lazy, goal))
                }
            }
        }
    }

    fn mplus(&self, stream: Self::Stream, lazy: Self::LazyStream) -> Self::Stream {
        match stream {
            Stream::Empty => self.force(lazy),
            Stream::Lazy(lazy_hat) => Stream::lazy_mplus(lazy, lazy_hat),
            Stream::Unit(a) => Stream::cons(a, lazy),
            Stream::Cons(head, lazy_hat) => Stream::cons(head, LazyStream::mplus(lazy, lazy_hat)),
        }
    }

    fn lazy(&self, goal: Goal<U, Self>, state: State<U>) -> Self::LazyStream {
        LazyStream::from_goal(goal, state)
    }

    fn next(&self, stream: &mut Self::Stream) -> Option<Box<State<U>>> {
        loop {
            match std::mem::replace(stream, Stream::Empty) {
                Stream::Empty => return None,
                Stream::Lazy(lazy) => {
                    let _ = std::mem::replace(stream, lazy.eval(self));
                }
                Stream::Unit(a) => {
                    return Some(a);
                }
                Stream::Cons(a, lazy) => {
                    let _ = std::mem::replace(stream, lazy.eval(self));
                    return Some(a);
                }
            }
        }
    }

    /// Returns a reference to next element in the stream, if any.
    fn peek<'a>(&self, stream: &'a mut Self::Stream) -> Option<&'a Box<State<U>>> {
        match stream {
            Stream::Empty => None,
            Stream::Lazy(lazy) => lazy.peek(self),
            Stream::Unit(a) => Some(a),
            Stream::Cons(a, _) => Some(a),
        }
    }

    /// Truncates the stream leaving at most one element, and returns a reference to
    /// the remaining element if any.
    fn trunc<'a>(&self, stream: &'a mut Self::Stream) -> Option<&'a Box<State<U>>> {
        match stream {
            Stream::Empty => None,
            Stream::Lazy(lazy) => lazy.trunc(self),
            Stream::Unit(a) => Some(a),
            Stream::Cons(_, _) => {
                if let Stream::Cons(a, _) = std::mem::replace(stream, Stream::Empty) {
                    let _ = std::mem::replace(stream, Stream::Unit(a));
                    self.peek(stream)
                } else {
                    unreachable!();
                }
            }
        }
    }

    fn delay(&self, stream: Self::Stream) -> Self::LazyStream {
        LazyStream::from_stream(stream)
    }

    // Packs lazy stream into stream without evaluating it
    fn inc(&self, lazy: Self::LazyStream) -> Self::Stream {
        Stream::Lazy(lazy)
    }

    // Evaluate lazy stream
    fn force(&self, lazy: Self::LazyStream) -> Self::Stream {
        lazy.eval(self)
    }
}
