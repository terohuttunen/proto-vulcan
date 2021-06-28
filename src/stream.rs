use crate::engine::Engine;
use crate::goal::Goal;
use crate::state::State;
use crate::user::User;

#[derive(Debug)]
pub enum Lazy<U: User, E: Engine<U>> {
    Bind(LazyStream<U, E>, Goal<U, E>),
    MPlus(LazyStream<U, E>, LazyStream<U, E>),
    Pause(Box<State<U, E>>, Goal<U, E>),
    Delay(Stream<U, E>),
}

#[derive(Debug)]
pub struct LazyStream<U: User, E: Engine<U>>(Box<Lazy<U, E>>);

impl<U: User, E: Engine<U>> LazyStream<U, E> {
    pub fn bind(ls: LazyStream<U, E>, goal: Goal<U, E>) -> LazyStream<U, E> {
        LazyStream(Box::new(Lazy::Bind(ls, goal)))
    }

    pub fn mplus(ls1: LazyStream<U, E>, ls2: LazyStream<U, E>) -> LazyStream<U, E> {
        LazyStream(Box::new(Lazy::MPlus(ls1, ls2)))
    }

    pub fn pause(state: Box<State<U, E>>, goal: Goal<U, E>) -> LazyStream<U, E> {
        LazyStream(Box::new(Lazy::Pause(state, goal)))
    }

    pub fn delay(stream: Stream<U, E>) -> LazyStream<U, E> {
        LazyStream(Box::new(Lazy::Delay(stream)))
    }

    pub fn step_into(self, engine: &E) -> Stream<U, E> {
        engine.step(*self.0)
    }

    pub fn into_mature(self, engine: &E) -> Stream<U, E> {
        let mut stream = self.step_into(engine);
        loop {
            match stream {
                Stream::Lazy(lazy) => stream = lazy.step_into(engine),
                _ => return stream,
            }
        }
    }
}

#[derive(Debug)]
pub enum Stream<U: User, E: Engine<U>> {
    Empty,
    Unit(Box<State<U, E>>),
    Lazy(LazyStream<U, E>),
    Cons(Box<State<U, E>>, LazyStream<U, E>),
}

impl<U: User, E: Engine<U>> Stream<U, E> {
    pub fn is_empty(&self) -> bool {
        match self {
            Stream::Empty => true,
            _ => false,
        }
    }

    pub fn unit(u: Box<State<U, E>>) -> Stream<U, E> {
        Stream::Unit(u)
    }

    pub fn empty() -> Stream<U, E> {
        Stream::Empty
    }

    pub fn cons(a: Box<State<U, E>>, lazy: LazyStream<U, E>) -> Stream<U, E> {
        Stream::Cons(a, lazy)
    }

    pub fn lazy(lazy: LazyStream<U, E>) -> Stream<U, E> {
        Stream::Lazy(lazy)
    }

    pub fn mplus(stream: Stream<U, E>, lazy: LazyStream<U, E>) -> Stream<U, E> {
        match stream {
            Stream::Empty => Stream::lazy(lazy),
            Stream::Lazy(lazy_hat) => Stream::lazy_mplus(lazy, lazy_hat),
            Stream::Unit(a) => Stream::cons(a, lazy),
            Stream::Cons(head, lazy_hat) => Stream::cons(head, LazyStream::mplus(lazy, lazy_hat)),
        }
    }

    pub fn bind(stream: Stream<U, E>, goal: Goal<U, E>) -> Stream<U, E> {
        if goal.is_succeed() {
            stream
        } else if goal.is_fail() {
            Stream::empty()
        } else {
            match stream {
                Stream::Empty => Stream::Empty,
                Stream::Lazy(lazy) => Stream::lazy_bind(lazy, goal),
                Stream::Unit(a) => Stream::pause(a, goal),
                Stream::Cons(state, lazy) => Stream::lazy_mplus(
                    LazyStream::pause(state, goal.clone()),
                    LazyStream::bind(lazy, goal),
                ),
            }
        }
    }

    pub fn lazy_mplus(lazy: LazyStream<U, E>, lazy_hat: LazyStream<U, E>) -> Stream<U, E> {
        Stream::Lazy(LazyStream::mplus(lazy, lazy_hat))
    }

    pub fn lazy_bind(lazy: LazyStream<U, E>, goal: Goal<U, E>) -> Stream<U, E> {
        if goal.is_succeed() {
            Stream::lazy(lazy)
        } else if goal.is_fail() {
            Stream::empty()
        } else {
            Stream::Lazy(LazyStream::bind(lazy, goal))
        }
    }

    pub fn pause(state: Box<State<U, E>>, goal: Goal<U, E>) -> Stream<U, E> {
        Stream::Lazy(LazyStream::pause(state, goal))
    }

    pub fn delay(stream: Stream<U, E>) -> Stream<U, E> {
        Stream::Lazy(LazyStream::delay(stream))
    }

    pub fn is_mature(&self) -> bool {
        match self {
            Stream::Lazy(_) => false,
            _ => true,
        }
    }

    pub fn mature(&mut self, engine: &E) {
        match std::mem::replace(self, Stream::Empty) {
            Stream::Lazy(lazy) => {
                let _ = std::mem::replace(self, lazy.into_mature(engine));
            }
            s => {
                let _ = std::mem::replace(self, s);
            }
        }
    }

    pub fn into_mature(self, engine: &E) -> Stream<U, E> {
        match self {
            Stream::Lazy(lazy) => lazy.into_mature(engine),
            _ => self,
        }
    }

    pub fn next(&mut self, engine: &E) -> Option<Box<State<U, E>>> {
        self.mature(engine);
        match std::mem::replace(self, Stream::Empty) {
            Stream::Empty => return None,
            Stream::Lazy(_) => unreachable!(),
            Stream::Unit(a) => {
                return Some(a);
            }
            Stream::Cons(a, lazy) => {
                let _ = std::mem::replace(self, Stream::Lazy(lazy));
                return Some(a);
            }
        }
    }

    /// Returns a reference to next element in the stream, if any.
    pub fn peek<'a>(&'a mut self, engine: &E) -> Option<&'a Box<State<U, E>>> {
        self.mature(engine);
        match self {
            Stream::Empty => None,
            Stream::Lazy(_) => unreachable!(),
            Stream::Unit(a) | Stream::Cons(a, _) => Some(a),
        }
    }

    /// Truncates the stream leaving at most one element, and returns a reference to
    /// the remaining element if any.
    pub fn trunc<'a>(&'a mut self, engine: &E) -> Option<&'a Box<State<U, E>>> {
        self.mature(engine);
        match std::mem::replace(self, Stream::Empty) {
            Stream::Empty => (),
            Stream::Lazy(_) => unreachable!(),
            Stream::Unit(a) | Stream::Cons(a, _) => {
                let _ = std::mem::replace(self, Stream::Unit(a));
            }
        }
        self.peek(engine)
    }
}

#[derive(Debug)]
pub struct StreamEngine<U: User> {
    context: U::UserContext,
}

impl<U> Engine<U> for StreamEngine<U>
where
    U: User,
{
    fn new(context: U::UserContext) -> Self {
        StreamEngine { context }
    }

    fn start(&self, state: Box<State<U, Self>>, goal: Goal<U, Self>) -> Stream<U, Self> {
        match goal {
            Goal::Succeed => Stream::unit(state),
            Goal::Fail => Stream::empty(),
            Goal::Disj(disj) => Stream::lazy_mplus(
                LazyStream::pause(state.clone(), disj.goal_1.clone()),
                LazyStream::pause(state, disj.goal_2.clone()),
            ),
            Goal::Conj(conj) => Stream::lazy_bind(
                LazyStream::pause(state, conj.goal_1.clone()),
                conj.goal_2.clone(),
            ),
            Goal::Inner(goal) => goal.solve(self, *state),
        }
    }

    fn step(&self, lazy: Lazy<U, Self>) -> Stream<U, Self> {
        match lazy {
            Lazy::Pause(state, goal) => self.start(state, goal),
            Lazy::MPlus(s1, s2) => {
                let stream = s1.step_into(self);
                Stream::mplus(stream, s2)
            }
            Lazy::Bind(s, goal) => {
                let stream = s.step_into(self);
                Stream::bind(stream, goal)
            }
            Lazy::Delay(stream) => stream,
        }
    }

    fn context(&self) -> &U::UserContext {
        &self.context
    }
}
