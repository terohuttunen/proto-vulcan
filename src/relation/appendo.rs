use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::UserState;
use std::rc::Rc;

/// A relation where `l`, `s`, and `ls` are proper lists, such that `ls` is `s` appended to `l`.
///
/// # Example
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::*;
/// use proto_vulcan::relation::appendo;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         appendo([1, 2, 3], [4, 5], q)
///     });
///     assert!(query.run().next().unwrap().q == lterm!([1, 2, 3, 4, 5]));
/// }
pub fn appendo<U: UserState>(l: &Rc<LTerm>, s: &Rc<LTerm>, ls: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    let l = Rc::clone(l);
    let s = Rc::clone(s);
    let ls = Rc::clone(ls);
    proto_vulcan_closure!(
        match [l, s, ls] {
            [[], x, x] => ,
            [[x | l1], l2, [x | l3]] => appendo(l1, l2, l3),
        }
    )
}

#[cfg(test)]
mod test {
    use super::appendo;
    use crate::*;

    #[test]
    fn test_appendo_1() {
        let query = proto_vulcan_query!(|q| { appendo([1, 2, 3], [4, 5], q) });
        assert!(query.run().next().unwrap().q == lterm!([1, 2, 3, 4, 5]));
    }
}
