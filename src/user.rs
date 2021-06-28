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
    fn process_extension<E: Engine<Self>>(state: State<Self, E>, _extension: &SMap<Self, E>) -> SResult<Self, E> {
        Ok(state)
    }

    // User unification.
    fn unify<E: Engine<Self>>(
        _state: State<Self, E>,
        _extension: &mut SMap<Self, E>,
        _uwalk: LTerm<Self, E>,
        _vwalk: LTerm<Self, E>,
    ) -> SResult<Self, E> {
        Err(())
    }

    /// Called before the constraint is added to the state
    fn with_constraint<E: Engine<Self>>(_state: &mut State<Self, E>, _constraint: &Rc<dyn Constraint<Self, E>>) {}

    /// Called after the constraint has been removed from the state
    fn take_constraint<E: Engine<Self>>(_state: &mut State<Self, E>, _constraint: &Rc<dyn Constraint<Self, E>>) {}

    /// Called in reification when constraints are finalized. For example finite domain
    /// constraints are converted to sequences of integers.
    fn enforce_constraints<E: Engine<Self>>(_x: LTerm<Self, E>) -> Goal<Self, E> {
        proto_vulcan!(true)
    }

    fn finalize<E: Engine<Self>>(_state: &mut State<Self, E>) {}

    fn reify<E: Engine<Self>>(_state: &mut State<Self, E>) {}
}

#[derive(Debug, Clone)]
pub struct DefaultUser {}

impl DefaultUser {
    pub fn new() -> DefaultUser {
        DefaultUser {}
    }
}

impl fmt::Display for DefaultUser {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

impl Default for DefaultUser {
    fn default() -> DefaultUser {
        DefaultUser {}
    }
}

impl User for DefaultUser {
    type UserTerm = ();
    type UserContext = ();
}
