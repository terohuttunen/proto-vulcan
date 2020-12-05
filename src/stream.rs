use crate::goal::Goal;
use crate::state::{SResult, State};
use crate::user::UserState;
use std::fmt;

pub enum Thunk<U: UserState> {
    /// A delayed stream.
    Stream(Stream<U>),

    /// The goal is applied to the state to generate a delayed stream.
    Goal(Goal<U>, State<U>),

    /// Interleaving operations
    MPlus(LazyStream<U>, LazyStream<U>),
    Bind(LazyStream<U>, Goal<U>),

    /// Generic closure. This cannot be serialized.
    Closure(Box<dyn FnOnce() -> Stream<U>>),
}

impl<U: UserState> Thunk<U> {
    /// Evaluates the thunk.
    pub fn call(self) -> Stream<U> {
        match self {
            Thunk::Stream(stream) => stream,
            Thunk::Goal(goal, state) => goal.apply(state),
            Thunk::MPlus(lazy, lazy_hat) => Stream::mplus(lazy.eval(), lazy_hat),
            Thunk::Bind(lazy, goal) => Stream::bind(lazy.eval(), goal),
            Thunk::Closure(f) => f(),
        }
    }

    /// Returns a reference to next element in the stream, if any. The thunk is
    /// evaluated if necessary.
    pub fn peek(&mut self) -> Option<&Box<State<U>>> {
        let thunk = std::mem::replace(self, Thunk::Stream(Stream::Empty));
        let stream = thunk.call();
        let _ = std::mem::replace(self, Thunk::Stream(stream));
        if let Thunk::Stream(stream) = self {
            stream.peek()
        } else {
            unreachable!();
        }
    }

    /// Truncates the stream leaving at most one element, and returns a reference to
    /// the remaining element if any. The thunk is evaluated if necessary.
    pub fn trunc(&mut self) -> Option<&Box<State<U>>> {
        let thunk = std::mem::replace(self, Thunk::Stream(Stream::Empty));
        let stream = thunk.call();
        let _ = std::mem::replace(self, Thunk::Stream(stream));
        if let Thunk::Stream(stream) = self {
            stream.trunc()
        } else {
            unreachable!();
        }
    }
}

impl<U: UserState> fmt::Debug for Thunk<U> {
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
pub enum LazyStream<U: UserState> {
    Empty,
    Thunk { delay: usize, thunk: Box<Thunk<U>> },
}

impl<U: UserState> LazyStream<U> {
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

    pub fn from_goal(goal: Goal<U>, state: State<U>) -> LazyStream<U> {
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

    pub fn bind(lazy: LazyStream<U>, goal: Goal<U>) -> LazyStream<U> {
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
    pub fn eval(mut self) -> Stream<U> {
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
                thunk.call()
            }
        }
    }

    /// Returns a reference to next element in the stream, if any.
    pub fn peek(&mut self) -> Option<&Box<State<U>>> {
        match self {
            LazyStream::Empty => None,
            LazyStream::Thunk { delay: _, thunk } => thunk.peek(),
        }
    }

    /// Truncates the stream leaving at most one element, and returns a reference to
    /// the remaining element if any.
    pub fn trunc(&mut self) -> Option<&Box<State<U>>> {
        match self {
            LazyStream::Empty => None,
            LazyStream::Thunk { delay: _, thunk } => thunk.trunc(),
        }
    }
}

#[derive(Debug)]
pub enum Stream<U: UserState> {
    Empty,
    Lazy(LazyStream<U>),
    Unit(Box<State<U>>),
    Cons(Box<State<U>>, LazyStream<U>),
}

impl<U: UserState> Stream<U> {
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

    /// Create empty stream.
    pub fn empty() -> Stream<U> {
        Stream::Empty
    }

    /// Create stream with single element.
    pub fn unit(mut u: Box<State<U>>) -> Stream<U> {
        U::finalize(u.as_mut());
        Stream::Unit(u)
    }

    pub fn cons(mut a: Box<State<U>>, lazy: LazyStream<U>) -> Stream<U> {
        U::finalize(a.as_mut());
        Stream::Cons(a, lazy)
    }

    pub fn from_goal(goal: Goal<U>, mut state: State<U>) -> Stream<U> {
        U::finalize(&mut state);
        goal.apply(state)
    }

    pub fn mplus(stream: Stream<U>, lazy: LazyStream<U>) -> Stream<U> {
        match stream {
            Stream::Empty => lazy.eval(),
            Stream::Lazy(lazy_hat) => Stream::lazy_mplus(lazy, lazy_hat),
            Stream::Unit(a) => Stream::cons(a, lazy),
            Stream::Cons(head, lazy_hat) => Stream::cons(head, LazyStream::mplus(lazy, lazy_hat)),
        }
    }

    pub fn lazy_mplus(lazy: LazyStream<U>, lazy_hat: LazyStream<U>) -> Stream<U> {
        Stream::Lazy(LazyStream::mplus(lazy, lazy_hat))
    }

    pub fn bind(stream: Stream<U>, goal: Goal<U>) -> Stream<U> {
        if goal.is_succeed() {
            stream
        } else if goal.is_fail() {
            Stream::empty()
        } else {
            match stream {
                Stream::Empty => Stream::empty(),
                Stream::Lazy(lazy) => Stream::lazy_bind(lazy, goal),
                Stream::Unit(a) => goal.apply(*a),
                Stream::Cons(head, lazy) => {
                    Stream::mplus(goal.apply(*head), LazyStream::bind(lazy, goal))
                }
            }
        }
    }

    pub fn lazy_bind(lazy: LazyStream<U>, goal: Goal<U>) -> Stream<U> {
        if goal.is_succeed() {
            Stream::Lazy(lazy)
        } else if goal.is_fail() {
            Stream::empty()
        } else {
            Stream::Lazy(LazyStream::bind(lazy, goal))
        }
    }

    /// Returns the next element from the stream.
    pub fn next(&mut self) -> Option<Box<State<U>>> {
        loop {
            match std::mem::replace(self, Stream::Empty) {
                Stream::Empty => return None,
                Stream::Lazy(lazy) => {
                    let _ = std::mem::replace(self, lazy.eval());
                }
                Stream::Unit(a) => {
                    return Some(a);
                }
                Stream::Cons(a, lazy) => {
                    let _ = std::mem::replace(self, lazy.eval());
                    return Some(a);
                }
            }
        }
    }

    /// Returns a reference to next element in the stream, if any.
    pub fn peek(&mut self) -> Option<&Box<State<U>>> {
        match self {
            Stream::Empty => None,
            Stream::Lazy(lazy) => lazy.peek(),
            Stream::Unit(a) => Some(a),
            Stream::Cons(a, _) => Some(a),
        }
    }

    /// Truncates the stream leaving at most one element, and returns a reference to
    /// the remaining element if any.
    pub fn trunc(&mut self) -> Option<&Box<State<U>>> {
        match self {
            Stream::Empty => None,
            Stream::Lazy(lazy) => lazy.trunc(),
            Stream::Unit(a) => Some(a),
            Stream::Cons(_, _) => {
                if let Stream::Cons(a, _) = std::mem::replace(self, Stream::Empty) {
                    let _ = std::mem::replace(self, Stream::Unit(a));
                    self.peek()
                } else {
                    unreachable!();
                }
            }
        }
    }
}

impl<U: UserState> Iterator for Stream<U> {
    type Item = Box<State<U>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

impl<U: UserState> From<SResult<U>> for Stream<U> {
    fn from(u: SResult<U>) -> Stream<U> {
        match u {
            Ok(u) => Stream::unit(Box::new(u)),
            Err(_) => Stream::empty(),
        }
    }
}

impl<U: UserState> From<State<U>> for Stream<U> {
    fn from(u: State<U>) -> Stream<U> {
        Stream::unit(Box::new(u))
    }
}
