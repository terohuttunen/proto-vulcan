use crate::engine::Engine;
use crate::goal::{AnyGoal, DFSGoal, Goal, InferredGoal};
use crate::operator::conj::InferredConj;
use crate::operator::OperatorParam;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;
use crate::GoalCast;
use std::any::Any;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct Conde<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    conjunctions: Vec<G>,
    _phantom: PhantomData<U>,
    _phantom2: PhantomData<E>,
}

impl<U, E, G> Conde<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    pub fn from_vec(conjunctions: Vec<G>) -> InferredGoal<U, E, G> {
        InferredGoal::new(G::dynamic(Rc::new(Conde {
            conjunctions,
            _phantom: PhantomData,
            _phantom2: PhantomData,
        })))
    }

    pub fn from_array(goals: &[G]) -> InferredGoal<U, E, G> {
        InferredGoal::new(G::dynamic(Rc::new(Conde {
            conjunctions: goals.to_vec(),
            _phantom: PhantomData,
            _phantom2: PhantomData,
        })))
    }

    pub fn as_any(&self) -> &dyn Any {
        self
    }

    // The parameter is a list of conjunctions, and the resulting goal is a disjunction
    // of conjunctions.
    pub fn from_conjunctions(goals: &[&[G]]) -> InferredGoal<U, E, G> {
        let mut conjunctions = vec![];
        for conjunction_goals in goals {
            conjunctions.push(GoalCast::cast_into(InferredConj::from_array(
                conjunction_goals,
            )));
        }
        Conde::from_vec(conjunctions)
    }
}

impl<U, E, G> Solve<U, E> for Conde<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        if let Some(bfs) = self.as_any().downcast_ref::<Conde<U, E, Goal<U, E>>>() {
            let mut stream = Stream::empty();

            // Process first element separately to avoid one extra clone of `state`.
            if bfs.conjunctions.len() > 1 {
                for conjunction in bfs
                    .conjunctions
                    .iter()
                    .rev()
                    .take(bfs.conjunctions.len() - 1)
                {
                    let new_stream = conjunction.solve(solver, state.clone());
                    stream = Stream::mplus(new_stream, LazyStream::delay(stream));
                }
            }

            if self.conjunctions.len() > 0 {
                let new_stream = bfs.conjunctions[0].solve(solver, state);
                stream = Stream::mplus(new_stream, LazyStream::delay(stream));
            }

            stream
        } else if let Some(dfs) = self.as_any().downcast_ref::<Conde<U, E, DFSGoal<U, E>>>() {
            let mut stream = Stream::empty();

            // Process first element separately to avoid one extra clone of `state`.
            if dfs.conjunctions.len() > 1 {
                for conjunction in dfs
                    .conjunctions
                    .iter()
                    .rev()
                    .take(dfs.conjunctions.len() - 1)
                {
                    let new_stream = conjunction.solve(solver, state.clone());
                    stream = Stream::mplus_dfs(new_stream, LazyStream::delay(stream));
                }
            }

            if self.conjunctions.len() > 0 {
                let new_stream = dfs.conjunctions[0].solve(solver, state);
                stream = Stream::mplus_dfs(new_stream, LazyStream::delay(stream));
            }

            stream
        } else {
            unreachable!()
        }
    }
}

/// Disjunction operator.
///
/// The conde operator is a disjunction of conjunctions where the body expression is of the
/// following form where commas are replaced with ANDs and ORs to show the logical relations:
/// ```text
/// conde {
///     [g11 AND g12 AND g13 AND ...] OR
///     [g21 AND g22 AND g23 AND ...] OR
///     [g31 AND g32 AND g33 AND ...] OR
///     ...
/// }
/// ```
///
/// If there is only one goal within the conjunction as: `[foo()]`, then the brackets are not
/// necessary, and we can write just `foo()`.
///
/// # Example
/// Conde is one of the core miniKanren operators, and it executes an interleaved search of the
/// streams of solutions from the conjunctions. This example shows how the solutions from the
/// `member`-relations, are interleaved by the `conde`-operator:
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// use proto_vulcan::relation::member;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         conde {
///             member(q, [1, 2, 3]),
///             member(q, [4, 5, 6]),
///             member(q, [7, 8, 9]),
///         }
///     });
///     let mut iter = query.run();
///     let mut expected = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
///     iter.for_each(|x| {
///         let n = x.q.get_number().unwrap();
///         assert!(expected.contains(&n));
///         expected.retain(|y| n != *y);
///     });
///     assert_eq!(expected.len(), 0);
/// }
/// ```
pub fn conde<U, E>(param: OperatorParam<U, E, Goal<U, E>>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Conde::from_conjunctions(param.body).cast_into()
}

/// Inferred version of conde
pub fn cond<U, E, G>(param: OperatorParam<U, E, G>) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    Conde::from_conjunctions(param.body)
}

#[cfg(test)]
mod test {
    use super::{cond, conde};
    use crate::operator::dfs;
    use crate::prelude::*;
    use crate::relation::member;

    #[test]
    fn test_conde_1() {
        let query = proto_vulcan_query!(|q| {
            conde {
                member(q, [1, 2, 3]),
                member(q, [4, 5, 6]),
                member(q, [7, 8, 9]),
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 4);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 7);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 5);
        assert_eq!(iter.next().unwrap().q, 8);
        assert_eq!(iter.next().unwrap().q, 6);
        assert_eq!(iter.next().unwrap().q, 9);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_cond_1() {
        // cond in BFS is same as conde
        let query = proto_vulcan_query!(|q| {
            cond {
                member(q, [1, 2, 3]),
                member(q, [4, 5, 6]),
                member(q, [7, 8, 9]),
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 4);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 7);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 5);
        assert_eq!(iter.next().unwrap().q, 8);
        assert_eq!(iter.next().unwrap().q, 6);
        assert_eq!(iter.next().unwrap().q, 9);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_cond_2() {
        // cond in DFS goes depth-first
        let query = proto_vulcan_query!(|q| {
            dfs {
                cond {
                    member(q, [1, 2, 3]),
                    member(q, [4, 5, 6]),
                    member(q, [7, 8, 9]),
                }
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 4);
        assert_eq!(iter.next().unwrap().q, 5);
        assert_eq!(iter.next().unwrap().q, 6);
        assert_eq!(iter.next().unwrap().q, 7);
        assert_eq!(iter.next().unwrap().q, 8);
        assert_eq!(iter.next().unwrap().q, 9);
        assert!(iter.next().is_none());
    }
}
