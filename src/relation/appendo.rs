use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::operator::conde;
use crate::relation::conso;
use crate::relation::emptyo;
use crate::state::UserState;
use std::rc::Rc;

/// A relation where `l`, `s`, and `ls` are proper lists, such that `ls` is `s` appended to `l`.
///
/// # Example
/// ```rust
/// # #![recursion_limit = "512"]
/// use proto_vulcan::*;
/// use proto_vulcan::relation::appendo;
/// let query = proto_vulcan_query!(|q| {
///     appendo([1, 2, 3], [4, 5], q)
/// });
/// assert!(query.run().next().unwrap().q == lterm!([1, 2, 3, 4, 5]));
pub fn appendo<U: UserState>(l: &Rc<LTerm>, s: &Rc<LTerm>, ls: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    let s = Rc::clone(s);
    proto_vulcan!(
        conde {
            [s == ls, emptyo(l)],
            |a, d, res| {
                conso(a, d, l),
                conso(a, res, ls),
                closure {
                    appendo(d, s, res)
                }
            }
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
