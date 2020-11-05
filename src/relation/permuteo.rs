use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::relation::rembero;
use crate::user::UserState;
use std::rc::Rc;

/// A relation that will permute xl into yl.
pub fn permuteo<U: UserState>(xl: &Rc<LTerm>, yl: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    let xl = Rc::clone(xl);
    let yl = Rc::clone(yl);
    proto_vulcan_closure!(
        match [xl, yl] {
            [[], []] => ,
            [[x | xs], _] => |ys| {
                permuteo(xs, ys),
                rembero(x, yl, ys),
            }
        }
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    #[test]
    fn test_permuteo_1() {
        let query = proto_vulcan_query!(|q| { permuteo([1, 2], q) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([]));
        assert_eq!(iter.next().unwrap().q, lterm!([1]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([2]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 1]));
        assert!(iter.next().is_none());
    }
}
