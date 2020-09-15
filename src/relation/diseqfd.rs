/// Constrain disequality in finite domains
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::state::State;
use crate::state::{BaseConstraint, DiseqFdConstraint};
use crate::stream::Stream;
use crate::user::UserState;
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DiseqFd<U: UserState> {
    u: Rc<LTerm>,
    v: Rc<LTerm>,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<U>,
}

impl<U: UserState> DiseqFd<U> {
    pub fn new(u: Rc<LTerm>, v: Rc<LTerm>) -> Rc<dyn Goal<U>> {
        Rc::new(DiseqFd {
            u,
            v,
            _phantom: PhantomData,
        })
    }
}

impl<U: UserState> Goal<U> for DiseqFd<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        let u = Rc::clone(&self.u);
        let v = Rc::clone(&self.v);
        let c = Rc::new(DiseqFdConstraint::new(u, v));
        Stream::from(c.run(state))
    }
}

/// Disequality relation for finite domains.
///
/// Note: The built-in syntax `x != y` does not work with finite domains.
/// ```rust
/// extern crate proto_vulcan;
/// use proto_vulcan::*;
/// use proto_vulcan::relation::diseqfd;
/// use proto_vulcan::relation::infd;
/// fn main() {
///     let query = proto_vulcan_query!(|q| {
///         |x, y| {
///             infd(x, #&[1, 2]),
///             infd(y, #&[2, 3]),
///             diseqfd(x, y),
///             q == [x, y],
///         }
///     });
///     let mut iter = query.run();
///     assert!(iter.next().unwrap().q == lterm!([2, 3]));
///     assert!(iter.next().unwrap().q == lterm!([1, 2]));
///     assert!(iter.next().unwrap().q == lterm!([1, 3]));
///     assert!(iter.next().is_none())
/// }
/// ```
pub fn diseqfd<U: UserState>(u: &Rc<LTerm>, v: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    DiseqFd::new(Rc::clone(u), Rc::clone(v))
}

#[cfg(test)]
mod tests {
    use super::diseqfd;
    use crate::relation::infd::infd;
    use crate::*;

    #[test]
    fn test_diseqfd_1() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                infd(x, #&[1, 2]),
                infd(y, #&[2, 3]),
                infd([z, q], #&[2, 4]),
                x == y,
                diseqfd(x, z),
                q == z,
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 4);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_diseqfd_2() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                x == y,
                infd(y, #&[2, 3]),
                diseqfd(x, z),
                infd([z, q], #&[2, 4]),
                q == z,
                infd(x, #&[1, 2]),
            }
        });
        let mut iter = query.run();
        assert_eq!(iter.next().unwrap().q, 4);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_diseqfd_3() {
        let query = proto_vulcan_query!(|x, y| {
            infd(x, #&[1, 2]),
            infd(y, #&[2, 3]),
            x == y,
        });
        let mut iter = query.run();
        let result = iter.next().unwrap();
        assert_eq!(result.x, 2);
        assert_eq!(result.y, 2);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_diseqfd_4() {
        let query = proto_vulcan_query!(|q| {
            |x, y, z| {
                infd([x, y, z], #&[1, 2]),
                diseqfd(x, y),
                diseqfd(x, z),
                diseqfd(y, z),
            }
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }
}
