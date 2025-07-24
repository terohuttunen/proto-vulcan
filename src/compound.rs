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

use crate::{Downcast, Upcast};
use std::any::Any;
use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub trait CompoundTerm
where
    Self: CompoundObject + Sized,
{
    fn new_var(name: &'static str) -> Self;

    fn new_wildcard() -> Self;

    fn new_none() -> Self;
}

pub trait CompoundObject:
    CompoundHash + CompoundEq + CompoundAs + WalkStar + std::fmt::Debug
{
    /// Get the type name for this compound object (non-static, can access environment)
    fn type_name(&self) -> String {
        "Unknown".to_string()
    }

    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject> + 'a>;

    fn as_term(&self) -> Option<&LTerm> {
        None
    }

    /// Check if this compound object is an enum variant
    fn is_enum_variant(&self) -> bool {
        false
    }

    /// Get the variant name for enum variants, None for other compound objects
    fn variant_name(&self) -> Option<String> {
        None
    }

    /// Get the variant index for enum variants, None for other compound objects
    /// This allows unification to distinguish between different enum variants
    /// before comparing their children (bodies)
    fn variant_index(&self) -> Option<usize> {
        None
    }

    /// Get the type registry index for compound objects that use the type registry
    /// This allows unification to distinguish between different enum types
    /// Returns None for compound objects that don't use the type registry
    fn type_registry_index(&self) -> Option<usize> {
        None
    }

    /// Get a string representation of this compound object for display purposes
    /// Default implementation uses type_name and variant_name
    fn display_string(&self) -> String {
        if let Some(variant_name) = self.variant_name() {
            format!("{}::{}", self.type_name(), variant_name)
        } else {
            self.type_name()
        }
    }

    fn is_term(&self) -> bool {
        match self.as_term() {
            Some(_) => true,
            None => false,
        }
    }
}

pub trait WalkStar {
    fn walk_star(&self, smap: &SMap) -> LTerm;
}

impl<T> WalkStar for T
where
    T: CompoundWalkStar + Into<LTerm>,
{
    fn walk_star(&self, smap: &SMap) -> LTerm {
        self.compound_walk_star(smap).into()
    }
}

pub trait CompoundWalkStar {
    fn compound_walk_star(&self, smap: &SMap) -> Self;
}

pub trait CompoundAs: Any {
    fn as_any(&self) -> &dyn Any;

    fn as_object(&self) -> &dyn CompoundObject;
}

impl<T> CompoundAs for T
where
    T: CompoundObject,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_object(&self) -> &dyn CompoundObject {
        self
    }
}

pub trait CompoundEq {
    fn compound_eq(&self, other: &dyn CompoundObject) -> bool;
}

impl<T: PartialEq> CompoundEq for T
where
    T: PartialEq + CompoundObject,
{
    fn compound_eq(&self, other: &dyn CompoundObject) -> bool {
        match other.as_any().downcast_ref::<T>() {
            Some(other_object) => self.eq(other_object),
            None => false,
        }
    }
}

pub trait CompoundHash {
    fn compound_hash(&self, state: &mut dyn Hasher);
}

impl<T> CompoundHash for T
where
    T: Hash + CompoundObject + ?Sized,
{
    fn compound_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }
}

impl PartialEq for dyn CompoundObject {
    fn eq(&self, other: &dyn CompoundObject) -> bool {
        self.compound_eq(other)
    }
}

impl Hash for dyn CompoundObject {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.compound_hash(state);
    }
}

impl<T> Upcast<Self> for T
where
    Self: CompoundObject + Clone,
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

impl<T> CompoundObject for Option<T>
where
    T: CompoundObject + CompoundWalkStar + std::fmt::Debug + PartialEq + Hash,
{
    fn type_name(&self) -> String {
        match self {
            Some(_) => "Some".to_string(),
            None => "None".to_string(),
        }
    }

    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject> + 'a> {
        Box::new(self.iter().map(|x| x as &dyn CompoundObject))
    }
}

impl<T> CompoundWalkStar for Option<T>
where
    T: CompoundObject + CompoundWalkStar + std::fmt::Debug + PartialEq + Hash,
{
    fn compound_walk_star(&self, smap: &SMap) -> Self {
        self.as_ref().map(|x| x.compound_walk_star(smap))
    }
}

impl<T> Upcast<LTerm> for Option<T>
where
    T: CompoundObject + CompoundWalkStar + Clone + Hash + PartialEq,
{
    #[inline]
    fn to_super<K: Borrow<Self>>(k: &K) -> LTerm {
        Into::into(Clone::clone(k.borrow()))
    }

    #[inline]
    fn into_super(self) -> LTerm {
        Into::into(self)
    }
}

impl<T> Downcast for Option<T>
where
    T: CompoundObject + CompoundWalkStar + PartialEq + Hash,
{
    type SubType = Self;

    #[inline]
    fn into_sub(self) -> Self::SubType {
        self
    }
}

impl<T> Into<LTerm> for Option<T>
where
    T: CompoundObject + Hash + PartialEq,
{
    fn into(self) -> LTerm {
        match self {
            Some(x) => LTerm::from(Rc::new(x) as Rc<dyn CompoundObject>),
            None => LTerm::empty_list(),
        }
    }
}

impl CompoundTerm for LTerm {
    fn new_var(name: &'static str) -> LTerm {
        LTerm::var(name)
    }

    fn new_wildcard() -> LTerm {
        LTerm::any()
    }

    fn new_none() -> LTerm {
        LTerm::empty_list()
    }
}

impl CompoundObject for LTerm {
    fn type_name(&self) -> String {
        "LTerm".to_string()
    }

    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject> + 'a> {
        match self.as_ref() {
            LTermInner::Compound(object) => object.children(),
            _ => Box::new(std::iter::empty()),
        }
    }

    fn as_term(&self) -> Option<&LTerm> {
        Some(self)
    }
}

impl CompoundWalkStar for LTerm {
    fn compound_walk_star(&self, smap: &SMap) -> Self {
        smap.walk_star(self)
    }
}

impl Downcast for LTerm {
    type SubType = Self;

    #[inline]
    fn into_sub(self) -> Self::SubType {
        self
    }
}

impl CompoundObject for (LTerm, LTerm) {
    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject> + 'a> {
        // TODO: use array into_iter when it becomes stable
        Box::new(IntoIterator::into_iter(vec![
            &self.0 as &dyn CompoundObject,
            &self.1 as &dyn CompoundObject,
        ]))
    }
}

impl CompoundWalkStar for (LTerm, LTerm) {
    fn compound_walk_star(&self, smap: &SMap) -> Self {
        (smap.walk_star(&self.0), smap.walk_star(&self.1))
    }
}

impl Into<LTerm> for (LTerm, LTerm) {
    fn into(self) -> LTerm {
        LTerm::from(Rc::new(self) as Rc<dyn CompoundObject>)
    }
}

impl Upcast<LTerm> for (LTerm, LTerm) {
    #[inline]
    fn to_super<K: Borrow<Self>>(k: &K) -> LTerm {
        Into::into(Clone::clone(k.borrow()))
    }

    #[inline]
    fn into_super(self) -> LTerm {
        Into::into(self)
    }
}

impl Downcast for (LTerm, LTerm) {
    type SubType = Self;

    #[inline]
    fn into_sub(self) -> Self::SubType {
        self
    }
}
