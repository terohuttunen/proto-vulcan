use crate::goal::Goal;
use crate::state::State;
use crate::stream::{Lazy, Stream, StreamEngine};
use crate::user::User;
use std::fmt::Debug;

pub mod debugger;

pub type DefaultEngine<U> = StreamEngine<U>;

pub trait Engine<U>: Sized
where
    U: User,
{
    fn new(context: U::UserContext) -> Self;

    fn start(&self, state: Box<State<U, Self>>, goal: Goal<U, Self>) -> Stream<U, Self>;

    fn step(&self, lazy: Lazy<U, Self>) -> Stream<U, Self>;

    fn context(&self) -> &U::UserContext;
}
