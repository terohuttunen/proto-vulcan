use crate::goal::Goal;
use crate::operator::all::All;
use crate::operator::conde::conde;
use crate::state::State;
use crate::stream::Stream;
use crate::user::UserState;
use std::rc::Rc;

#[derive(Debug)]
pub struct Anyo<U: UserState> {
    g: Rc<dyn Goal<U>>,
}

impl<U: UserState> Anyo<U> {
    pub fn new(g: Rc<dyn Goal<U>>) -> Rc<dyn Goal<U>> {
        Rc::new(Anyo { g })
    }
}

impl<U: UserState> Goal<U> for Anyo<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        let g = Rc::clone(&self.g);
        let g2 = Rc::clone(&self.g);
        let goal = proto_vulcan!(
            conde {
                g,
                anyo {
                    g2
                },
            }
        );
        goal.apply(state)
    }
}

/// Try a goal unbounded number of times operator.
///
/// Proto-vulcan provides a built-in keyword `loop` that maps to anyo.
/// # Example
/// In this example the conde-operator would be tried unbounded number of times.
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::*;
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
pub fn anyo<U: UserState>(goals: &[&[Rc<dyn Goal<U>>]]) -> Rc<dyn Goal<U>> {
    Anyo::new(All::from_conjunctions(goals))
}

#[cfg(test)]
mod tests {
    use super::anyo;
    use crate::operator::conde::conde;
    use crate::*;

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
