use crate::goal::Goal;
use crate::solver::Solver;
use crate::state::State;
use crate::stream::{Lazy, Stream, StreamEngine};
use crate::user::User;

pub type DefaultEngine<U> = StreamEngine<U>;

pub trait Engine<U>: Sized + 'static
where
    U: User,
{
    fn new() -> Self;

    fn start<'a>(
        &'a self,
        solver: &'a Solver<U, Self>,
        state: Box<State<U, Self>>,
        goal: &Goal<U, Self>,
    ) -> Stream<U, Self>;

    fn step<'a>(&'a self, solver: &'a Solver<U, Self>, lazy: Lazy<U, Self>) -> Stream<U, Self>;
}
