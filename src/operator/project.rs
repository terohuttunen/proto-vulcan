use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::state::State;
use crate::stream::Stream;
use crate::user::UserState;
use std::rc::Rc;

#[derive(Debug)]
pub struct Project<U: UserState> {
    variables: Vec<Rc<LTerm>>,
    body: Rc<dyn Goal<U>>,
}

impl<U: UserState> Project<U> {
    pub fn new(variables: Vec<Rc<LTerm>>, body: Rc<dyn Goal<U>>) -> Rc<dyn Goal<U>> {
        Rc::new(Project { variables, body }) as Rc<dyn Goal<U>>
    }
}

impl<U: UserState> Goal<U> for Project<U> {
    fn apply(&self, state: State<U>) -> Stream<U> {
        // Walk* each projected variable with the current substitution
        for v in self.variables.iter() {
            v.project(|x| (*state.smap_ref().walk_star(x)).clone());
        }
        self.body.apply(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use std::marker::PhantomData;

    #[derive(Debug)]
    pub struct SqEq<U: UserState> {
        u: Rc<LTerm>,
        v: Rc<LTerm>,
        _phantom: PhantomData<U>,
    }

    impl<U: UserState> SqEq<U> {
        pub fn new(u: Rc<LTerm>, v: Rc<LTerm>) -> Rc<dyn Goal<U>> {
            Rc::new(SqEq {
                u,
                v,
                _phantom: PhantomData,
            })
        }
    }

    impl<U: UserState> Goal<U> for SqEq<U> {
        fn apply(&self, state: State<U>) -> Stream<U> {
            let u = Rc::clone(&self.u);
            let v = Rc::clone(&self.v);
            proto_vulcan!(fngoal move |state| {
                match u.as_ref() {
                    // sqeq is non-relational operator and requires `u` to be associated with
                    // integer value to succeed.
                    LTerm::Val(LValue::Number(u)) => {
                        let sq = Rc::new(LTerm::from(u * u));
                        Stream::from(state.unify(&sq, &v))
                    }
                    _ => Stream::from(Err(())),
                }
            })
            .apply(state)
        }
    }

    fn sqeq<U: UserState>(u: &Rc<LTerm>, v: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
        SqEq::new(Rc::clone(u), Rc::clone(v))
    }

    #[test]
    fn test_project_1() {
        // Project helps non-relational goal sqeq! to succeed
        let query = proto_vulcan_query!(|q| {
            |x| {
                5 == x,
                project |x| {
                    sqeq(x, q)
                }
            }
        });
        let mut iter = query.run();
        assert!(iter.next().unwrap().q == 25);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_project_2() {
        // Does not succeed without project!
        let query = proto_vulcan_query!(|q| {
            |x| {
                5 == x,
                sqeq(x, q)
            }
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_project_3() {
        // project! itself is non-relational, and fails if the variable is not grounded.
        let query = proto_vulcan_query!(|q| {
            |x| {
                project |x| {
                    sqeq(x, q),
                    5 == x,
                }
            }
        });
        let mut iter = query.run();
        assert!(iter.next().is_none());
    }
}
