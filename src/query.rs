use crate::engine::{DefaultEngine, Engine};
use crate::goal::Goal;
use crate::lresult::LResult;
use crate::lterm::LTerm;
use crate::state::State;
use crate::user::{EmptyUser, User};
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::rc::Rc;

pub trait QueryResult<U = EmptyUser>
where
    U: User,
{
    fn from_vec(v: Vec<LResult<U>>) -> Self;
}

pub struct ResultIterator<R, U = EmptyUser, E = DefaultEngine<U>>
where
    R: QueryResult<U>,
    U: User,
    E: Engine<U>,
{
    engine: E,
    variables: Vec<LTerm<U>>,
    stream: E::Stream,
    _phantom: PhantomData<R>,
}

#[doc(hidden)]
impl<R, U, E> ResultIterator<R, U, E>
where
    R: QueryResult<U>,
    U: User,
    E: Engine<U>,
{
    pub fn new(
        engine: E,
        variables: Vec<LTerm<U>>,
        goal: Goal<U, E>,
        initial_state: State<U>,
    ) -> ResultIterator<R, U, E> {
        let stream = goal.solve(&engine, initial_state);
        ResultIterator {
            engine,
            variables,
            stream,
            _phantom: PhantomData,
        }
    }
}

#[doc(hidden)]
impl<R, U, E> Iterator for ResultIterator<R, U, E>
where
    R: QueryResult<U>,
    U: User,
    E: Engine<U>,
{
    type Item = R;

    fn next(&mut self) -> Option<Self::Item> {
        match self.engine.next(&mut self.stream) {
            Some(state) => {
                // At this point the state has already gone through initial reification
                // process
                let smap = state.smap_ref();
                let purified_cstore = state.cstore_ref().clone().purify(smap).normalize();
                let reified_cstore = Rc::new(purified_cstore.walk_star(smap));
                let results = self
                    .variables
                    .iter()
                    .map(|v| LResult(state.smap_ref().walk_star(v), Rc::clone(&reified_cstore)))
                    .collect();
                Some(R::from_vec(results))
            }
            None => None,
        }
    }
}

/* ResultIterator is fused because uncons() will always keep returning None on empty stream */
#[doc(hidden)]
impl<R, U, E> FusedIterator for ResultIterator<R, U, E>
where
    R: QueryResult<U>,
    U: User,
    E: Engine<U>,
{
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Query<R, U = EmptyUser, E = DefaultEngine<U>>
where
    R: QueryResult<U>,
    U: User,
    E: Engine<U>,
{
    variables: Vec<LTerm<U>>,
    goal: Goal<U, E>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R, E> Query<R, EmptyUser, E>
where
    R: QueryResult<EmptyUser>,
    E: Engine<EmptyUser>,
{
    pub fn run(&self) -> ResultIterator<R, EmptyUser, E> {
        let user_state = EmptyUser::new();
        let user_globals = ();
        self.run_with_user(user_state, user_globals)
    }
}

impl<R, U, E> Query<R, U, E>
where
    R: QueryResult<U>,
    U: User,
    E: Engine<U>,
{
    pub fn new(variables: Vec<LTerm<U>>, goal: Goal<U, E>) -> Query<R, U, E> {
        Query {
            variables,
            goal,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn run_with_user(
        &self,
        user_state: U,
        user_globals: U::UserGlobals,
    ) -> ResultIterator<R, U, E> {
        let initial_state = State::new(user_state);
        let user_globals = user_globals;
        let engine = E::new(user_globals);
        ResultIterator::new(
            engine,
            self.variables.clone(),
            self.goal.clone(),
            initial_state,
        )
    }
}
