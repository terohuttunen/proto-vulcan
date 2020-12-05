use crate::lterm::LTerm;
use crate::state::{SMap, SResult, State};
use std::fmt::Debug;
use std::rc::Rc;

pub trait UserState: Debug + Clone + 'static {
    /// Process extension to substitution map.
    fn process_extension(state: State<Self>, _extension: &SMap) -> SResult<Self> {
        Ok(state)
    }

    fn finalize(_state: &mut State<Self>) {}

    fn reify(_state: &mut State<Self>) {}
}

pub trait UserUnify: Debug {
    /// Return Some(smap) if the terms can be unified, and None if not.
    fn unify<'a>(
        &self,
        this: &LTerm,
        other: &LTerm,
        smap: &mut Rc<SMap>,
        // Note: Extensions to `smap` must be added also to `extension`.
        extension: &mut SMap,
    ) -> bool;
}
