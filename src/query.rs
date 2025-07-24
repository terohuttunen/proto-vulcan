
use crate::goal::Goal;
use crate::lresult::LResult;
use crate::lterm::LTerm;
use crate::solver::Solver;
use crate::state::State;
use crate::stream::Stream;
use crate::user::DefaultUser;

use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::rc::Rc;

pub trait QueryResult
{
    fn from_vec(v: Vec<LResult>) -> Self;
}

pub struct ResultIterator<R>
where
    R: QueryResult,
{
    solver: Solver,
    variables: Vec<LTerm>,
    stream: Stream,
    _phantom: PhantomData<R>,
}

#[doc(hidden)]
impl<R> ResultIterator<R>
where
    R: QueryResult,
{
    pub fn new(
        solver: Solver,
        variables: Vec<LTerm>,
        goal: Goal,
        initial_state: State,
    ) -> ResultIterator<R> {
        let stream = solver.start(&goal, initial_state);
        ResultIterator {
            solver,
            variables,
            stream,
            _phantom: PhantomData,
        }
    }
}

#[doc(hidden)]
impl<R> Iterator for ResultIterator<R>
where
    R: QueryResult,
{
    type Item = R;

    fn next(&mut self) -> Option<Self::Item> {
        match self.solver.next(&mut self.stream) {
            crate::solver::SolverResult::Solution(state) => {
                // At this point the state has already gone through initial reification
                // process
                let smap = state.smap_ref();
                let purified_cstore = state.cstore_ref().clone().purify(smap).normalize();
                let reified_cstore = Rc::new(purified_cstore.walk_star(smap));
                let results = self
                    .variables
                    .iter()
                    .map(|v| {
                        LResult(state.smap_ref().walk_star(v), Rc::clone(&reified_cstore))
                    })
                    .collect();

                Some(R::from_vec(results))
            }
            crate::solver::SolverResult::NoMoreSolutions => None,
            crate::solver::SolverResult::Timeout => None, // Iterator should terminate on timeout
            crate::solver::SolverResult::Error(_) => None, // Iterator should terminate on error
        }
    }
}

/* ResultIterator is fused because uncons() will always keep returning None on empty stream */
#[doc(hidden)]
impl<R> FusedIterator for ResultIterator<R>
where
    R: QueryResult,
{
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Query<R>
where
    R: QueryResult,
{
    variables: Vec<LTerm>,
    goal: Goal,
    _phantom: std::marker::PhantomData<R>,
}

impl<R> Query<R>
where
    R: QueryResult,
{
    pub fn run(&self) -> ResultIterator<R> {
        let user_state = DefaultUser::new();
        let user_globals = ();
        self.run_with_user(user_state, user_globals)
    }
}

impl<R> Query<R>
where
    R: QueryResult,
{
    pub fn new(variables: Vec<LTerm>, goal: Goal) -> Query<R> {
        Query {
            variables,
            goal,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn run_with_user(
        &self,
        user_state: DefaultUser,
        user_globals: <DefaultUser as crate::user::User>::UserContext,
    ) -> ResultIterator<R> {
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
}
