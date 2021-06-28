use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::engine::Engine;
use crate::relation::diseq::DisequalityConstraint;
use crate::state::constraint::store::ConstraintStore;
use crate::state::constraint::Constraint;
use crate::user::User;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct LResult<U: User, E: Engine<U>>(pub LTerm<U, E>, pub Rc<ConstraintStore<U, E>>);

impl<U, E> LResult<U, E>
where
    U: User,
    E: Engine<U>,
{
    /// Check if the wrapped LTerm is an Any-variable with constraints such that it cannot be
    /// the `exception`.
    pub fn is_any_except<T>(&self, exception: &T) -> bool
    where
        T: PartialEq<LTerm<U, E>>,
    {
        if self.0.is_any() {
            // result is an `any` variable, see if it has the expected constraint
            for constraint in self.constraints() {
                if let Some(tree) = constraint.downcast_ref::<DisequalityConstraint<U, E>>() {
                    for (cu, cv) in tree.smap_ref().iter() {
                        if &self.0 == cu && exception == cv || &self.0 == cv && exception == cu {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Check if the wrapped LTerm is constrained by any constraint.
    pub fn is_constrained(&self) -> bool {
        self.constraints().any(|_| true)
    }

    /// Returns iterator to constraints that refer to the wrapped LTerm.
    pub fn constraints<'a>(&'a self) -> impl Iterator<Item = &'a Rc<dyn Constraint<U, E>>> {
        let anyvars = self.0.anyvars();
        self.1.relevant(&anyvars)
    }
}

impl<U, E> Deref for LResult<U, E>
where
    U: User,
    E: Engine<U>,
{
    type Target = LTerm<U, E>;

    fn deref(&self) -> &LTerm<U, E> {
        &self.0
    }
}

impl<U, E> fmt::Display for LResult<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)?;
        if self.is_constrained() {
            write!(f, "  where  {{ ")?;
            self.1.display_relevant(&self.0, f)?;
            write!(f, " }}")
        } else {
            write!(f, "")
        }
    }
}

impl<U, E> PartialEq<LTerm<U, E>> for LResult<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LTerm<U, E>) -> bool {
        &self.0 == other
    }
}

impl<U, E> PartialEq<LResult<U, E>> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LResult<U, E>) -> bool {
        &other.0 == self
    }
}

impl<U, E> PartialEq<LValue> for LResult<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LValue) -> bool {
        match self.as_ref() {
            LTermInner::Val(v) => v == other,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<LResult<U, E>> for LValue
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LResult<U, E>) -> bool {
        match other.as_ref() {
            LTermInner::Val(v) => v == self,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<bool> for LResult<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &bool) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Bool(x)) => x == other,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<LResult<U, E>> for bool
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LResult<U, E>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Bool(x)) => x == self,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<isize> for LResult<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &isize) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Number(x)) => x == other,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<LResult<U, E>> for isize
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LResult<U, E>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Number(x)) => x == self,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<char> for LResult<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &char) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Char(x)) => x == other,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<LResult<U, E>> for char
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LResult<U, E>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Char(x)) => x == self,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<String> for LResult<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &String) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<LResult<U, E>> for String
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LResult<U, E>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<&str> for LResult<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &&str) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<LResult<U, E>> for &str
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LResult<U, E>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}
