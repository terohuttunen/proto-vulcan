use crate::goal::{AnyGoal, DFSGoal, Goal};
use crate::solver::Solver;
use crate::state::State;
use crate::stream::{LazyStream, Stream, StreamIterator};

pub fn map_sum<F, T>(
    solver: &Solver,
    state: State,
    mut f: F,
    iter: impl Iterator<Item = T>,
) -> Stream
where
    F: FnMut(T) -> Goal,
{
    let mut iter = iter.peekable();
    let mut stream = Stream::empty();
    loop {
        match iter.next() {
            Some(d) => {
                if iter.peek().is_none() {
                    // If this is last value in the domain, no need to clone `state`.
                    let new_stream = f(d).solve(solver, state);
                    stream = Stream::mplus(new_stream, LazyStream::delay(stream));
                    break;
                } else {
                    let new_stream = f(d).solve(solver, state.clone());
                    stream = Stream::mplus(new_stream, LazyStream::delay(stream));
                }
            }
            None => {
                unreachable!();
            }
        }
    }
    stream
}

#[derive(Clone)]
pub struct MapSumIterator<G, F, T, I>
where
    G: AnyGoal,
    F: Fn(T) -> G + Clone + 'static,
    T: Clone + 'static,
    I: Iterator<Item = T> + Clone,
{
    state: State,
    f: F,
    iter: I,
}

impl<G, F, T, I> MapSumIterator<G, F, T, I>
where
    G: AnyGoal,
    F: Fn(T) -> G + Clone + 'static,
    T: Clone + 'static,
    I: Iterator<Item = T> + Clone,
{
    pub fn new(state: State, f: F, iter: I) -> MapSumIterator<G, F, T, I> {
        MapSumIterator { state, f, iter }
    }
}

impl<G, F, T, I> StreamIterator for MapSumIterator<G, F, T, I>
where
    G: AnyGoal,
    F: Fn(T) -> G + Clone + 'static,
    T: Clone + 'static,
    I: Iterator<Item = T> + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn StreamIterator> {
        Box::new((*self).clone())
    }

    fn next(&mut self, solver: &Solver) -> Option<Stream> {
        match self.iter.next() {
            Some(t) => {
                let stream = (self.f)(t).solve(solver, self.state.clone());
                Some(stream)
            }
            None => None,
        }
    }
}

pub fn map_sum_iter<F, T, I>(state: State, f: F, iter: I) -> Stream
where
    F: Fn(T) -> DFSGoal + Clone + 'static,
    T: Clone + 'static,
    I: Iterator<Item = T> + Clone + 'static,
{
    Stream::iterator(Box::new(MapSumIterator::new(state, f, iter)))
}
