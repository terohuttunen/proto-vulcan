use crate::engine::Engine;
use crate::goal::{AnyGoal, Goal, InferredGoal};
use crate::operator::conj::InferredConj;
use crate::operator::OperatorParam;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;
use crate::GoalCast;
use std::marker::PhantomData;

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
        InferredGoal::dynamic(Conde {
            conjunctions,
            _phantom: PhantomData,
            _phantom2: PhantomData,
        })
    }

    pub fn from_array(goals: &[G]) -> InferredGoal<U, E, G> {
        InferredGoal::dynamic(Conde {
            conjunctions: goals.to_vec(),
            _phantom: PhantomData,
            _phantom2: PhantomData,
        })
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
        let mut stream = Stream::empty();

        // Process first element separately to avoid one extra clone of `state`.
        if self.conjunctions.len() > 1 {
            for conjunction in self
                .conjunctions
                .iter()
                .rev()
                .take(self.conjunctions.len() - 1)
            {
                let new_stream = conjunction.solve(solver, state.clone());
                stream = Stream::mplus(new_stream, LazyStream::delay(stream));
            }
        }

        if self.conjunctions.len() > 0 {
            let new_stream = self.conjunctions[0].solve(solver, state);
            stream = Stream::mplus(new_stream, LazyStream::delay(stream));
        }

        stream
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
/// `membero`-relations, are interleaved by the `conde`-operator:
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// use proto_vulcan::relation::membero;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         conde {
///             membero(q, [1, 2, 3]),
///             membero(q, [4, 5, 6]),
///             membero(q, [7, 8, 9]),
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
    use super::conde;
    use crate::prelude::*;
    use crate::relation::membero::membero;

    #[test]
    fn test_conde_1() {
        let query = proto_vulcan_query!(|q| {
            conde {
                membero(q, [1, 2, 3]),
                membero(q, [4, 5, 6]),
                membero(q, [7, 8, 9]),
            }
        });
        let iter = query.run();
        let mut expected = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        iter.for_each(|x| {
            let n = x.q.get_number().unwrap();
            assert!(expected.contains(&n));
            expected.retain(|y| n != *y);
        });
        assert_eq!(expected.len(), 0);
    }
}
