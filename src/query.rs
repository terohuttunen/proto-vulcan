//use crate::debugger::Debugger;
use crate::engine::{DefaultEngine, Engine};
use crate::goal::Goal;
use crate::lresult::LResult;
use crate::lterm::LTerm;
use crate::solver::Solver;
use crate::state::State;
use crate::stream::Stream;
use crate::user::{DefaultUser, User};
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::rc::Rc;

pub trait QueryResult<U = DefaultUser, E = DefaultEngine<U>>
where
    U: User,
    E: Engine<U>,
{
    fn from_vec(v: Vec<LResult<U, E>>) -> Self;
}

pub struct ResultIterator<R, U = DefaultUser, E = DefaultEngine<U>>
where
    R: QueryResult<U, E>,
    U: User,
    E: Engine<U>,
{
    solver: Solver<U, E>,
    variables: Vec<LTerm<U, E>>,
    stream: Stream<U, E>,
    _phantom: PhantomData<R>,
}

#[doc(hidden)]
impl<R, U, E> ResultIterator<R, U, E>
where
    R: QueryResult<U, E>,
    U: User,
    E: Engine<U>,
{
    pub fn new(
        solver: Solver<U, E>,
        variables: Vec<LTerm<U, E>>,
        goal: Goal<U, E>,
        initial_state: State<U, E>,
    ) -> ResultIterator<R, U, E> {
        let stream = goal.solve(&solver, initial_state);
        ResultIterator {
            solver,
            variables,
            stream,
            _phantom: PhantomData,
        }
    }
}

#[doc(hidden)]
impl<R, U, E> Iterator for ResultIterator<R, U, E>
where
    R: QueryResult<U, E>,
    U: User,
    E: Engine<U>,
{
    type Item = R;

    fn next(&mut self) -> Option<Self::Item> {
        match self.stream.next(&mut self.solver) {
            Some(state) => {
                // At this point the state has already gone through initial reification
                // process
                let smap = state.smap_ref();
                let purified_cstore = state.cstore_ref().clone().purify(smap).normalize();
                let reified_cstore = Rc::new(purified_cstore.walk_star(smap));
                let results = self
                    .variables
                    .iter()
                    .map(|v| {
                        LResult::<U, E>(state.smap_ref().walk_star(v), Rc::clone(&reified_cstore))
                    })
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
    R: QueryResult<U, E>,
    U: User,
    E: Engine<U>,
{
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Query<R, U = DefaultUser, E = DefaultEngine<U>>
where
    R: QueryResult<U, E>,
    U: User,
    E: Engine<U>,
{
    variables: Vec<LTerm<U, E>>,
    goal: Goal<U, E>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R, E> Query<R, DefaultUser, E>
where
    R: QueryResult<DefaultUser, E>,
    E: Engine<DefaultUser>,
{
    pub fn run(&self) -> ResultIterator<R, DefaultUser, E> {
        let user_state = DefaultUser::new();
        let user_globals = ();
        self.run_with_user(user_state, user_globals)
    }
}

impl<R, U, E> Query<R, U, E>
where
    R: QueryResult<U, E>,
    U: User,
    E: Engine<U>,
{
    pub fn new(variables: Vec<LTerm<U, E>>, goal: Goal<U, E>) -> Query<R, U, E> {
        Query {
            variables,
            goal,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn run_with_user(
        &self,
        user_state: U,
        user_globals: U::UserContext,
    ) -> ResultIterator<R, U, E> {
        let initial_state = State::new(user_state);
        let user_globals = user_globals;
        let solver = Solver::new(user_globals, false);
        ResultIterator::new(
            solver,
            self.variables.clone(),
            self.goal.clone(),
            initial_state,
        )
    }

    /*
    pub fn run_with_debugger(&self, user_state: U, user_globals: U::UserContext) -> Debugger<R, U, E> {
        let initial_state = State::new(user_state);
        let engine = E::new(user_globals);
        Debugger::new(engine, self.variables.clone(), self.goal.clone(), initial_state)
    }
    */
}
