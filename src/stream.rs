use crate::engine::Engine;
use crate::goal::{AnyGoal, DFSGoal, Goal};
use crate::solver::Solver;
use crate::state::State;

pub enum StreamCursor<'a> {
    Stream(usize, &'a Stream),
    LazyStream(usize, &'a LazyStream),
    End,
}

pub enum StreamWalkStep<'a> {
    State(&'a State),
    LazyStream(&'a LazyStream),
    Backtrack(&'a LazyStream),
}

// Depth-first walk of the stream.
pub struct StreamWalker<'a> {
    next_pos: StreamCursor<'a>,
    deferred_stack: Vec<(usize, &'a LazyStream)>,
}

impl<'a> StreamWalker<'a> {
    pub fn new(stream: &'a Stream) -> StreamWalker<'a> {
        let deferred_stack = Vec::new();
        let next_pos = StreamCursor::Stream(0, stream);
        StreamWalker {
            next_pos,
            deferred_stack,
        }
    }

    fn backtrack(&mut self) -> Option<(usize, StreamWalkStep<'a>)> {
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
        stream: &'a Stream,
    ) -> Option<(usize, StreamWalkStep<'a>)> {
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
            Stream::Error(_) => {
                self.next_pos = StreamCursor::End;
                return None; // Error terminates the walk
            }
        };

        return Some((depth, step));
    }

    fn branch(
        &mut self,
        depth: usize,
        lazy_stream: &'a LazyStream,
    ) -> Option<(usize, StreamWalkStep<'a>)> {
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
            Lazy::Iterator(_iter) => {
                self.next_pos = StreamCursor::End;
            }
        }

        Some((depth, StreamWalkStep::LazyStream(lazy_stream)))
    }

    pub fn next(&mut self) -> Option<(usize, StreamWalkStep<'a>)> {
        match self.next_pos {
            StreamCursor::Stream(depth, s) => self.downstream(depth, s),
            StreamCursor::LazyStream(depth, l) => self.branch(depth, l),
            StreamCursor::End => self.backtrack(),
        }
    }
}

pub trait StreamIterator {
    fn clone_box(&self) -> Box<dyn StreamIterator>;

    fn next(&mut self, solver: &Solver) -> Option<Stream>;
}

impl Clone for Box<dyn StreamIterator> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl std::fmt::Debug for Box<dyn StreamIterator> {
    fn fmt(&self, fm: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fm, "StreamIterator(...)")
    }
}

#[derive(Clone, Debug)]
pub enum Lazy {
    Bind(LazyStream, Goal),
    MPlus(LazyStream, LazyStream),
    Pause(Box<State>, Goal),
    BindDFS(LazyStream, DFSGoal),
    MPlusDFS(LazyStream, LazyStream),
    PauseDFS(Box<State>, DFSGoal),
    Delay(Stream),
    Iterator(Box<dyn StreamIterator>),
}

#[derive(Clone, Debug)]
pub struct LazyStream(pub Box<Lazy>);

impl LazyStream {
    pub fn bind(ls: LazyStream, goal: Goal) -> LazyStream {
        LazyStream(Box::new(Lazy::Bind(ls, goal)))
    }

    pub fn mplus(ls1: LazyStream, ls2: LazyStream) -> LazyStream {
        LazyStream(Box::new(Lazy::MPlus(ls1, ls2)))
    }

    pub fn pause(state: Box<State>, goal: Goal) -> LazyStream {
        LazyStream(Box::new(Lazy::Pause(state, goal)))
    }

    pub fn bind_dfs(ls: LazyStream, goal: DFSGoal) -> LazyStream {
        LazyStream(Box::new(Lazy::BindDFS(ls, goal)))
    }

    pub fn mplus_dfs(ls1: LazyStream, ls2: LazyStream) -> LazyStream {
        LazyStream(Box::new(Lazy::MPlusDFS(ls1, ls2)))
    }

    pub fn pause_dfs(state: Box<State>, goal: DFSGoal) -> LazyStream {
        LazyStream(Box::new(Lazy::PauseDFS(state, goal)))
    }

    pub fn delay(stream: Stream) -> LazyStream {
        LazyStream(Box::new(Lazy::Delay(stream)))
    }

    pub fn iterator(iter: Box<dyn StreamIterator>) -> LazyStream {
        LazyStream(Box::new(Lazy::Iterator(iter)))
    }
}

#[derive(Clone, Debug)]
pub enum Stream {
    Empty,
    Unit(Box<State>),
    Lazy(LazyStream),
    Cons(Box<State>, LazyStream),
    Error(String),
}

impl Stream {
    pub fn is_empty(&self) -> bool {
        match self {
            Stream::Empty => true,
            _ => false,
        }
    }

    pub fn unit(u: Box<State>) -> Stream {
        Stream::Unit(u)
    }

    pub fn empty() -> Stream {
        Stream::Empty
    }

    pub fn error(msg: String) -> Stream {
        Stream::Error(msg)
    }

    pub fn cons(a: Box<State>, lazy: LazyStream) -> Stream {
        Stream::Cons(a, lazy)
    }

    pub fn lazy(lazy: LazyStream) -> Stream {
        Stream::Lazy(lazy)
    }

    pub fn mplus(stream: Stream, lazy: LazyStream) -> Stream {
        match stream {
            Stream::Empty => Stream::lazy(lazy),
            Stream::Lazy(lazy_hat) => Stream::lazy_mplus(lazy, lazy_hat),
            Stream::Unit(a) => Stream::cons(a, lazy),
            Stream::Cons(head, lazy_hat) => Stream::cons(head, LazyStream::mplus(lazy, lazy_hat)),
            Stream::Error(msg) => Stream::Error(msg),
        }
    }

    pub fn bind(stream: Stream, goal: Goal) -> Stream {
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
                Stream::Error(msg) => Stream::Error(msg),
            }
        }
    }

    pub fn lazy_mplus(lazy: LazyStream, lazy_hat: LazyStream) -> Stream {
        Stream::Lazy(LazyStream::mplus(lazy, lazy_hat))
    }

    pub fn pause(state: Box<State>, goal: Goal) -> Stream {
        Stream::Lazy(LazyStream::pause(state, goal))
    }

    pub fn mplus_dfs(stream: Stream, lazy: LazyStream) -> Stream {
        match stream {
            Stream::Empty => Stream::lazy(lazy),
            Stream::Lazy(lazy_hat) => Stream::lazy_mplus_dfs(lazy_hat, lazy),
            Stream::Unit(a) => Stream::cons(a, lazy),
            Stream::Cons(head, lazy_hat) => {
                Stream::cons(head, LazyStream::mplus_dfs(lazy_hat, lazy))
            }
            Stream::Error(msg) => Stream::Error(msg),
        }
    }

    pub fn bind_dfs(stream: Stream, goal: DFSGoal) -> Stream {
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
                Stream::Error(msg) => Stream::Error(msg),
            }
        }
    }

    pub fn lazy_bind(lazy: LazyStream, goal: Goal) -> Stream {
        if goal.is_succeed() {
            Stream::lazy(lazy)
        } else if goal.is_fail() {
            Stream::empty()
        } else {
            Stream::Lazy(LazyStream::bind(lazy, goal))
        }
    }

    pub fn lazy_mplus_dfs(lazy: LazyStream, lazy_hat: LazyStream) -> Stream {
        Stream::Lazy(LazyStream::mplus_dfs(lazy, lazy_hat))
    }

    pub fn lazy_bind_dfs(lazy: LazyStream, goal: DFSGoal) -> Stream {
        if goal.is_succeed() {
            Stream::lazy(lazy)
        } else if goal.is_fail() {
            Stream::empty()
        } else {
            Stream::Lazy(LazyStream::bind_dfs(lazy, goal))
        }
    }

    pub fn pause_dfs(state: Box<State>, goal: DFSGoal) -> Stream {
        Stream::Lazy(LazyStream::pause_dfs(state, goal))
    }

    pub fn delay(stream: Stream) -> Stream {
        Stream::Lazy(LazyStream::delay(stream))
    }

    pub fn iterator(iter: Box<dyn StreamIterator>) -> Stream {
        Stream::Lazy(LazyStream::iterator(iter))
    }

    pub fn is_mature(&self) -> bool {
        match self {
            Stream::Lazy(_) => false,
            _ => true,
        }
    }

    pub fn head(&self) -> Option<&Box<State>> {
        match self {
            Stream::Unit(a) | Stream::Cons(a, _) => Some(a),
            _ => None,
        }
    }

    pub fn walk<'a>(&'a self) -> StreamWalker<'a> {
        StreamWalker::new(self)
    }
}

#[derive(Debug)]
pub struct StreamEngine {
}

impl Engine for StreamEngine
{
    fn new() -> Self {
        StreamEngine {
        }
    }

    fn step(&self, solver: &Solver, lazy: Lazy) -> Stream {
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
            Lazy::PauseDFS(state, goal) => solver.start_dfs(&goal, *state),
            Lazy::Delay(stream) => stream,
            Lazy::Iterator(mut iter) => {
                // The point of iterator (at least for now) is to conserve used resources by
                // deferring stream expansion; thus using DFS search to process the returned
                // stream fully before asking for more from the iterator.
                match iter.next(solver) {
                    Some(stream) => Stream::mplus_dfs(stream, LazyStream::iterator(iter)),
                    None => Stream::empty(),
                }
            }
        }
    }
}
