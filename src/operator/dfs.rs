use crate::goal::{AnyGoal, DFSGoal, InferredGoal};
use crate::operator::conj::DFSConj;
use crate::operator::OperatorParam;

pub fn dfs<G: AnyGoal>(param: OperatorParam<DFSGoal>) -> InferredGoal<G> {
    let g = DFSConj::from_conjunctions(param.body);
    match g {
        DFSGoal::Succeed => InferredGoal::new(G::succeed()),
        DFSGoal::Fail => InferredGoal::new(G::fail()),
        DFSGoal::Breakpoint(id) => InferredGoal::new(G::breakpoint(id)),
        DFSGoal::Dynamic(dynamic) => InferredGoal::new(G::dynamic(dynamic)),
        DFSGoal::LazyMacro(closure) => InferredGoal::new(G::lazy_macro(closure)),
    }
}
