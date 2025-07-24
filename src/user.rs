//! # User extensions
//!
//! By defining a struct that implements the `Clone`- `Debug`- and `User`-traits, the search
//! `State`-monad can be extended with any kind of information that gets cloned along with the
//! search when it forks, and discarded when branches fail. This can be used to add additional
//! clone-on-write constraint-stores, for example. The user-defined state can be accessed wherever
//! `State` is available, such as in in `fngoal |state| { }`-functions and in constraints.
//!
//! The `User`-trait provides optional hooks that the user can implement. What hooks there
//! should be is still largely TBD.
//!
//! Another way of extending Proto-vulcan is `LTerm`s that implement `UserUnify`-trait. User
//! defined state is not available in user defined unification, as `LTerm` is not parametrized
//! by the user state type.

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
    fn process_extension(state: State, _extension: &SMap) -> SResult {
        Ok(state)
    }

    // User unification.
    fn unify(_state: State, _extension: &mut SMap, _uwalk: LTerm, _vwalk: LTerm) -> SResult {
        Err(())
    }

    /// Called before the constraint is added to the state
    fn with_constraint(_state: &mut State, _constraint: &Rc<dyn Constraint>) {}

    /// Called after the constraint has been removed from the state
    fn take_constraint(_state: &mut State, _constraint: &Rc<dyn Constraint>) {}

    /// Called in reification when constraints are finalized. For example finite domain
    /// constraints are converted to sequences of integers.
    fn enforce_constraints(_x: LTerm) -> Goal {
        Goal::Succeed
    }

    fn finalize(_state: &mut State) {}

    fn reify(_state: &mut State) {}
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
