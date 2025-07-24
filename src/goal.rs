//! Solvable goals
//!
//! Proto-vulcan goals can be divided into two main categories: 1) breadth-first
//! search (BFS) and 2) depth-first search (DFS) goals. The solutions to BFS goals
//! are searched in the usual miniKanren-style interleaved fashion, whereas solutions
//! to DFS goals are searched in the Prolog-style depth-first fashion. Breadth-first
//! search is fairer, and can provide solutions from multiple infinite streams in
//! parallel, but requires more resources to maintain the parallel search streams.
//! Depth-first search on the other hand requires less resources, but cannot handle
//! infinite streams of solutions. If all solutions are needed, and the order does
//! not matter, then DFS is recommended.
//!
//! * BFS goals are represented by the `Goal` type.
//! * DFS goals are represented by the `DFSGoal` type.
//!
//! Proto-vulcan allows any branch in the tree of goals to be DFS; by default, queries
//! are BFS. Preventing embedding of BFS goals in DFS branches is enforced with Rust
//! typing. A branch can be made DFS with an operator that is strictly DFS, or with
//! the dfs-operator and inferred goals. The embedding of goals is controlled with
//! the `GoalCast`-trait; wherever a goal is a parameter of another goal, the macros
//! generate a `GoalCast::cast_into(goal)`-call to convert the parameter goal into
//! the kind of the parent goal constructor parameter.
//!
//! Often an operator or relation can be either DFS or BFS; such goals can be wrapped
//! into `InferredGoal<G>` which is always cast into the search type of the
//! parent goal.
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use std::rc::Rc;

pub use crate::GoalCast;

pub trait AnyGoal: std::fmt::Debug + std::clone::Clone + 'static {
    fn succeed() -> Self
    where
        Self: Sized;

    fn fail() -> Self
    where
        Self: Sized;

    fn breakpoint(id: &'static str) -> Self
    where
        Self: Sized;

    fn dynamic(u: Rc<dyn Solve>) -> Self
    where
        Self: Sized;

    fn is_succeed(&self) -> bool;

    fn is_fail(&self) -> bool;

    fn is_breakpoint(&self) -> bool;

    fn solve(&self, solver: &Solver, state: State) -> Stream;
}

/// Breadth-first searched goal
#[derive(Debug, Clone)]
pub enum Goal {
    Succeed,
    Fail,
    Breakpoint(&'static str),
    Dynamic(Rc<dyn Solve>),
}

impl AnyGoal for Goal {
    fn succeed() -> Goal {
        Goal::Succeed
    }

    fn fail() -> Goal {
        Goal::Fail
    }

    fn breakpoint(id: &'static str) -> Goal {
        Goal::Breakpoint(id)
    }

    fn dynamic(u: Rc<dyn Solve>) -> Goal {
        Goal::Dynamic(u)
    }

    fn is_succeed(&self) -> bool {
        match self {
            Goal::Succeed => true,
            _ => false,
        }
    }

    fn is_fail(&self) -> bool {
        match self {
            Goal::Fail => true,
            _ => false,
        }
    }

    fn is_breakpoint(&self) -> bool {
        match self {
            Goal::Breakpoint(_) => true,
            _ => false,
        }
    }

    fn solve(&self, solver: &Solver, state: State) -> Stream {
        match self {
            Goal::Succeed => Stream::unit(Box::new(state)),
            Goal::Fail => Stream::empty(),
            Goal::Breakpoint(_) => Stream::unit(Box::new(state)),
            Goal::Dynamic(dynamic) => dynamic.solve(solver, state),
        }
    }
}

/// Depth-first searched goal
#[derive(Debug, Clone)]
pub enum DFSGoal {
    Succeed,
    Fail,
    Breakpoint(&'static str),
    Dynamic(Rc<dyn Solve>),
}

impl AnyGoal for DFSGoal {
    fn succeed() -> DFSGoal {
        DFSGoal::Succeed
    }

    fn fail() -> DFSGoal {
        DFSGoal::Fail
    }

    fn breakpoint(id: &'static str) -> DFSGoal {
        DFSGoal::Breakpoint(id)
    }

    fn dynamic(u: Rc<dyn Solve>) -> DFSGoal {
        DFSGoal::Dynamic(u)
    }

    fn is_succeed(&self) -> bool {
        match self {
            DFSGoal::Succeed => true,
            _ => false,
        }
    }

    fn is_fail(&self) -> bool {
        match self {
            DFSGoal::Fail => true,
            _ => false,
        }
    }

    fn is_breakpoint(&self) -> bool {
        match self {
            DFSGoal::Breakpoint(_) => true,
            _ => false,
        }
    }

    fn solve(&self, solver: &Solver, state: State) -> Stream {
        match self {
            DFSGoal::Succeed => Stream::unit(Box::new(state)),
            DFSGoal::Fail => Stream::empty(),
            DFSGoal::Breakpoint(_) => Stream::unit(Box::new(state)),
            DFSGoal::Dynamic(dynamic) => dynamic.solve(solver, state),
        }
    }
}

impl Into<Goal> for DFSGoal {
    fn into(self) -> Goal {
        match self {
            DFSGoal::Succeed => Goal::Succeed,
            DFSGoal::Fail => Goal::Fail,
            DFSGoal::Breakpoint(id) => Goal::Breakpoint(id),
            DFSGoal::Dynamic(dynamic) => Goal::Dynamic(dynamic),
        }
    }
}

/// A wrapper for goals that can be either DFS or BFS.
///
/// The correct kind is inferred at compile time from the type of the parent goal
/// constructor parameter where the goal is placed.
#[derive(Debug, Clone)]
pub struct InferredGoal<G>
where
    G: AnyGoal,
{
    pub goal: G,
}

impl<G> InferredGoal<G>
where
    G: AnyGoal,
{
    pub fn new(goal: G) -> InferredGoal<G> {
        InferredGoal { goal }
    }
}

// DFSGoal -> Goal
impl GoalCast<Goal> for DFSGoal {
    #[inline]
    fn cast_into(self) -> Goal {
        self.into()
    }
}

// InferredGoal<G> -> G
impl<G> GoalCast<G> for InferredGoal<G>
where
    G: AnyGoal,
{
    #[inline]
    fn cast_into(self) -> G {
        self.goal
    }
}

// InferredGoal<G> -> InferredGoal<G>
impl<G> GoalCast<InferredGoal<G>> for InferredGoal<G>
where
    G: AnyGoal,
{
    #[inline]
    fn cast_into(self) -> InferredGoal<G> {
        self
    }
}

// Goal -> Goal
impl GoalCast<Goal> for Goal {
    #[inline]
    fn cast_into(self) -> Self {
        self
    }
}

// DFSGoal -> DFSGoal
impl GoalCast<DFSGoal> for DFSGoal {
    #[inline]
    fn cast_into(self) -> Self {
        self
    }
}

#[cfg(test)]
mod test {
    use super::AnyGoal;
    use crate::engine::{DefaultEngine, Engine};
    use crate::prelude::*;
    use crate::solver::Solve;
    use crate::state::State;
    use crate::stream::Stream;

    use std::rc::Rc;

    #[test]
    fn test_goal_succeed() {
        let g = Goal::succeed();
        assert!(g.is_succeed());
        assert!(!g.is_fail());
    }

    #[test]
    fn test_goal_fail() {
        let g = Goal::fail();
        assert!(g.is_fail());
        assert!(!g.is_succeed());
    }

    #[derive(Debug)]
    struct TestGoal {}

    impl Solve for TestGoal {
        fn solve(&self, _engine: &Solver, _state: State) -> Stream {
            Stream::empty()
        }
    }

    #[test]
    fn test_goal_inner() {
        let g = Goal::dynamic(Rc::new(TestGoal {}));
        assert!(!g.is_succeed());
        assert!(!g.is_fail());
    }
}
