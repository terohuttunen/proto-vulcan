use crate::goal::Goal;
use crate::state::State;
use crate::stream::Stream;
use crate::user::{EmptyUser, User};
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::rc::Rc;

#[doc(hidden)]
pub trait ReifyQuery<R, U = EmptyUser>
where
    U: User,
{
    fn reify(&self, state: &State<U>) -> R;
}

pub struct ResultIterator<V: ReifyQuery<R, U>, R, U: User> {
    variables: Rc<V>,
    stream: Stream<U>,
    _phantom: PhantomData<R>,
}

#[doc(hidden)]
impl<V: ReifyQuery<R, U>, R, U: User> ResultIterator<V, R, U> {
    pub fn new(
        variables: Rc<V>,
        goal: Goal<U>,
        initial_state: State<U>,
    ) -> ResultIterator<V, R, U> {
        let stream = goal.solve(initial_state);
        ResultIterator {
            variables,
            stream,
            _phantom: PhantomData,
        }
    }
}

#[doc(hidden)]
impl<V: ReifyQuery<R, U>, R, U: User> Iterator for ResultIterator<V, R, U> {
    type Item = R;

    fn next(&mut self) -> Option<Self::Item> {
        match self.stream.next() {
            Some(state) => {
                // At this point the state has already gone through initial reification
                // process
                let result = self.variables.reify(&state);
                Some(result)
            }
            None => None,
        }
    }
}

/* ResultIterator is fused because uncons() will always keep returning None on empty stream */
#[doc(hidden)]
impl<V: ReifyQuery<R, U>, R, U: User> FusedIterator for ResultIterator<V, R, U> {}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Query<V, R, U = EmptyUser>
where
    V: ReifyQuery<R, U>,
    U: User,
{
    variables: Rc<V>,
    goal: Goal<U>,
    #[derivative(Debug = "ignore")]
    _phantom: PhantomData<R>,
}

impl<V: ReifyQuery<R, U>, R, U: User> Query<V, R, U> {
    pub fn new(variables: Rc<V>, goal: Goal<U>) -> Query<V, R, U> {
        Query {
            variables,
            goal,
            _phantom: PhantomData,
        }
    }

    pub fn run_with_user(&self, user_state: U) -> ResultIterator<V, R, U> {
        let initial_state = State::new(user_state);
        ResultIterator::new(Rc::clone(&self.variables), self.goal.clone(), initial_state)
    }
}

impl<V: ReifyQuery<R, EmptyUser>, R> Query<V, R, EmptyUser> {
    pub fn run(&self) -> ResultIterator<V, R, EmptyUser> {
        let user_state = EmptyUser::new();
        let initial_state = State::new(user_state);
        ResultIterator::new(Rc::clone(&self.variables), self.goal.clone(), initial_state)
    }
}

#[macro_export]
macro_rules! proto_vulcan_query {
    (| $($query:ident),+ | { $( $body:tt )* } ) => {{
        use $crate::state::State;
        use $crate::user::{User, EmptyUser};
        use std::fmt;
        use std::rc::Rc;
        use $crate::lresult::LResult;
        use $crate::lterm::LTerm;
        use $crate::query::{ReifyQuery};

        #[derive(Clone, Debug)]
        struct QueryResult<U = EmptyUser>
        where
            U: User,
        {
            $( $query: LResult<U>, )+
        }

        impl<U: User> fmt::Display for QueryResult<U> {
            #[allow(unused_variables, unused_assignments)]
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let mut count = 0;
                $( if count > 0 { writeln!(f, "")?; }  write!(f, "{}: {}", stringify!($query), self.$query)?; count += 1; )+
                write!(f, "")
            }
        }

        /* The query variables */
        $(let $query = LTerm::var(stringify!($query));)+
        #[derive(Debug)]
        struct QueryVariables<R, U = EmptyUser>
        where
            U: User,
        {
            $( $query: LTerm<U>, )+
            _phantom: ::std::marker::PhantomData<R>,
        }

        impl<U: User> ReifyQuery<QueryResult<U>, U> for QueryVariables<QueryResult<U>, U> {
            fn reify(&self, state: &State<U>) -> QueryResult<U> {
                let smap = state.smap_ref();
                let purified_cstore = state.cstore_ref().clone().purify(smap).normalize();
                let reified_cstore = Rc::new(purified_cstore.walk_star(smap));
                QueryResult {
                    $( $query: LResult(state.smap_ref().walk_star(&self.$query), Rc::clone(&reified_cstore)), )+
                }
            }
        }

        let vars = Rc::new(QueryVariables {
            $($query: ::std::clone::Clone::clone(&$query),)+
            _phantom: ::std::marker::PhantomData,
        });

        use $crate::state::reify;
        let reified_query = proto_vulcan!(|__query__| {
            __query__ == [$($query),+],
            [ $( $body )* ],
            reify(__query__)
        });

        $crate::query::Query::new(Rc::clone(&vars), reified_query)
    }};
}
