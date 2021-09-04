use crate::engine::Engine;
use crate::goal::Goal;
use crate::operator::conj::Conj;
use crate::operator::OperatorParam;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct Cond<U, E>
where
    U: User,
    E: Engine<U>,
{
    conjunctions: Vec<Goal<U, E>>,
}

impl<U, E> Cond<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn from_vec(conjunctions: Vec<Goal<U, E>>) -> Goal<U, E> {
        Goal::dynamic(Cond { conjunctions })
    }

    pub fn from_array(goals: &[Goal<U, E>]) -> Goal<U, E> {
        Goal::dynamic(Cond {
            conjunctions: goals.to_vec(),
        })
    }

    // The parameter is a list of conjunctions, and the resulting goal is a disjunction
    // of conjunctions.
    pub fn from_conjunctions(goals: &[&[Goal<U, E>]]) -> Goal<U, E> {
        let mut conjunctions = vec![];
        for conjunction_goals in goals {
            conjunctions.push(Conj::from_array(conjunction_goals));
        }
        Cond::from_vec(conjunctions)
    }
}

impl<U, E> Solve<U, E> for Cond<U, E>
where
    U: User,
    E: Engine<U>,
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
                stream = Stream::mplus_dfs(new_stream, LazyStream::delay(stream));
            }
        }

        if self.conjunctions.len() > 0 {
            let new_stream = self.conjunctions[0].solve(solver, state);
            stream = Stream::mplus_dfs(new_stream, LazyStream::delay(stream));
        }

        stream
    }
}

/// Disjunction operator.
///
/// The cond operator is a disjunction of conjunctions where the body expression is of the
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

pub fn cond<U, E>(param: OperatorParam<U, E, Goal<U, E>>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Cond::from_conjunctions(param.body)
}

#[cfg(test)]
mod test {
    use super::cond;
    use crate::prelude::*;
    use crate::relation::membero::membero;

    #[test]
    fn test_conde_1() {
        let query = proto_vulcan_query!(|q| {
            cond {
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
