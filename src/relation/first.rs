
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::relation::cons;


/// A relation such that the `first` is the first element of `list`.
pub fn first<G>(list: LTerm, first: LTerm) -> InferredGoal<G>
where
    G: AnyGoal,
{
    proto_vulcan!(|rest| { cons(first, rest, list) })
}

#[cfg(test)]
mod test {
    use super::first;
    use crate::prelude::*;

    #[test]
    fn test_first_1() {
        let query = proto_vulcan_query!(|q| { first([1], q) });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_first_2() {
        let query = proto_vulcan_query!(|q| { first([1, 2], q) });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_first_3() {
        let query = proto_vulcan_query!(|q| { first([1, 2, 3], q) });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_first_4() {
        let query = proto_vulcan_query!(|q| { first([[1, 2], 3], q) });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == lterm!([1, 2]));
        assert!(iter.next().is_none());
    }
}
