use crate::engine::Engine;
use crate::goal::{AnyGoal, InferredGoal};
use crate::lterm::LTerm;
use crate::relation::rember;
use crate::user::User;

/// A relation that will permute xl into yl.
pub fn permute<U, E, G>(xl: LTerm<U, E>, yl: LTerm<U, E>) -> InferredGoal<U, E, G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    proto_vulcan_closure!(
        match [xl, yl] {
            [[], []] => ,
            [[x | xs], _] => |ys| {
                permute(xs, ys),
                rember(x, yl, ys),
            }
        }
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_permute_1() {
        let query = proto_vulcan_query!(|q| { permute([1, 2], q) });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, lterm!([]));
        assert_eq!(iter.next().unwrap().q, lterm!([1]));
        assert_eq!(iter.next().unwrap().q, lterm!([1, 2]));
        assert_eq!(iter.next().unwrap().q, lterm!([2]));
        assert_eq!(iter.next().unwrap().q, lterm!([2, 1]));
        assert!(iter.next().is_none());
    }
}
