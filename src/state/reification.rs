use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::{LTerm, LTermInner};
use crate::operator::onceo;
use crate::state::State;
use crate::user::User;

fn enforce_constraints_diseq<U: User, E: Engine<U>>(_x: LTerm<U>) -> Goal<U, E> {
    proto_vulcan!(true)
}

fn map_sum<U, E, F, T>(
    engine: &E,
    state: State<U>,
    mut f: F,
    iter: impl DoubleEndedIterator<Item = T>,
) -> E::Stream
where
    U: User,
    E: Engine<U>,
    F: FnMut(T) -> Goal<U, E>,
{
    let mut iter = iter.rev().peekable();
    let mut stream = engine.mzero();
    loop {
        match iter.next() {
            Some(d) => {
                if iter.peek().is_none() {
                    // If this is last value in the domain, no need to clone `state`.
                    let new_stream = f(d).solve(engine, state);
                    stream = engine.mplus(new_stream, engine.delay(stream));
                    break;
                } else {
                    let new_stream = f(d).solve(engine, state.clone());
                    stream = engine.mplus(new_stream, engine.delay(stream));
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
fn force_ans<U: User, E: Engine<U>>(x: LTerm<U>) -> Goal<U, E> {
    proto_vulcan!(fngoal move |engine, state| {
        let xwalk = state.smap_ref().walk(&x).clone();
        let maybe_xdomain = state.dstore_ref().get(&xwalk).cloned();

        match (xwalk.as_ref(), maybe_xdomain) {
            (LTermInner::Var(_, _), Some(xdomain)) => {
                // Stream of solutions where xwalk can equal any value of xdomain
                map_sum(engine, state, |d| {
                    let dterm = LTerm::from(d);
                    proto_vulcan!(dterm == xwalk)
                }, xdomain.iter())

            }
            (LTermInner::Cons(head, tail), _) => {
                proto_vulcan!([
                    force_ans(head),
                    force_ans(tail)
                ]).solve(engine, state)
            },
            (_, _) => proto_vulcan!(true).solve(engine, state),
        }
    })
}

fn enforce_constraints_fd<U: User, E: Engine<U>>(x: LTerm<U>) -> Goal<U, E> {
    proto_vulcan!([
        force_ans(x),
        fngoal | engine,
        state | {
            state.verify_all_bound();
            let bound_x = state.dstore_ref().keys().cloned().collect::<LTerm<U>>();
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
fn enforce_constraints<U: User, E: Engine<U>>(x: LTerm<U>) -> Goal<U, E> {
    proto_vulcan!([
        enforce_constraints_diseq(x),
        enforce_constraints_fd(x),
        U::enforce_constraints(x)
    ])
}

pub fn reify<U: User, E: Engine<U>>(x: LTerm<U>) -> Goal<U, E> {
    proto_vulcan!([
        enforce_constraints(x),
        fngoal move |engine, state| {
            let smap = state.get_smap();
            let v = smap.walk_star(&x);
            let r = smap.reify(&v);
            let cstore = state.get_cstore().walk_star(&smap);
            engine.munit(state.with_smap(r).with_cstore(cstore))
        }
    ])
}
