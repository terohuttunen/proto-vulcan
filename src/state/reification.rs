use crate::engine::Engine;
use crate::goal::{AnyGoal, Goal};
use crate::lterm::{LTerm, LTermInner};
use crate::stream::Stream;
use crate::user::User;

#[cfg(feature = "clpfd")]
use crate::operator::onceo;

use crate::state::map_sum::map_sum;

/// Enforces the finite domain constraints by expanding the domains into sequences of numbers,
/// and returning solutions for all numbers. Adds a `x == d` substitution for each `d` in
/// the domain.
#[cfg(feature = "clpfd")]
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
                }, xdomain.iter().rev())
                /*
                map_sum_iter(state, move |d| {
                    let dterm = LTerm::from(d);
                    proto_vulcan!(dterm == xwalk)
                }, Box::new((*xdomain).clone().into_iter()))
                */
            }
            (LTermInner::<U, E>::Cons(head, tail), _) => {
                let head: LTerm<U, E> = head.clone();
                let tail: LTerm<U, E> = tail.clone();
                let g: Goal<U, E>  = proto_vulcan!([
                    force_ans(head),
                    force_ans(tail),
                ]);
                g.solve(solver, state)
            },
            (_, _) => solver.start(&Goal::Succeed, state),
        }
    })
}

#[cfg(feature = "clpfd")]
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

#[cfg(not(feature = "clpfd"))]
fn enforce_constraints_fd<U: User, E: Engine<U>>(_x: LTerm<U, E>) -> Goal<U, E> {
    Goal::succeed()
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
    proto_vulcan!([enforce_constraints_fd(x), U::enforce_constraints(x)])
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
