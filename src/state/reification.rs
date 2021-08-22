use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::{LTerm, LTermInner};
use crate::operator::onceo;
use crate::solver::Solver;
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;

fn enforce_constraints_diseq<U: User, E: Engine<U>>(_x: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(true)
}

fn map_sum<U, E, F, T>(
    solver: &Solver<U, E>,
    state: State<U, E>,
    mut f: F,
    iter: impl DoubleEndedIterator<Item = T>,
) -> Stream<U, E>
where
    U: User,
    E: Engine<U>,
    F: FnMut(T) -> Goal<U, E>,
{
    let mut iter = iter.rev().peekable();
    let mut stream = Stream::empty();
    loop {
        match iter.next() {
            Some(d) => {
                if iter.peek().is_none() {
                    // If this is last value in the domain, no need to clone `state`.
                    let new_stream = f(d).solve(solver, state);
                    stream = Stream::mplus(new_stream, LazyStream::delay(stream));
                    break;
                } else {
                    let new_stream = f(d).solve(solver, state.clone());
                    stream = Stream::mplus(new_stream, LazyStream::delay(stream));
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
fn force_ans<U: User, E: Engine<U>>(x: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(fngoal move |solver, state| {
        let xwalk: LTerm<U, E> = state.smap_ref().walk(&x).clone();
        let maybe_xdomain = state.dstore_ref().get(&xwalk).cloned();

        match (xwalk.as_ref(), maybe_xdomain) {
            (LTermInner::<U, E>::Var(_, _), Some(xdomain)) => {
                // Stream of solutions where xwalk can equal any value of xdomain
                map_sum(solver, state, |d| {
                    let dterm = LTerm::from(d);
                    proto_vulcan!(dterm == xwalk)
                }, xdomain.iter())

            }
            (LTermInner::<U, E>::Cons(head, tail), _) => {
                let head: LTerm<U, E> = head.clone();
                let tail: LTerm<U, E> = tail.clone();
                proto_vulcan!([
                    force_ans(head),
                    force_ans(tail),
                ]).solve(solver, state)
            },
            (_, _) => proto_vulcan!(true).solve(solver, state),
        }
    })
}

fn enforce_constraints_fd<U: User, E: Engine<U>>(x: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!([
        force_ans(x),
        fngoal | engine,
        state | {
            state.verify_all_bound();
            let bound_x = state.dstore_ref().keys().cloned().collect::<LTerm<U, E>>();
            proto_vulcan!( onceo { force_ans(bound_x) } ).solve(engine, state)
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
fn enforce_constraints<U: User, E: Engine<U>>(x: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!([
        enforce_constraints_diseq(x),
        enforce_constraints_fd(x),
        U::enforce_constraints(x)
    ])
}

pub fn reify<U: User, E: Engine<U>>(x: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!([
        enforce_constraints(x),
        fngoal move |_engine, state| {
            let smap = state.get_smap();
            let v = smap.walk_star(&x);
            let r = smap.reify(&v);
            let cstore = state.get_cstore().walk_star(&smap);
            Stream::unit(Box::new(state.with_smap(r).with_cstore(cstore)))
        }
    ])
}
