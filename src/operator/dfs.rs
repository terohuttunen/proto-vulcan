use crate::engine::Engine;
use crate::goal::{AnyGoal, DFSGoal, InferredGoal};
use crate::operator::conj::DFSConj;
use crate::operator::OperatorParam;
use crate::user::User;

pub fn dfs<U, E, G>(param: OperatorParam<U, E, DFSGoal<U, E>>) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    let g = DFSConj::from_conjunctions(param.body);
    match g {
        DFSGoal::Succeed => InferredGoal::new(G::succeed()),
        DFSGoal::Fail => InferredGoal::new(G::fail()),
        DFSGoal::Breakpoint(id) => InferredGoal::new(G::breakpoint(id)),
        DFSGoal::Dynamic(dynamic) => InferredGoal::new(G::dynamic(dynamic)),
    }
}
