use crate::engine::Engine;
use super::substitution::SMap;
use super::{SResult, State, User};
use crate::lterm::LTerm;
use std::any::{Any, TypeId};
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};
use std::ptr;
use std::rc::Rc;

pub mod store;

pub trait Constraint<U, E>: Debug + Display + AnyConstraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn run(self: Rc<Self>, state: State<U, E>) -> SResult<U, E>;

    fn reify(&self, _state: &mut State<U, E>) {}

    fn operands(&self) -> Vec<LTerm<U, E>>;
}

pub trait AnyConstraint<U, E>: Any
where
    U: User,
    E: Engine<U>,
{
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<U, E, T> AnyConstraint<U, E> for T
where
    U: User,
    E: Engine<U>,
    T: Constraint<U, E>,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<U, E> dyn Constraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    #[inline]
    pub fn is<T: Constraint<U, E>>(&self) -> bool {
        TypeId::of::<T>() == self.type_id()
    }

    #[inline]
    pub fn downcast_ref<T: Any + Constraint<U, E>>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    #[inline]
    pub fn downcast_mut<T: Constraint<U, E>>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}

impl<U, E> Hash for dyn Constraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        ptr::hash(self as *const Self, state)
    }
}

impl<U, E> PartialEq for dyn Constraint<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl<U, E> Eq for dyn Constraint<U, E>
where
    U: User,
    E: Engine<U>,
{}
