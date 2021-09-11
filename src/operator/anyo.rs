use crate::engine::Engine;
use crate::goal::{AnyGoal, Goal};
use crate::operator::conde::conde;
use crate::operator::conj::Conj;
use crate::operator::OperatorParam;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use crate::GoalCast;

#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
pub struct Anyo<U, E>
where
    U: User,
    E: Engine<U>,
{
    g: Goal<U, E>,
}

impl<U, E> Anyo<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(g: Goal<U, E>) -> Goal<U, E> {
        Goal::dynamic(Anyo { g })
    }
}

impl<U, E> Solve<U, E> for Anyo<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        let g = self.g.clone();
        let g2 = self.g.clone();
        let goal = proto_vulcan!(
            conde {
                g,
                anyo {
                    g2
                },
            }
        );
        goal.solve(solver, state)
    }
}

/// Try a goal unbounded number of times operator.
///
/// Proto-vulcan provides a built-in keyword `loop` that maps to anyo.
/// # Example
/// In this example the conde-operator would be tried unbounded number of times.
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::prelude::*;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         loop {
///             conde {
///                 1 == q,
///                 2 == q,
///                 3 == q,
///             }
///         }
///     });
///     let mut iter = query.run();
///     assert_eq!(iter.next().unwrap().q, 1);
///     assert_eq!(iter.next().unwrap().q, 2);
///     assert_eq!(iter.next().unwrap().q, 3);
///     assert_eq!(iter.next().unwrap().q, 1);
///     assert_eq!(iter.next().unwrap().q, 2);
///     assert_eq!(iter.next().unwrap().q, 3);
/// }
/// ```
pub fn anyo<U, E>(param: OperatorParam<U, E, Goal<U, E>>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Anyo::new(GoalCast::cast_into(Conj::from_conjunctions(param.body)))
}

#[cfg(test)]
mod tests {
    use super::anyo;
    use crate::operator::conde::conde;
    use crate::prelude::*;

    #[test]
    fn test_anyo_1() {
        let query = proto_vulcan_query!(|q| {
            conde {
                anyo {
                    false == q,
                },
                true == q,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, true);
        assert_eq!(iter.next().unwrap().q, false);
        assert_eq!(iter.next().unwrap().q, false);
        assert_eq!(iter.next().unwrap().q, false);
        assert_eq!(iter.next().unwrap().q, false);
    }

    #[test]
    fn test_anyo_2() {
        let query = proto_vulcan_query!(|q| {
            anyo {
                conde {
                    1 == q,
                    2 == q,
                    3 == q,
                }
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
    }

    #[test]
    fn test_anyo_3() {
        // Test "loop" operator keyword
        let query = proto_vulcan_query!(|q| {
            loop {
                conde {
                    1 == q,
                    2 == q,
                    3 == q,
                }
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 3);
    }
}
