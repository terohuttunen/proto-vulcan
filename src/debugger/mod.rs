use crate::user::User;
use crate::engine::Engine;
use crate::state::State;
use crate::goal::Goal;
use crate::stream::{Stream, Lazy};
use crate::query::QueryResult;
use crate::lresult::LResult;
use crate::lterm::LTerm;
use std::marker::PhantomData;
use std::rc::Rc;

mod ui;

pub struct Debugger<R, U, E>
where
    R: QueryResult<U, E>,
    U: User,
    E: Engine<U>,
{
    engine: E,
    variables: Vec<LTerm<U, E>>,
    stream: Stream<U, E>,
    _phantom: PhantomData<R>,
}

impl<R, U, E> Debugger<R, U, E>
where
    R: QueryResult<U, E>,
    U: User,
    E: Engine<U>,
{
    pub fn new(
        mut engine: E,
        variables: Vec<LTerm<U, E>>,
        goal: Goal<U, E>,
        initial_state: State<U, E>,
    ) -> Debugger<R, U, E> {
        let stream = goal.solve(&mut engine, initial_state);
        Debugger {
            engine,
            variables,
            stream,
            _phantom: PhantomData,
        }
    }
}

impl<R, U, E> Iterator for Debugger<R, U, E>
where
    R: QueryResult<U, E>,
    U: User,
    E: Engine<U>,
{
    type Item = R;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let maybe_state = self.stream.step(&mut self.engine);
            if self.stream.is_empty() {
                return None;
            } else {
                if maybe_state.is_some() {
                    return maybe_state;
                }
            }
        }
    }
}