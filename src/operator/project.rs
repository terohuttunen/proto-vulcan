use crate::engine::Engine;
use crate::goal::{Goal, Solve};
use crate::lterm::LTerm;
use crate::operator::all::All;
use crate::operator::ProjectOperatorParam;
use crate::state::State;
use crate::stream::Stream;
use crate::user::User;

#[derive(Debug)]
pub struct Project<U, E>
where
    U: User,
    E: Engine<U>,
{
    variables: Vec<LTerm<U, E>>,
    body: Goal<U, E>,
}

impl<U, E> Project<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(variables: Vec<LTerm<U, E>>, body: Goal<U, E>) -> Goal<U, E> {
        Goal::new(Project { variables, body }) as Goal<U, E>
    }
}

impl<U, E> Solve<U, E> for Project<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, engine: &E, state: State<U, E>) -> Stream<U, E> {
        // Walk* each projected variable with the current substitution
        for v in self.variables.iter() {
            v.project(|x| state.smap_ref().walk_star(x));
        }
        self.body.solve(engine, state)
    }
}

pub fn project<U, E>(param: ProjectOperatorParam<U, E>) -> Goal<U, E>
where
    U: User,
    E: Engine<U>,
{
    Project::new(param.var_list, All::from_conjunctions(param.body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Engine;
    use crate::lterm::LTermInner;
    use crate::prelude::*;

    #[derive(Debug)]
    pub struct SqEq<U: User, E: Engine<U>> {
        u: LTerm<U, E>,
        v: LTerm<U, E>,
    }

    impl<U: User, E: Engine<U>> SqEq<U, E> {
        pub fn new(u: LTerm<U, E>, v: LTerm<U, E>) -> Goal<U, E> {
            Goal::new(SqEq { u, v })
        }
    }

    impl<U, E> Solve<U, E> for SqEq<U, E>
    where
        U: User,
        E: Engine<U>,
    {
        fn solve(&self, engine: &E, state: State<U, E>) -> Stream<U, E> {
            let u = self.u.clone();
            let v = self.v.clone();
            let g: Goal<U, E> = proto_vulcan!(fngoal move |_engine, state| {
                match u.as_ref() {
                    // sqeq is non-relational operator and requires `u` to be associated with
                    // integer value to succeed.
                    LTermInner::Val(LValue::Number(u)) => {
                        let sq = LTerm::from(u * u);
                        Stream::unit(Box::new(state.unify(&sq, &v).unwrap()))
                    }
                    _ => Stream::empty(),
                }
            });
            g.solve(engine, state)
        }
    }

    fn sqeq<U, E>(u: LTerm<U, E>, v: LTerm<U, E>) -> Goal<U, E>
    where
        U: User,
        E: Engine<U>,
    {
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
