use crate::solver::Solver;
use crate::stream::{Lazy, Stream, StreamEngine};

pub type DefaultEngine = StreamEngine;

pub trait Engine: Sized + 'static {
    fn new() -> Self;

    fn step<'a>(&'a self, solver: &'a Solver, lazy: Lazy) -> Stream;
}
