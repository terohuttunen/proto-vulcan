use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::relation::diseq::DisequalityConstraint;
use crate::state::constraint::store::ConstraintStore;
use crate::state::constraint::Constraint;

use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Clone, Debug)]
pub struct LResult(pub LTerm, pub Rc<ConstraintStore>);

impl PartialEq<LResult> for LResult {
    fn eq(&self, other: &LResult) -> bool {
        self.0 == other.0 && Rc::ptr_eq(&self.1, &other.1)
    }
}

impl LResult {
    /// Check if the wrapped LTerm is an Any-variable with constraints such that it cannot be
    /// the `exception`.
    pub fn is_any_except<T>(&self, exception: &T) -> bool
    where
        T: PartialEq<LTerm>,
    {
        if self.0.is_any() {
            // result is an `any` variable, see if it has the expected constraint
            for constraint in self.constraints() {
                if let Some(tree) = constraint.downcast_ref::<DisequalityConstraint>() {
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
    pub fn constraints<'a>(&'a self) -> impl Iterator<Item = &'a Rc<dyn Constraint>> {
        let anyvars = self.0.anyvars();
        self.1.relevant(&anyvars)
    }
}

impl Deref for LResult {
    type Target = LTerm;

    fn deref(&self) -> &LTerm {
        &self.0
    }
}

impl fmt::Display for LResult {
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

impl PartialEq<LTerm> for LResult {
    fn eq(&self, other: &LTerm) -> bool {
        &self.0 == other
    }
}

impl PartialEq<LResult> for LTerm {
    fn eq(&self, other: &LResult) -> bool {
        &other.0 == self
    }
}

impl PartialEq<LValue> for LResult {
    fn eq(&self, other: &LValue) -> bool {
        match self.as_ref() {
            LTermInner::Val(v) => v == other,
            _ => false,
        }
    }
}

impl PartialEq<LResult> for LValue {
    fn eq(&self, other: &LResult) -> bool {
        match other.as_ref() {
            LTermInner::Val(v) => v == self,
            _ => false,
        }
    }
}

impl PartialEq<bool> for LResult {
    fn eq(&self, other: &bool) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Bool(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LResult> for bool {
    fn eq(&self, other: &LResult) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Bool(x)) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<isize> for LResult {
    fn eq(&self, other: &isize) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Number(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LResult> for isize {
    fn eq(&self, other: &LResult) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Number(x)) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<char> for LResult {
    fn eq(&self, other: &char) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Char(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LResult> for char {
    fn eq(&self, other: &LResult) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Char(x)) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<String> for LResult {
    fn eq(&self, other: &String) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LResult> for String {
    fn eq(&self, other: &LResult) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<&str> for LResult {
    fn eq(&self, other: &&str) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LResult> for &str {
    fn eq(&self, other: &LResult) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}
