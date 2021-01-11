use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::state::constraint::store::ConstraintStore;
use crate::state::constraint::Constraint;
use crate::user::User;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct LResult<U: User>(pub LTerm<U>, pub Rc<ConstraintStore<U>>);

impl<U: User> LResult<U> {
    /// Check if the wrapped LTerm is an Any-variable with constraints such that it cannot be
    /// the `exception`.
    pub fn is_any_except<T>(&self, exception: &T) -> bool
    where
        T: PartialEq<LTerm<U>>,
    {
        if self.0.is_any() {
            // result is an `any` variable, see if it has the expected constraint
            for constraint in self.constraints() {
                if let Constraint::Tree(tree) = constraint {
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
    pub fn constraints<'a>(&'a self) -> impl Iterator<Item = &'a Constraint<U>> {
        let anyvars = self.0.anyvars();
        self.1.relevant(&anyvars)
    }
}

impl<U: User> Deref for LResult<U> {
    type Target = LTerm<U>;

    fn deref(&self) -> &LTerm<U> {
        &self.0
    }
}

impl<U: User> fmt::Display for LResult<U> {
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

impl<U: User> PartialEq<LTerm<U>> for LResult<U> {
    fn eq(&self, other: &LTerm<U>) -> bool {
        &self.0 == other
    }
}

impl<U: User> PartialEq<LResult<U>> for LTerm<U> {
    fn eq(&self, other: &LResult<U>) -> bool {
        &other.0 == self
    }
}

impl<U: User> PartialEq<LValue> for LResult<U> {
    fn eq(&self, other: &LValue) -> bool {
        match self.as_ref() {
            LTermInner::Val(v) => v == other,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LResult<U>> for LValue {
    fn eq(&self, other: &LResult<U>) -> bool {
        match other.as_ref() {
            LTermInner::Val(v) => v == self,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<bool> for LResult<U> {
    fn eq(&self, other: &bool) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Bool(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LResult<U>> for bool {
    fn eq(&self, other: &LResult<U>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Bool(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<isize> for LResult<U> {
    fn eq(&self, other: &isize) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Number(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LResult<U>> for isize {
    fn eq(&self, other: &LResult<U>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Number(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<char> for LResult<U> {
    fn eq(&self, other: &char) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Char(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LResult<U>> for char {
    fn eq(&self, other: &LResult<U>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Char(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<String> for LResult<U> {
    fn eq(&self, other: &String) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LResult<U>> for String {
    fn eq(&self, other: &LResult<U>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<&str> for LResult<U> {
    fn eq(&self, other: &&str) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LResult<U>> for &str {
    fn eq(&self, other: &LResult<U>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}
