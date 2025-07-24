use crate::goal::{AnyGoal, DFSGoal, Goal, InferredGoal};
use crate::lterm::LTerm;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use derivative::Derivative;
use std::any::Any;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Fresh<G>
where
    G: AnyGoal,
{
    variables: Vec<LTerm>,
    body: G,
}

impl<G> Fresh<G>
where
    G: AnyGoal,
{
    pub fn new(variables: Vec<LTerm>, body: G) -> InferredGoal<G> {
        InferredGoal::new(G::dynamic(Rc::new(Fresh { variables, body })))
    }

    pub fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<G> Solve for Fresh<G>
where
    G: AnyGoal,
{
    fn solve(&self, _solver: &Solver, state: State) -> Stream {
        if let Some(bfs) = self.as_any().downcast_ref::<Fresh<Goal>>() {
            Stream::pause(Box::new(state), bfs.body.clone())
        } else if let Some(dfs) = self.as_any().downcast_ref::<Fresh<DFSGoal>>() {
            Stream::pause_dfs(Box::new(state), dfs.body.clone())
        } else {
            unreachable!()
        }
    }
}
