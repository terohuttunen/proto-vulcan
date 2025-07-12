use crate::engine::Engine;
use crate::user::User;

/// Integration utilities for connecting with the existing runtime
pub struct Integration<U: User, E: Engine<U>> {
    _phantom: std::marker::PhantomData<(U, E)>,
}

impl<U: User, E: Engine<U>> Integration<U, E> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}
