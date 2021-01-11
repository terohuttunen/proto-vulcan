use super::substitution::SMap;
use super::{SResult, State, User};
use crate::lterm::LTerm;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ptr;
use std::rc::Rc;

mod tree;
pub use tree::DisequalityConstraint;

mod fd;
pub use fd::{
    DiseqFdConstraint, DistinctFdConstraint, FiniteDomain, LessThanOrEqualFdConstraint,
    MinusFdConstraint, PlusFdConstraint, TimesFdConstraint,
};

mod z;
pub use z::{PlusZConstraint, TimesZConstraint};

pub mod store;

pub trait BaseConstraint<U: User>: fmt::Debug + fmt::Display {
    // The only mandatory method. Must add the requirement to the state's constraint store
    // if it is still relevant.
    fn run(self: Rc<Self>, state: State<U>) -> SResult<U>;

    /// Get list of operands
    fn operands(&self) -> Vec<LTerm<U>>;

    fn reify(&self, _state: &mut State<U>) {}
}

pub trait TreeConstraint<U: User>: BaseConstraint<U> {
    // May return `true` if `other` is subsumable
    fn subsumes(&self, other: &dyn TreeConstraint<U>) -> bool;

    // Returns substitution map if subsumable
    fn smap_ref(&self) -> &SMap<U>;

    fn walk_star(&self, smap: &SMap<U>) -> SMap<U>;
}

pub trait FiniteDomainConstraint<U: User>: BaseConstraint<U> {}

pub trait ZConstraint<U: User>: BaseConstraint<U> {}

pub trait UserConstraint<U: User>: BaseConstraint<U> {}

#[derive(Clone, Debug)]
pub enum Constraint<U: User> {
    Tree(Rc<dyn TreeConstraint<U>>),
    FiniteDomain(Rc<dyn FiniteDomainConstraint<U>>),
    Z(Rc<dyn ZConstraint<U>>),
    User(Rc<dyn UserConstraint<U>>),
}

impl<U: User> Constraint<U> {
    pub fn is_tree(&self) -> bool {
        match self {
            Constraint::Tree(_) => true,
            _ => false,
        }
    }

    pub fn is_finite_domain(&self) -> bool {
        match self {
            Constraint::FiniteDomain(_) => true,
            _ => false,
        }
    }

    pub fn is_user(&self) -> bool {
        match self {
            Constraint::User(_) => true,
            _ => false,
        }
    }

    pub fn run(self, state: State<U>) -> SResult<U> {
        match self {
            Constraint::Tree(constraint) => constraint.run(state),
            Constraint::FiniteDomain(constraint) => constraint.run(state),
            Constraint::Z(constraint) => constraint.run(state),
            Constraint::User(constraint) => constraint.run(state),
        }
    }

    /// Get list of operands
    pub fn operands(&self) -> Vec<LTerm<U>> {
        match self {
            Constraint::Tree(constraint) => constraint.operands(),
            Constraint::FiniteDomain(constraint) => constraint.operands(),
            Constraint::Z(constraint) => constraint.operands(),
            Constraint::User(constraint) => constraint.operands(),
        }
    }

    pub fn reify(&self, state: &mut State<U>) {
        match self {
            Constraint::Tree(constraint) => BaseConstraint::reify(constraint.as_ref(), state),
            Constraint::FiniteDomain(constraint) => {
                BaseConstraint::reify(constraint.as_ref(), state)
            }
            Constraint::Z(constraint) => BaseConstraint::reify(constraint.as_ref(), state),
            Constraint::User(constraint) => BaseConstraint::reify(constraint.as_ref(), state),
        }
    }
}

impl<U: User> Hash for Constraint<U> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Constraint::Tree(constraint) => ptr::hash(&**constraint, state),
            Constraint::FiniteDomain(constraint) => ptr::hash(&**constraint, state),
            Constraint::Z(constraint) => ptr::hash(&**constraint, state),
            Constraint::User(constraint) => ptr::hash(&**constraint, state),
        }
    }
}

impl<U: User> std::fmt::Display for Constraint<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Constraint::Tree(constraint) => std::fmt::Display::fmt(constraint, f),
            Constraint::FiniteDomain(constraint) => std::fmt::Display::fmt(constraint, f),
            Constraint::Z(constraint) => std::fmt::Display::fmt(constraint, f),
            Constraint::User(constraint) => std::fmt::Display::fmt(constraint, f),
        }
    }
}

impl<U: User> PartialEq for Constraint<U> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Constraint::Tree(left), Constraint::Tree(right)) => Rc::ptr_eq(left, right),
            (Constraint::FiniteDomain(left), Constraint::FiniteDomain(right)) => {
                Rc::ptr_eq(left, right)
            }
            (Constraint::Z(left), Constraint::Z(right)) => Rc::ptr_eq(left, right),
            (Constraint::User(left), Constraint::User(right)) => Rc::ptr_eq(left, right),
            _ => false,
        }
    }
}

impl<U: User> Eq for Constraint<U> {}

impl<U: User> From<Rc<dyn TreeConstraint<U>>> for Constraint<U> {
    fn from(c: Rc<dyn TreeConstraint<U>>) -> Constraint<U> {
        Constraint::Tree(c)
    }
}

impl<U: User> From<Rc<dyn FiniteDomainConstraint<U>>> for Constraint<U> {
    fn from(c: Rc<dyn FiniteDomainConstraint<U>>) -> Constraint<U> {
        Constraint::FiniteDomain(c)
    }
}

impl<U: User> From<Rc<dyn ZConstraint<U>>> for Constraint<U> {
    fn from(c: Rc<dyn ZConstraint<U>>) -> Constraint<U> {
        Constraint::Z(c)
    }
}

impl<U: User> From<Rc<dyn UserConstraint<U>>> for Constraint<U> {
    fn from(c: Rc<dyn UserConstraint<U>>) -> Constraint<U> {
        Constraint::User(c)
    }
}
