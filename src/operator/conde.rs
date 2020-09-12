use crate::goal::Goal;
use crate::operator::any::Any;
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::UserState;
use std::rc::Rc;

#[derive(Debug)]
pub struct Conde<U: UserState> {
    body: Rc<dyn Goal<U>>,
}

impl<U: UserState> Conde<U> {
    pub fn new(body: Rc<dyn Goal<U>>) -> Rc<dyn Goal<U>> {
        Rc::new(Conde { body })
    }

    pub fn from_vec(v: Vec<Rc<dyn Goal<U>>>) -> Rc<dyn Goal<U>> {
        Rc::new(Conde {
            body: Any::from_vec(v),
        })
    }

    pub fn from_array(goals: &[Rc<dyn Goal<U>>]) -> Rc<dyn Goal<U>> {
        Rc::new(Conde {
            body: Any::from_array(goals),
        })
    }

    // The parameter is a list of conjunctions, and the resulting goal is a disjunction
    // of conjunctions.
    pub fn from_conjunctions(conjunctions: &[&[Rc<dyn Goal<U>>]]) -> Rc<dyn Goal<U>> {
        Conde::new(Any::from_conjunctions(conjunctions))
    }
}

impl<U: UserState> Goal<U> for Conde<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        let goal = Rc::clone(&self.body);
        Stream::Lazy(LazyStream::from_goal(goal, state))
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
/// # #![recursion_limit = "512"]
/// use proto_vulcan::*;
/// use proto_vulcan::relation::membero;
/// let query = proto_vulcan_query!(|q| {
///     conde {
///         membero(q, [1, 2, 3]),
///         membero(q, [4, 5, 6]),
///         membero(q, [7, 8, 9]),
///     }
/// });
/// let mut iter = query.run();
/// assert_eq!(iter.next().unwrap().q, 1);
/// assert_eq!(iter.next().unwrap().q, 2);
/// assert_eq!(iter.next().unwrap().q, 4);
/// assert_eq!(iter.next().unwrap().q, 7);
/// assert_eq!(iter.next().unwrap().q, 3);
/// assert_eq!(iter.next().unwrap().q, 5);
/// assert_eq!(iter.next().unwrap().q, 8);
/// assert_eq!(iter.next().unwrap().q, 6);
/// assert_eq!(iter.next().unwrap().q, 9);
/// assert!(iter.next().is_none());
/// ```
pub fn conde<U: UserState>(body: &[&[Rc<dyn Goal<U>>]]) -> Rc<dyn Goal<U>> {
    Conde::from_conjunctions(&body)
}

#[cfg(test)]
mod test {
    use super::conde;
    use crate::relation::membero::membero;
    use crate::*;

    #[test]
    fn test_conde_1() {
        // Check that the order of solutions matches with miniKanren. This depends on
        // the interleaving and insertion of delays to various operators.
        let query = proto_vulcan_query!(|q| {
            conde {
                membero(q, [1, 2, 3]),
                membero(q, [4, 5, 6]),
                membero(q, [7, 8, 9]),
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 1);
        assert_eq!(iter.next().unwrap().q, 2);
        assert_eq!(iter.next().unwrap().q, 4);
        assert_eq!(iter.next().unwrap().q, 7);
        assert_eq!(iter.next().unwrap().q, 3);
        assert_eq!(iter.next().unwrap().q, 5);
        assert_eq!(iter.next().unwrap().q, 8);
        assert_eq!(iter.next().unwrap().q, 6);
        assert_eq!(iter.next().unwrap().q, 9);
        assert!(iter.next().is_none());
    }
}
