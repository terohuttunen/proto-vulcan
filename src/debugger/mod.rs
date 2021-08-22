use crate::engine::Engine;
use crate::goal::Goal;
use crate::lresult::LResult;
use crate::lterm::LTerm;
use crate::query::QueryResult;
use crate::state::State;
use crate::stream::{Lazy, Stream};
use crate::user::User;
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
            println!("Step");
            match maybe_state {
                Some(state) => {
                    let smap = state.smap_ref();
                    let purified_cstore = state.cstore_ref().clone().purify(smap).normalize();
                    let reified_cstore = Rc::new(purified_cstore.walk_star(smap));
                    let results = self
                        .variables
                        .iter()
                        .map(|v| {
                            LResult::<U, E>(
                                state.smap_ref().walk_star(v),
                                Rc::clone(&reified_cstore),
                            )
                        })
                        .collect();

                    return Some(R::from_vec(results));
                }
                None => {
                    if self.stream.is_empty() {
                        return None;
                    }
                }
            }
        }
    }
}
