//! Compound types provide structural types to proto-vulcan langugage.
//!
//! In proto-vulcan type system, [`LTerm`] is the supertype of all types, and all its
//! subtypes types can be cast back to it. Compound types are Rust
//! structs that are built from `LTerm`s and other compound types.
//! Proto-vulcan compound types are comparable to prolog compound types.
//!
//! ```text
//! └─ LTerm
//!    ├─ Val
//!    │  ├─ Bool
//!    │  ├─ Number
//!    │  ├─ Char
//!    │  └─ String
//!    ├─ Var
//!    │  ├─ x
//!    │  └─ _
//!    ├─ User
//!    ├─ Empty/None
//!    ├─ Cons
//!    └─ Compound
//!
//! ```
//! Compound types are further divided into compound objects and compound terms
//! -- compound terms are also always compound objects.
//!
//! # Compound objects
//!  * Destructuring directly
//!  * Cannot be recursive
//!  * Existing types can be made into objects
//!
//! # Compound terms
//!  * Destructuring via unification only
//!  * Can be recursive
//!  * Can be wildcard variable `_` or `[]` instead of structural content.
//!
//! # `use`-clauses
//! When `use`ing compound object or term `Bar`, the corresponding `Bar_compound`
//! module must also be imported for the compound type to work in proto-vulcan
//! expressions: `use path::to::{Bar, Bar_compound};`.
//!
//! # Type conversions
//! Type conversions to supertypes are done implicitly via inserted `Into::into` calls;
//! for conversions to subtypes, such as compound types, unification must be used.

use crate::lterm::{LTerm, LTermInner};
use crate::state::SMap;
use crate::user::User;
use crate::{Downcast, Upcast};
use std::any::Any;
use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub trait CompoundTerm<U>
where
    U: User,
    Self: CompoundObject<U> + Sized,
{
    fn new_var(name: &'static str) -> Self;

    fn new_wildcard() -> Self;

    fn new_none() -> Self;
}

pub trait CompoundObject<U: User>:
    CompoundHash<U> + CompoundEq<U> + CompoundAs<U> + WalkStar<U> + std::fmt::Debug
{
    fn type_name(&self) -> &'static str {
        ""
    }

    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject<U>> + 'a>;

    fn as_term(&self) -> Option<&LTerm<U>> {
        None
    }

    fn is_term(&self) -> bool {
        match self.as_term() {
            Some(_) => true,
            None => false,
        }
    }
}

pub trait WalkStar<U: User> {
    fn walk_star(&self, smap: &SMap<U>) -> LTerm<U>;
}

impl<U, T> WalkStar<U> for T
where
    U: User,
    T: CompoundWalkStar<U> + Into<LTerm<U>>,
{
    fn walk_star(&self, smap: &SMap<U>) -> LTerm<U> {
        self.compound_walk_star(smap).into()
    }
}

pub trait CompoundWalkStar<U: User> {
    fn compound_walk_star(&self, smap: &SMap<U>) -> Self;
}

pub trait CompoundAs<U: User>: Any {
    fn as_any(&self) -> &dyn Any;

    fn as_object(&self) -> &dyn CompoundObject<U>;
}

impl<U, T> CompoundAs<U> for T
where
    U: User,
    T: CompoundObject<U>,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_object(&self) -> &dyn CompoundObject<U> {
        self
    }
}

pub trait CompoundEq<U: User> {
    fn compound_eq(&self, other: &dyn CompoundObject<U>) -> bool;
}

impl<U: User, T: PartialEq> CompoundEq<U> for T
where
    U: User,
    T: PartialEq + CompoundObject<U>,
{
    fn compound_eq(&self, other: &dyn CompoundObject<U>) -> bool {
        match other.as_any().downcast_ref::<T>() {
            Some(other_object) => self.eq(other_object),
            None => false,
        }
    }
}

pub trait CompoundHash<U: User> {
    fn compound_hash(&self, state: &mut dyn Hasher);
}

impl<U, T> CompoundHash<U> for T
where
    U: User,
    T: Hash + CompoundObject<U> + ?Sized,
{
    fn compound_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }
}

impl<U: User> PartialEq for dyn CompoundObject<U> {
    fn eq(&self, other: &dyn CompoundObject<U>) -> bool {
        self.compound_eq(other)
    }
}

impl<U: User> Hash for dyn CompoundObject<U> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.compound_hash(state);
    }
}

impl<U, T> Upcast<U, Self> for T
where
    U: User,
    Self: CompoundObject<U> + Clone,
{
    #[inline]
    fn to_super<K: Borrow<Self>>(k: &K) -> Self {
        Clone::clone(k.borrow())
    }

    #[inline]
    fn into_super(self) -> Self {
        self
    }
}

impl<U, T> CompoundObject<U> for Option<T>
where
    U: User,
    T: CompoundObject<U> + CompoundWalkStar<U> + std::fmt::Debug + PartialEq + Hash,
{
    fn type_name(&self) -> &'static str {
        match self {
            Some(_) => "Some",
            None => "None",
        }
    }

    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject<U>> + 'a> {
        Box::new(self.iter().map(|x| x as &dyn CompoundObject<U>))
    }
}

impl<U, T> CompoundWalkStar<U> for Option<T>
where
    U: User,
    T: CompoundObject<U> + CompoundWalkStar<U> + std::fmt::Debug + PartialEq + Hash,
{
    fn compound_walk_star(&self, smap: &SMap<U>) -> Self {
        self.as_ref().map(|x| x.compound_walk_star(smap))
    }
}

impl<U, T> Upcast<U, LTerm<U>> for Option<T>
where
    U: User,
    T: CompoundObject<U> + CompoundWalkStar<U> + Clone + Hash + PartialEq,
{
    #[inline]
    fn to_super<K: Borrow<Self>>(k: &K) -> LTerm<U> {
        Into::into(Clone::clone(k.borrow()))
    }

    #[inline]
    fn into_super(self) -> LTerm<U> {
        Into::into(self)
    }
}

impl<U, T> Downcast<U> for Option<T>
where
    U: User,
    T: CompoundObject<U> + CompoundWalkStar<U> + PartialEq + Hash,
{
    type SubType = Self;

    #[inline]
    fn into_sub(self) -> Self::SubType {
        self
    }
}

impl<U, T> Into<LTerm<U>> for Option<T>
where
    U: User,
    T: CompoundObject<U> + Hash + PartialEq,
{
    fn into(self) -> LTerm<U> {
        match self {
            Some(x) => LTerm::from(Rc::new(x) as Rc<dyn CompoundObject<U>>),
            None => LTerm::empty_list(),
        }
    }
}

impl<U: User> CompoundTerm<U> for LTerm<U> {
    fn new_var(name: &'static str) -> LTerm<U> {
        LTerm::var(name)
    }

    fn new_wildcard() -> LTerm<U> {
        LTerm::any()
    }

    fn new_none() -> LTerm<U> {
        LTerm::empty_list()
    }
}

impl<U: User> CompoundObject<U> for LTerm<U> {
    fn type_name(&self) -> &'static str {
        "LTerm"
    }

    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject<U>> + 'a> {
        match self.as_ref() {
            LTermInner::Compound(object) => object.children(),
            _ => Box::new(std::iter::empty()),
        }
    }

    fn as_term(&self) -> Option<&LTerm<U>> {
        Some(self)
    }
}

impl<U: User> CompoundWalkStar<U> for LTerm<U> {
    fn compound_walk_star(&self, smap: &SMap<U>) -> Self {
        smap.walk_star(self)
    }
}

impl<U: User> Downcast<U> for LTerm<U> {
    type SubType = Self;

    #[inline]
    fn into_sub(self) -> Self::SubType {
        self
    }
}

impl<U: User> CompoundObject<U> for (LTerm<U>, LTerm<U>) {
    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject<U>> + 'a> {
        // TODO: use array into_iter when it becomes stable
        Box::new(IntoIterator::into_iter(vec![
            &self.0 as &dyn CompoundObject<U>,
            &self.1 as &dyn CompoundObject<U>,
        ]))
    }
}

impl<U: User> CompoundWalkStar<U> for (LTerm<U>, LTerm<U>) {
    fn compound_walk_star(&self, smap: &SMap<U>) -> Self {
        (smap.walk_star(&self.0), smap.walk_star(&self.1))
    }
}

impl<U> Into<LTerm<U>> for (LTerm<U>, LTerm<U>)
where
    U: User,
{
    fn into(self) -> LTerm<U> {
        LTerm::from(Rc::new(self) as Rc<dyn CompoundObject<U>>)
    }
}

impl<U> Upcast<U, LTerm<U>> for (LTerm<U>, LTerm<U>)
where
    U: User,
{
    #[inline]
    fn to_super<K: Borrow<Self>>(k: &K) -> LTerm<U> {
        Into::into(Clone::clone(k.borrow()))
    }

    #[inline]
    fn into_super(self) -> LTerm<U> {
        Into::into(self)
    }
}

impl<U: User> Downcast<U> for (LTerm<U>, LTerm<U>) {
    type SubType = Self;

    #[inline]
    fn into_sub(self) -> Self::SubType {
        self
    }
}
