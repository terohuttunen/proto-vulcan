use crate::goal::{Goal, Solver};
use crate::lterm::LTerm;
use crate::operator::all::All;
use crate::operator::ProjectOperatorParam;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;

#[derive(Debug)]
pub struct Project<U: User> {
    variables: Vec<LTerm>,
    body: Goal<U>,
}

impl<U: User> Project<U> {
    pub fn new(variables: Vec<LTerm>, body: Goal<U>) -> Goal<U> {
        Goal::new(Project { variables, body }) as Goal<U>
    }
}

impl<U: User> Solver<U> for Project<U> {
    fn solve(&self, state: State<U>) -> Stream<U> {
        // Walk* each projected variable with the current substitution
        for v in self.variables.iter() {
            v.project(|x| state.smap_ref().walk_star(x));
        }
        self.body.solve(state)
    }
}

pub fn project<U: User>(param: ProjectOperatorParam<U>) -> Goal<U> {
    Project::new(param.var_list, All::from_conjunctions(param.body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use std::marker::PhantomData;
    use crate::lterm::LTermInner;

    #[derive(Debug)]
    pub struct SqEq<U: User> {
        u: LTerm,
        v: LTerm,
        _phantom: PhantomData<U>,
    }

    impl<U: User> SqEq<U> {
        pub fn new(u: LTerm, v: LTerm) -> Goal<U> {
            Goal::new(SqEq {
                u,
                v,
                _phantom: PhantomData,
            })
        }
    }

    impl<U: User> Solver<U> for SqEq<U> {
        fn solve(&self, state: State<U>) -> Stream<U> {
            let u = self.u.clone();
            let v = self.v.clone();
            proto_vulcan!(fngoal move |state| {
                println!("u = {}", u);
                match u.as_ref() {
                    // sqeq is non-relational operator and requires `u` to be associated with
                    // integer value to succeed.
                    LTermInner::Val(LValue::Number(u)) => {
                        let sq = LTerm::from(u * u);
                        Stream::from(state.unify(&sq, &v))
                    }
                    _ => Stream::from(Err(())),
                }
            })
            .solve(state)
        }
    }

    fn sqeq<U: User>(u: LTerm, v: LTerm) -> Goal<U> {
        SqEq::new(u, v)
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
