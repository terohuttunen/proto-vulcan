use crate::lterm::LTerm;
use crate::state::{SMap, SResult, State};
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;

pub trait User: Debug + Clone + Default + 'static {
    type UserTerm: Debug + Clone + Hash + PartialEq + Eq;

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
}
