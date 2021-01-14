use super::substitution::SMap;
use super::{SResult, State, User};
use crate::lterm::LTerm;
use std::any::{Any, TypeId};
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};
use std::ptr;
use std::rc::Rc;

mod tree;
pub use tree::DisequalityConstraint;

mod fd;
pub use fd::{
    DiseqFdConstraint, DistinctFdConstraint, FiniteDomain, IsFiniteDomain,
    LessThanOrEqualFdConstraint, MinusFdConstraint, PlusFdConstraint, TimesFdConstraint,
};

mod z;
pub use z::{PlusZConstraint, TimesZConstraint};

pub mod store;

pub trait Constraint<U: User>: Debug + Display + AnyConstraint<U> {
    fn run(self: Rc<Self>, state: State<U>) -> SResult<U>;

    fn reify(&self, _state: &mut State<U>) {}

    fn operands(&self) -> Vec<LTerm<U>>;
}

pub trait AnyConstraint<U>: Any {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Constraint<U>, U: User> AnyConstraint<U> for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<U: User> dyn Constraint<U> {
    #[inline]
    pub fn is<T: Constraint<U>>(&self) -> bool {
        TypeId::of::<T>() == self.type_id()
    }

    #[inline]
    pub fn downcast_ref<T: Any + Constraint<U>>(&self) -> Option<&T> {
        Any::downcast_ref::<T>(self.as_any())
    }

    #[inline]
    pub fn downcast_mut<T: Constraint<U>>(&mut self) -> Option<&mut T> {
        Any::downcast_mut::<T>(self.as_any_mut())
    }
}

impl<U: User> Hash for dyn Constraint<U> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ptr::hash(self as *const Self, state)
    }
}

impl<U: User> PartialEq for dyn Constraint<U> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl<U: User> Eq for dyn Constraint<U> {}
