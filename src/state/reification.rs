use crate::goal::Goal;
use crate::lterm::{LTerm, LTermInner};
use crate::operator::onceo;
use crate::state::{State, User};
use crate::stream::{LazyStream, Stream};

fn enforce_constraints_diseq<U: User>(_x: LTerm) -> Goal<U> {
    proto_vulcan!(true)
}

/// Map function `f` to each value of iterable (stream of separate solutions)
fn map_sum<U, F, T>(
    state: State<U>,
    mut f: F,
    iter: impl DoubleEndedIterator<Item = T>,
) -> Stream<U>
where
    U: User,
    F: FnMut(T) -> Goal<U>,
{
    let mut iter = iter.rev().peekable();
    let mut stream = Stream::Empty;
    loop {
        match iter.next() {
            Some(d) => {
                if iter.peek().is_none() {
                    // If this is last value in the domain, no need to clone `state`.
                    let new_stream = f(d).apply(state);
                    stream = Stream::mplus(new_stream, LazyStream::from_stream(stream));
                    break;
                } else {
                    let new_stream = f(d).apply(state.clone());
                    stream = Stream::mplus(new_stream, LazyStream::from_stream(stream));
                }
            }
            None => {
                unreachable!();
            }
        }
    }
    stream
}

/// Enforces the finite domain constraints by expanding the domains into sequences of numbers,
/// and returning solutions for all numbers. Adds a `x == d` substitution for each `d` in
/// the domain.
fn force_ans<U: User>(x: LTerm) -> Goal<U> {
    proto_vulcan!(fngoal move |state| {
        let xwalk = state.smap_ref().walk(&x).clone();
        let maybe_xdomain = state.dstore_ref().get(&xwalk).cloned();

        match (xwalk.as_ref(), maybe_xdomain) {
            (LTermInner::Var(_, _), Some(xdomain)) => {
                // Stream of solutions where xwalk can equal any value of xdomain
                map_sum(state, |d| {
                    let dterm = LTerm::from(d);
                    proto_vulcan!(dterm == xwalk)
                }, xdomain.iter())
            }
            (LTermInner::Cons(head, tail), _) => proto_vulcan!([force_ans(head), force_ans(tail)]).apply(state),
            (_, _) => proto_vulcan!(true).apply(state),
        }
    })
}

fn enforce_constraints_fd<U: User>(x: LTerm) -> Goal<U> {
    proto_vulcan!([
        force_ans(x),
        fngoal | state | {
            state.verify_all_bound();
            let bound_x = state.dstore_ref().keys().cloned().collect::<LTerm>();
            proto_vulcan!( onceo { force_ans(bound_x) } ).apply(state)
        }
    ])
}

/// A goal that enforces the current set of constraints.
///
/// The constraints are enforced just before reification.
///
/// For finite domain constraints it means that the domains are converted into sequences
/// of answers such that the result variariables always have singular domains.
///
/// For disequality constraints this is a no-op.
fn enforce_constraints<U: User>(x: LTerm) -> Goal<U> {
    proto_vulcan!([enforce_constraints_diseq(x), enforce_constraints_fd(x)])
}

pub fn reify<U: User>(x: LTerm) -> Goal<U> {
    proto_vulcan!([
        enforce_constraints(x),
        fngoal move |state| {
            let smap = state.get_smap();
            let v = smap.walk_star(&x);
            let r = smap.reify(&v);
            let cstore = state.get_cstore().walk_star(&smap);
            Stream::unit(Box::new(state.with_smap(r).with_cstore(cstore)))
        }
    ])
}
