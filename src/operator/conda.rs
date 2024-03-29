use crate::engine::Engine;
/// Conditional ?
///
/// Returns stream from first clause [x0 AND x1 AND ...] whose first (a0, b0, ...) goal succeeds
///
/// [a0 AND a1 AND ...] OR
/// [b0 AND b1 AND ...] OR ...
use crate::goal::{AnyGoal, Goal};
use crate::operator::conj::Conj;
use crate::operator::OperatorParam;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use crate::GoalCast;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct Conda<U, E>
where
    U: User,
    E: Engine<U>,
{
    // First goal of this conda clause
    first: Goal<U, E>,

    // Rest of the goals of this conda clause
    rest: Goal<U, E>,

    // Next conda clause
    next: Goal<U, E>,
}

impl<U, E> Conda<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn from_conjunctions(body: &[&[Goal<U, E>]]) -> Goal<U, E> {
        let mut next = Goal::fail();
        for clause in body.to_vec().drain(..).rev() {
            let mut clause = clause.to_vec();
            if !clause.is_empty() {
                let rest = GoalCast::cast_into(Conj::from_vec(clause.split_off(1)));
                let first = clause.pop().unwrap();
                next = Goal::dynamic(Rc::new(Conda { first, rest, next }));
            }
        }
        next
    }
}

impl<U, E> Solve<U, E> for Conda<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        let mut stream = solver.start(&self.first, state.clone());

        match solver.peek(&mut stream) {
            Some(_) => Stream::bind(stream, self.rest.clone()),
            None => self.next.solve(solver, state),
        }
    }
}

/// Soft cut operator.
pub fn conda<U, E>(param: OperatorParam<U, E, Goal<U, E>>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Conda::from_conjunctions(param.body)
}

#[cfg(test)]
mod tests {
    use crate::operator::conda::conda;
    use crate::prelude::*;
    use crate::relation::member::member;

    #[test]
    fn test_conda_1() {
        // First goal of first clause succeeds twice
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                q == [x, y],
                member(z, [5, 6]),
                conda {
                    [x == z, y == 2],
                    [x == z, y == 4],
                }
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([5, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([6, 2]));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_conda_2() {
        // First goal of first clause fails
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                q == [x, y],
                member(z, [5, 6]),
                conda {
                    [false, y == 2],
                    [x == z, y == 4],
                }
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([5, 4]));
        assert_eq!(iter.next().unwrap().q, lterm!([6, 4]));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_conda_3() {
        // From W. Byrd dissertation page 22
        let query = proto_vulcan_query!(|x| {
            conda {
                "olive" == x,
                "oil" == x,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().x, "olive");
    }

    #[test]
    fn test_conda_4() {
        // From W. Byrd dissertation page 22
        let query = proto_vulcan_query!(|x| {
            conda {
                ["virgin" == x, true == false],
                ["olive" == x],
                ["oil" == x]
            }
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }
}
