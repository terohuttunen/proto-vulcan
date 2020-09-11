use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::operator::conde::Conde;
use crate::operator::onceo;
use crate::state::UserState;
use crate::stream::Stream;
use std::rc::Rc;

fn enforce_constraints_diseq<U: UserState>(_x: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    proto_vulcan!(true)
}

/// Enforces the finite domain constraints by expanding the domains into sequences of numbers,
/// and returning solutions for all numbers. Adds a `x == d` substitution for each `d` in
/// the domain.
fn force_ans<U: UserState>(x: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    let x = Rc::clone(x);
    proto_vulcan!(fngoal move |state| {
        let xwalk = state.smap_ref().walk(&x);
        let maybe_xdomain = state.dstore_ref().get(xwalk);

        match (xwalk.as_ref(), maybe_xdomain) {
            (LTerm::Var(_, _), Some(xdomain)) => {
                let goals = xdomain.iter().map(|d| {
                    let d = Rc::new(LTerm::from(d));
                    proto_vulcan!(xwalk == d)
                }).collect();
                Conde::from_vec(goals)
            }
            (LTerm::Cons(head, tail), _) => proto_vulcan!([force_ans(head), force_ans(tail)]),
            (_, _) => proto_vulcan!(true),
        }
        .apply(state)
    })
}

fn enforce_constraints_fd<U: UserState>(x: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    proto_vulcan!([
        force_ans(x),
        fngoal | state | {
            state.verify_all_bound();
            let bound_x = Rc::new(state.dstore_ref().keys().cloned().collect::<LTerm>());
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
fn enforce_constraints<U: UserState>(x: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    proto_vulcan!([enforce_constraints_diseq(x), enforce_constraints_fd(x)])
}

pub fn reify<U: UserState>(x: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    let y = Rc::clone(x);
    proto_vulcan!([
        enforce_constraints(x),
        fngoal move |state| {
            let smap = state.get_smap();
            let v = smap.walk_star(&y);
            let r = smap.reify(&v);
            let cstore = state.get_cstore().walk_star(&smap);
            Stream::unit(Box::new(state.with_smap(r).with_cstore(cstore)))
        }
    ])
}
