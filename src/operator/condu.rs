/// Conditional ?
///
/// Returns stream from first clause [x0 AND x1 AND ...] whose first (a0, b0, ...) goal succeeds.
/// As opposed to conda, condu takes only first item from the stream of the first goal.
///
/// [a0 AND a1 AND ...] OR
/// [b0 AND b1 AND ...] OR ...
use crate::goal::{Goal, Solver};
use crate::operator::all::All;
use crate::operator::OperatorParam;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;
use std::rc::Rc;

#[derive(Debug)]
pub struct Condu<U: User> {
    // First goal of this condu clause
    first: Goal<U>,

    // Rest of the goals of this condu clause
    rest: Goal<U>,

    // Next condu clause
    next: Goal<U>,
}

impl<U: User> Condu<U> {
    pub fn from_conjunctions(body: &[&[Goal<U>]]) -> Goal<U> {
        let mut next = proto_vulcan!(false);
        for clause in body.to_vec().drain(..).rev() {
            let mut clause = clause.to_vec();
            if !clause.is_empty() {
                let rest = All::from_vec(clause.split_off(1));
                let first = clause.pop().unwrap();
                next = Rc::new(Condu { first, rest, next });
            }
        }
        next
    }
}

impl<U: User> Solver<U> for Condu<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        let mut stream = self.first.solve(state.clone());

        // Take only first item from the stream of first goal by truncating the stream
        match stream.trunc() {
            Some(_) => Stream::bind(stream, Rc::clone(&self.rest)),
            None => self.next.solve(state),
        }
    }
}

/// Committed choice operator.
pub fn condu<U: User>(param: OperatorParam<U>) -> Goal<U> {
    Condu::from_conjunctions(param.body)
}

#[cfg(test)]
mod tests {
    use super::condu;
    use crate::relation::membero::membero;
    use crate::*;

    #[test]
    fn test_conda_1() {
        // First goal of first clause succeeds twice
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                q == [x, y],
                membero(z, [5, 6]),
                condu {
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
                membero(z, [5, 6]),
                condu {
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

    // TODO: Improve tests. Difference between condu and conda is not tested.
}
