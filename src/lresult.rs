use crate::lterm::LTerm;
use crate::lvalue::LValue;
use crate::state::constraint::store::ConstraintStore;
use crate::state::constraint::Constraint;
use crate::state::UserState;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct LResult<U: UserState>(pub Rc<LTerm>, pub Rc<ConstraintStore<U>>);

impl<U: UserState> LResult<U> {
    /// Check if the wrapped LTerm is an Any-variable with constraints such that it cannot be
    /// the `exception`.
    pub fn is_any_except<T>(&self, exception: &T) -> bool
    where
        T: PartialEq<LTerm>,
    {
        if self.0.is_any() {
            // result is an `any` variable, see if it has the expected constraint
            for constraint in self.constraints() {
                if let Constraint::Tree(tree) = constraint {
                    for (cu, cv) in tree.smap_ref().iter() {
                        if &self.0 == cu && exception == &**cv
                            || &self.0 == cv && exception == &**cu
                        {
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

impl<U: UserState> Deref for LResult<U> {
    type Target = Rc<LTerm>;

    fn deref(&self) -> &Rc<LTerm> {
        &self.0
    }
}

impl<U: UserState> fmt::Display for LResult<U> {
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

impl<U: UserState> PartialEq<LTerm> for LResult<U> {
    fn eq(&self, other: &LTerm) -> bool {
        &*self == other
    }
}

impl<U: UserState> PartialEq<LResult<U>> for LTerm {
    fn eq(&self, other: &LResult<U>) -> bool {
        &*other == self
    }
}

impl<U: UserState> PartialEq<Rc<LTerm>> for LResult<U> {
    fn eq(&self, other: &Rc<LTerm>) -> bool {
        &self.0 == other
    }
}

impl<U: UserState> PartialEq<LResult<U>> for Rc<LTerm> {
    fn eq(&self, other: &LResult<U>) -> bool {
        &other.0 == self
    }
}

impl<U: UserState> PartialEq<LValue> for LResult<U> {
    fn eq(&self, other: &LValue) -> bool {
        match self.as_ref() {
            LTerm::Val(v) => v == other,
            _ => false,
        }
    }
}

impl<U: UserState> PartialEq<LResult<U>> for LValue {
    fn eq(&self, other: &LResult<U>) -> bool {
        match other.as_ref() {
            LTerm::Val(v) => v == self,
            _ => false,
        }
    }
}

impl<U: UserState> PartialEq<bool> for LResult<U> {
    fn eq(&self, other: &bool) -> bool {
        match self.as_ref() {
            LTerm::Val(LValue::Bool(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: UserState> PartialEq<LResult<U>> for bool {
    fn eq(&self, other: &LResult<U>) -> bool {
        match other.as_ref() {
            LTerm::Val(LValue::Bool(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: UserState> PartialEq<isize> for LResult<U> {
    fn eq(&self, other: &isize) -> bool {
        match self.as_ref() {
            LTerm::Val(LValue::Number(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: UserState> PartialEq<LResult<U>> for isize {
    fn eq(&self, other: &LResult<U>) -> bool {
        match other.as_ref() {
            LTerm::Val(LValue::Number(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: UserState> PartialEq<char> for LResult<U> {
    fn eq(&self, other: &char) -> bool {
        match self.as_ref() {
            LTerm::Val(LValue::Char(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: UserState> PartialEq<LResult<U>> for char {
    fn eq(&self, other: &LResult<U>) -> bool {
        match other.as_ref() {
            LTerm::Val(LValue::Char(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: UserState> PartialEq<String> for LResult<U> {
    fn eq(&self, other: &String) -> bool {
        match self.as_ref() {
            LTerm::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: UserState> PartialEq<LResult<U>> for String {
    fn eq(&self, other: &LResult<U>) -> bool {
        match other.as_ref() {
            LTerm::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: UserState> PartialEq<&str> for LResult<U> {
    fn eq(&self, other: &&str) -> bool {
        match self.as_ref() {
            LTerm::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: UserState> PartialEq<LResult<U>> for &str {
    fn eq(&self, other: &LResult<U>) -> bool {
        match other.as_ref() {
            LTerm::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}
