use crate::goal::Goal;
use crate::state::State;
use crate::stream::{Lazy, Stream, StreamEngine};
use crate::user::User;

pub type DefaultEngine<U> = StreamEngine<U>;

pub trait Engine<U>: Sized + 'static
where
    U: User,
{
    fn new(context: U::UserContext) -> Self;

    fn start(&mut self, state: Box<State<U, Self>>, goal: Goal<U, Self>) -> Stream<U, Self>;

    fn step(&mut self, lazy: Lazy<U, Self>) -> Stream<U, Self>;

    fn context(&self) -> &U::UserContext;
}
