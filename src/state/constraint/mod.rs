use super::substitution::SMap;
use super::{SResult, State};
use crate::lterm::LTerm;
use std::any::{Any, TypeId};
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};
use std::ptr;
use std::rc::Rc;

pub mod store;

pub trait Constraint: Debug + Display + AnyConstraint {
    fn run(self: Rc<Self>, state: State) -> SResult;

    fn reify(&self, _state: &mut State) {}

    fn operands(&self) -> Vec<LTerm>;
}

pub trait AnyConstraint: Any {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> AnyConstraint for T
where
    T: Constraint,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl dyn Constraint {
    #[inline]
    pub fn is<T: Constraint>(&self) -> bool {
        TypeId::of::<T>() == self.type_id()
    }

    #[inline]
    pub fn downcast_ref<T: Any + Constraint>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    #[inline]
    pub fn downcast_mut<T: Constraint>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}

impl Hash for dyn Constraint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ptr::hash(self as *const Self, state)
    }
}

impl PartialEq for dyn Constraint {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl Eq for dyn Constraint {}
