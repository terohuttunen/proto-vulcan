use crate::goal::Goal;
use crate::state::State;
use crate::stream::StreamEngine;
use crate::user::User;
use std::fmt::Debug;

pub type DefaultEngine<U> = StreamEngine<U>;

pub trait Engine<U>: Debug + Sized + 'static
where
    U: User,
{
    // TODO: remove Debug-trait
    // Stream types are opaque and all operations are done via an engine instance.
    // This way a (lazy-)stream does not need to have a reference to its engine
    // if it needs one.
    type LazyStream;
    type Stream;

    fn new(user_globals: U::UserGlobals) -> Self;

    // Identity for mplus, i.e. empty stream
    fn mzero(&self) -> Self::Stream;

    // Stream with single element
    fn munit(&self, state: State<U>) -> Self::Stream;

    fn is_empty(&self, stream: &Self::Stream) -> bool;

    // Returns a stream of solutions from solving `goal` in each solution of `stream`.
    // Conjunction.
    fn mbind(&self, stream: Self::Stream, goal: Goal<U, Self>) -> Self::Stream;

    fn mplus(&self, stream: Self::Stream, lazy: Self::LazyStream) -> Self::Stream;

    fn lazy(&self, goal: Goal<U, Self>, state: State<U>) -> Self::LazyStream;

    fn next(&self, stream: &mut Self::Stream) -> Option<Box<State<U>>>;

    /// Returns a reference to next element in the stream, if any.
    fn peek<'a>(&self, stream: &'a mut Self::Stream) -> Option<&'a Box<State<U>>>;

    /// Truncates the stream leaving at most one element, and returns a reference to
    /// the remaining element if any.
    fn trunc<'a>(&self, stream: &'a mut Self::Stream) -> Option<&'a Box<State<U>>>;

    fn delay(&self, stream: Self::Stream) -> Self::LazyStream;

    // Packs lazy stream into stream without evaluating it
    fn inc(&self, stream: Self::LazyStream) -> Self::Stream;

    // Evaluate lazy stream
    fn force(&self, lazy: Self::LazyStream) -> Self::Stream;

    fn user_globals(&self) -> &U::UserGlobals;
}
