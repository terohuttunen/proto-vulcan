use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::relation::conso;
use crate::user::UserState;
use std::rc::Rc;

/// A relation such that the `first` is the first element of `list`.
pub fn firsto<U: UserState>(list: Rc<LTerm>, first: Rc<LTerm>) -> Rc<dyn Goal<U>> {
    proto_vulcan!(|rest| { conso(first, rest, list) })
}

#[cfg(test)]
mod test {
    use super::firsto;
    use crate::*;

    #[test]
    fn test_firsto_1() {
        let query = proto_vulcan_query!(|q| { firsto([1], q) });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_firsto_2() {
        let query = proto_vulcan_query!(|q| { firsto([1, 2], q) });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_firsto_3() {
        let query = proto_vulcan_query!(|q| { firsto([1, 2, 3], q) });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_firsto_4() {
        let query = proto_vulcan_query!(|q| { firsto([[1, 2], 3], q) });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == lterm!([1, 2]));
        assert!(iter.next().is_none());
    }
}
