use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::state::constraint::Constraint;
use crate::state::{SMap, SResult, State};
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

pub trait User: Debug + Clone + Default + 'static {
    type UserTerm: Debug + Clone + Hash + PartialEq + Eq;

    /// Type of data-structure stored in the Engine-instance. Retrievable
    /// with Engine::context().
    type UserContext: Debug;

    /// Process extension to substitution map.
    fn process_extension(state: State<Self>, _extension: &SMap<Self>) -> SResult<Self> {
        Ok(state)
    }

    // User unification.
    fn unify(
        state: State<Self>,
        extension: &mut SMap<Self>,
        u: &LTerm<Self>,
        v: &LTerm<Self>,
    ) -> SResult<Self> {
        crate::state::unify_rec(state, extension, u, v)
    }

    /// Called before the constraint is added to the state
    fn with_constraint(_state: &mut State<Self>, _constraint: &Rc<dyn Constraint<Self>>) {}

    /// Called after the constraint has been removed from the state
    fn take_constraint(_state: &mut State<Self>, _constraint: &Rc<dyn Constraint<Self>>) {}

    /// Called in reification when constraints are finalized. For example finite domain
    /// constraints are converted to sequences of integers.
    fn enforce_constraints<E: Engine<Self>>(_x: LTerm<Self>) -> Goal<Self, E> {
        proto_vulcan!(true)
    }

    fn finalize(_state: &mut State<Self>) {}

    fn reify(_state: &mut State<Self>) {}
}

#[derive(Debug, Clone)]
pub struct EmptyUser {}

impl EmptyUser {
    pub fn new() -> EmptyUser {
        EmptyUser {}
    }
}

impl fmt::Display for EmptyUser {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

impl Default for EmptyUser {
    fn default() -> EmptyUser {
        EmptyUser {}
    }
}

impl User for EmptyUser {
    type UserTerm = ();
    type UserContext = ();
}
