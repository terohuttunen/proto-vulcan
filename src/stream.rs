use crate::engine::Engine;
use crate::goal::Goal;
use crate::solver::Solver;
use crate::state::State;
use crate::user::User;
use std::marker::PhantomData;

pub enum StreamCursor<'a, U, E>
where
    U: User,
    E: Engine<U>,
{
    Stream(usize, &'a Stream<U, E>),
    LazyStream(usize, &'a LazyStream<U, E>),
    End,
}

pub enum StreamWalkStep<'a, U, E>
where
    U: User,
    E: Engine<U>,
{
    State(&'a State<U, E>),
    LazyStream(&'a LazyStream<U, E>),
    Backtrack(&'a LazyStream<U, E>),
}

// Depth-first walk of the stream.
pub struct StreamWalker<'a, U, E>
where
    U: User,
    E: Engine<U>,
{
    next_pos: StreamCursor<'a, U, E>,
    deferred_stack: Vec<(usize, &'a LazyStream<U, E>)>,
}

impl<'a, U, E> StreamWalker<'a, U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(stream: &'a Stream<U, E>) -> StreamWalker<'a, U, E> {
        let deferred_stack = Vec::new();
        let next_pos = StreamCursor::Stream(0, stream);
        StreamWalker {
            next_pos,
            deferred_stack,
        }
    }

    fn backtrack(&mut self) -> Option<(usize, StreamWalkStep<'a, U, E>)> {
        match self.deferred_stack.pop() {
            Some((depth, lazy_stream)) => {
                match &*lazy_stream.0 {
                    Lazy::Bind(_, _) => {}
                    Lazy::MPlus(_left, right) | Lazy::MPlusDFS(_left, right) => {
                        self.next_pos = StreamCursor::LazyStream(depth + 1, right);
                    }
                    _ => unreachable!(),
                }
                Some((depth, StreamWalkStep::Backtrack(lazy_stream)))
            }
            None => None,
        }
    }

    fn downstream(
        &mut self,
        depth: usize,
        stream: &'a Stream<U, E>,
    ) -> Option<(usize, StreamWalkStep<'a, U, E>)> {
        let step = match stream {
            Stream::Empty => {
                return self.backtrack();
            }
            Stream::Unit(a) => {
                self.next_pos = StreamCursor::End;
                // Return state now, backtrack on next call
                StreamWalkStep::State(a)
            }
            Stream::Lazy(lazy_stream) => {
                return self.branch(depth, lazy_stream);
            }
            Stream::Cons(a, lazy_stream) => {
                self.next_pos = StreamCursor::LazyStream(depth + 1, lazy_stream);
                StreamWalkStep::State(a)
            }
        };

        return Some((depth, step));
    }

    fn branch(
        &mut self,
        depth: usize,
        lazy_stream: &'a LazyStream<U, E>,
    ) -> Option<(usize, StreamWalkStep<'a, U, E>)> {
        match &*lazy_stream.0 {
            Lazy::Bind(bound_stream, _goal) => {
                self.deferred_stack.push((depth, lazy_stream));
                self.next_pos = StreamCursor::LazyStream(depth + 1, bound_stream);
            }
            Lazy::MPlus(left, _right) => {
                self.deferred_stack.push((depth, lazy_stream));
                self.next_pos = StreamCursor::LazyStream(depth + 1, left);
            }
            Lazy::Pause(_state, _goal) => {
                self.next_pos = StreamCursor::End;
            }
            Lazy::BindDFS(bound_stream, _goal) => {
                self.deferred_stack.push((depth, lazy_stream));
                self.next_pos = StreamCursor::LazyStream(depth + 1, bound_stream);
            }
            Lazy::MPlusDFS(left, _right) => {
                self.deferred_stack.push((depth, lazy_stream));
                self.next_pos = StreamCursor::LazyStream(depth + 1, left);
            }
            Lazy::PauseDFS(_state, _goal) => {
                self.next_pos = StreamCursor::End;
            }
            Lazy::Delay(stream) => {
                self.next_pos = StreamCursor::Stream(depth + 1, stream);
            }
        }

        Some((depth, StreamWalkStep::LazyStream(lazy_stream)))
    }

    pub fn next(&mut self) -> Option<(usize, StreamWalkStep<'a, U, E>)> {
        match self.next_pos {
            StreamCursor::Stream(depth, s) => self.downstream(depth, s),
            StreamCursor::LazyStream(depth, l) => self.branch(depth, l),
            StreamCursor::End => self.backtrack(),
        }
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "U: User"), Debug(bound = "U: User"))]
pub enum Lazy<U: User, E: Engine<U>> {
    Bind(LazyStream<U, E>, Goal<U, E>),
    MPlus(LazyStream<U, E>, LazyStream<U, E>),
    Pause(Box<State<U, E>>, Goal<U, E>),
    BindDFS(LazyStream<U, E>, Goal<U, E>),
    MPlusDFS(LazyStream<U, E>, LazyStream<U, E>),
    PauseDFS(Box<State<U, E>>, Goal<U, E>),
    Delay(Stream<U, E>),
}

#[derive(Derivative)]
#[derivative(Clone(bound = "U: User"), Debug(bound = "U: User"))]
pub struct LazyStream<U: User, E: Engine<U>>(pub Box<Lazy<U, E>>);

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

    pub fn bind_dfs(ls: LazyStream<U, E>, goal: Goal<U, E>) -> LazyStream<U, E> {
        LazyStream(Box::new(Lazy::BindDFS(ls, goal)))
    }

    pub fn mplus_dfs(ls1: LazyStream<U, E>, ls2: LazyStream<U, E>) -> LazyStream<U, E> {
        LazyStream(Box::new(Lazy::MPlusDFS(ls1, ls2)))
    }

    pub fn pause_dfs(state: Box<State<U, E>>, goal: Goal<U, E>) -> LazyStream<U, E> {
        LazyStream(Box::new(Lazy::PauseDFS(state, goal)))
    }

    pub fn delay(stream: Stream<U, E>) -> LazyStream<U, E> {
        LazyStream(Box::new(Lazy::Delay(stream)))
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "U: User"), Debug(bound = "U: User"))]
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

    pub fn pause(state: Box<State<U, E>>, goal: Goal<U, E>) -> Stream<U, E> {
        Stream::Lazy(LazyStream::pause(state, goal))
    }

    pub fn mplus_dfs(stream: Stream<U, E>, lazy: LazyStream<U, E>) -> Stream<U, E> {
        match stream {
            Stream::Empty => Stream::lazy(lazy),
            Stream::Lazy(lazy_hat) => Stream::lazy_mplus_dfs(lazy_hat, lazy),
            Stream::Unit(a) => Stream::cons(a, lazy),
            Stream::Cons(head, lazy_hat) => {
                Stream::cons(head, LazyStream::mplus_dfs(lazy_hat, lazy))
            }
        }
    }

    pub fn bind_dfs(stream: Stream<U, E>, goal: Goal<U, E>) -> Stream<U, E> {
        if goal.is_succeed() {
            stream
        } else if goal.is_fail() {
            Stream::empty()
        } else {
            match stream {
                Stream::Empty => Stream::Empty,
                Stream::Lazy(lazy) => Stream::lazy_bind_dfs(lazy, goal),
                Stream::Unit(a) => Stream::pause_dfs(a, goal),
                Stream::Cons(state, lazy) => Stream::lazy_mplus_dfs(
                    LazyStream::pause_dfs(state, goal.clone()),
                    LazyStream::bind_dfs(lazy, goal),
                ),
            }
        }
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

    pub fn lazy_mplus_dfs(lazy: LazyStream<U, E>, lazy_hat: LazyStream<U, E>) -> Stream<U, E> {
        Stream::Lazy(LazyStream::mplus_dfs(lazy, lazy_hat))
    }

    pub fn lazy_bind_dfs(lazy: LazyStream<U, E>, goal: Goal<U, E>) -> Stream<U, E> {
        if goal.is_succeed() {
            Stream::lazy(lazy)
        } else if goal.is_fail() {
            Stream::empty()
        } else {
            Stream::Lazy(LazyStream::bind_dfs(lazy, goal))
        }
    }

    pub fn pause_dfs(state: Box<State<U, E>>, goal: Goal<U, E>) -> Stream<U, E> {
        Stream::Lazy(LazyStream::pause_dfs(state, goal))
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

    pub fn head(&self) -> Option<&Box<State<U, E>>> {
        match self {
            Stream::Unit(a) | Stream::Cons(a, _) => Some(a),
            _ => None,
        }
    }

    pub fn walk<'a>(&'a self) -> StreamWalker<'a, U, E> {
        StreamWalker::new(self)
    }
}

#[derive(Debug)]
pub struct StreamEngine<U: User> {
    _phantom: PhantomData<U>,
}

impl<U> Engine<U> for StreamEngine<U>
where
    U: User,
{
    fn new() -> Self {
        StreamEngine {
            _phantom: PhantomData,
        }
    }

    fn step(&self, solver: &Solver<U, Self>, lazy: Lazy<U, Self>) -> Stream<U, Self> {
        match lazy {
            Lazy::MPlus(s1, s2) => {
                let stream = self.step(solver, *s1.0);
                Stream::mplus(stream, s2)
            }
            Lazy::Bind(s, goal) => {
                let stream = self.step(solver, *s.0);
                Stream::bind(stream, goal)
            }
            Lazy::Pause(state, goal) => solver.start(&goal, *state),
            Lazy::MPlusDFS(s1, s2) => {
                let stream = self.step(solver, *s1.0);
                Stream::mplus_dfs(stream, s2)
            }
            Lazy::BindDFS(s, goal) => {
                let stream = self.step(solver, *s.0);
                Stream::bind_dfs(stream, goal)
            }
            Lazy::PauseDFS(state, goal) => solver.start(&goal, *state),
            Lazy::Delay(stream) => stream,
        }
    }
}
