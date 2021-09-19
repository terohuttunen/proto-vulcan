use crate::engine::Engine;
use crate::goal::{AnyGoal, DFSGoal, Goal};
use crate::solver::Solver;
use crate::state::State;
use crate::stream::{LazyStream, Stream, StreamIterator};
use crate::user::User;
use std::marker::PhantomData;

pub fn map_sum<U, E, F, T>(
    solver: &Solver<U, E>,
    state: State<U, E>,
    mut f: F,
    iter: impl Iterator<Item = T>,
) -> Stream<U, E>
where
    U: User,
    E: Engine<U>,
    F: FnMut(T) -> Goal<U, E>,
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

#[derive(Derivative)]
#[derivative(Clone(bound = "U: User"))]
pub struct MapSumIterator<U, E, G, F, T, I>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
    F: Fn(T) -> G + Clone + 'static,
    T: 'static,
    I: Iterator<Item = T> + Clone,
{
    state: State<U, E>,
    f: F,
    iter: I,
    _phantom: PhantomData<U>,
    _phantom2: PhantomData<E>,
}

impl<U, E, G, F, T, I> MapSumIterator<U, E, G, F, T, I>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
    F: Fn(T) -> G + Clone + 'static,
    T: 'static,
    I: Iterator<Item = T> + Clone,
{
    pub fn new(state: State<U, E>, f: F, iter: I) -> MapSumIterator<U, E, G, F, T, I> {
        MapSumIterator {
            state,
            f,
            iter,
            _phantom: PhantomData,
            _phantom2: PhantomData,
        }
    }
}

impl<U, E, G, F, T, I> StreamIterator<U, E> for MapSumIterator<U, E, G, F, T, I>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
    F: Fn(T) -> G + Clone + 'static,
    T: 'static,
    I: Iterator<Item = T> + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn StreamIterator<U, E>> {
        Box::new(self.clone())
    }

    fn next(&mut self, solver: &Solver<U, E>) -> Option<Stream<U, E>> {
        match self.iter.next() {
            Some(t) => {
                let stream = (self.f)(t).solve(solver, self.state.clone());
                Some(stream)
            }
            None => None,
        }
    }
}

pub fn map_sum_iter<U, E, F, T, I>(state: State<U, E>, f: F, iter: I) -> Stream<U, E>
where
    U: User,
    E: Engine<U>,
    F: Fn(T) -> DFSGoal<U, E> + Clone + 'static,
    T: 'static,
    I: Iterator<Item = T> + Clone + 'static,
{
    Stream::iterator(Box::new(MapSumIterator::new(state, f, iter)))
}
