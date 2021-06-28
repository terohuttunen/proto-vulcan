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

use crate::engine::Engine;
use crate::lterm::{LTerm, LTermInner};
use crate::state::SMap;
use crate::user::User;
use crate::{Downcast, Upcast};
use std::any::Any;
use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub trait CompoundTerm<U, E>
where
    U: User,
    E: Engine<U>,
    Self: CompoundObject<U, E> + Sized,
{
    fn new_var(name: &'static str) -> Self;

    fn new_wildcard() -> Self;

    fn new_none() -> Self;
}

pub trait CompoundObject<U, E>:
    CompoundHash<U, E> + CompoundEq<U, E> + CompoundAs<U, E> + WalkStar<U, E> + std::fmt::Debug
where
    U: User,
    E: Engine<U>,
{
    fn type_name(&self) -> &'static str {
        ""
    }

    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject<U, E>> + 'a>;

    fn as_term(&self) -> Option<&LTerm<U, E>> {
        None
    }

    fn is_term(&self) -> bool {
        match self.as_term() {
            Some(_) => true,
            None => false,
        }
    }
}

pub trait WalkStar<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn walk_star(&self, smap: &SMap<U, E>) -> LTerm<U, E>;
}

impl<U, E, T> WalkStar<U, E> for T
where
    U: User,
    E: Engine<U>,
    T: CompoundWalkStar<U, E> + Into<LTerm<U, E>>,
{
    fn walk_star(&self, smap: &SMap<U, E>) -> LTerm<U, E> {
        self.compound_walk_star(smap).into()
    }
}

pub trait CompoundWalkStar<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn compound_walk_star(&self, smap: &SMap<U, E>) -> Self;
}

pub trait CompoundAs<U, E>: Any
where
    U: User,
    E: Engine<U>,
{
    fn as_any(&self) -> &dyn Any;

    fn as_object(&self) -> &dyn CompoundObject<U, E>;
}

impl<U, E, T> CompoundAs<U, E> for T
where
    U: User,
    E: Engine<U>,
    T: CompoundObject<U, E>,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_object(&self) -> &dyn CompoundObject<U, E> {
        self
    }
}

pub trait CompoundEq<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn compound_eq(&self, other: &dyn CompoundObject<U, E>) -> bool;
}

impl<U, E, T: PartialEq> CompoundEq<U, E> for T
where
    U: User,
    E: Engine<U>,
    T: PartialEq + CompoundObject<U, E>,
{
    fn compound_eq(&self, other: &dyn CompoundObject<U, E>) -> bool {
        match other.as_any().downcast_ref::<T>() {
            Some(other_object) => self.eq(other_object),
            None => false,
        }
    }
}

pub trait CompoundHash<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn compound_hash(&self, state: &mut dyn Hasher);
}

impl<U, E, T> CompoundHash<U, E> for T
where
    U: User,
    E: Engine<U>,
    T: Hash + CompoundObject<U, E> + ?Sized,
{
    fn compound_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }
}

impl<U, E> PartialEq for dyn CompoundObject<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &dyn CompoundObject<U, E>) -> bool {
        self.compound_eq(other)
    }
}

impl<U, E> Hash for dyn CompoundObject<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.compound_hash(state);
    }
}

impl<U, E, T> Upcast<U, E, Self> for T
where
    U: User,
    E: Engine<U>,
    Self: CompoundObject<U, E> + Clone,
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

impl<U, E, T> CompoundObject<U, E> for Option<T>
where
    U: User,
    E: Engine<U>,
    T: CompoundObject<U, E> + CompoundWalkStar<U, E> + std::fmt::Debug + PartialEq + Hash,
{
    fn type_name(&self) -> &'static str {
        match self {
            Some(_) => "Some",
            None => "None",
        }
    }

    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject<U, E>> + 'a> {
        Box::new(self.iter().map(|x| x as &dyn CompoundObject<U, E>))
    }
}

impl<U, E, T> CompoundWalkStar<U, E> for Option<T>
where
    U: User,
    E: Engine<U>,
    T: CompoundObject<U, E> + CompoundWalkStar<U, E> + std::fmt::Debug + PartialEq + Hash,
{
    fn compound_walk_star(&self, smap: &SMap<U, E>) -> Self {
        self.as_ref().map(|x| x.compound_walk_star(smap))
    }
}

impl<U, E, T> Upcast<U, E, LTerm<U, E>> for Option<T>
where
    U: User,
    E: Engine<U>,
    T: CompoundObject<U, E> + CompoundWalkStar<U, E> + Clone + Hash + PartialEq,
{
    #[inline]
    fn to_super<K: Borrow<Self>>(k: &K) -> LTerm<U, E> {
        Into::into(Clone::clone(k.borrow()))
    }

    #[inline]
    fn into_super(self) -> LTerm<U, E> {
        Into::into(self)
    }
}

impl<U, E, T> Downcast<U, E> for Option<T>
where
    U: User,
    E: Engine<U>,
    T: CompoundObject<U, E> + CompoundWalkStar<U, E> + PartialEq + Hash,
{
    type SubType = Self;

    #[inline]
    fn into_sub(self) -> Self::SubType {
        self
    }
}

impl<U, E, T> Into<LTerm<U, E>> for Option<T>
where
    U: User,
    E: Engine<U>,
    T: CompoundObject<U, E> + Hash + PartialEq,
{
    fn into(self) -> LTerm<U, E> {
        match self {
            Some(x) => LTerm::from(Rc::new(x) as Rc<dyn CompoundObject<U, E>>),
            None => LTerm::empty_list(),
        }
    }
}

impl<U, E> CompoundTerm<U, E> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn new_var(name: &'static str) -> LTerm<U, E> {
        LTerm::var(name)
    }

    fn new_wildcard() -> LTerm<U, E> {
        LTerm::any()
    }

    fn new_none() -> LTerm<U, E> {
        LTerm::empty_list()
    }
}

impl<U, E> CompoundObject<U, E> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn type_name(&self) -> &'static str {
        "LTerm"
    }

    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject<U, E>> + 'a> {
        match self.as_ref() {
            LTermInner::Compound(object) => object.children(),
            _ => Box::new(std::iter::empty()),
        }
    }

    fn as_term(&self) -> Option<&LTerm<U, E>> {
        Some(self)
    }
}

impl<U, E> CompoundWalkStar<U, E> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn compound_walk_star(&self, smap: &SMap<U, E>) -> Self {
        smap.walk_star(self)
    }
}

impl<U, E> Downcast<U, E> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    type SubType = Self;

    #[inline]
    fn into_sub(self) -> Self::SubType {
        self
    }
}

impl<U, E> CompoundObject<U, E> for (LTerm<U, E>, LTerm<U, E>)
where
    U: User,
    E: Engine<U>,
{
    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject<U, E>> + 'a> {
        // TODO: use array into_iter when it becomes stable
        Box::new(IntoIterator::into_iter(vec![
            &self.0 as &dyn CompoundObject<U, E>,
            &self.1 as &dyn CompoundObject<U, E>,
        ]))
    }
}

impl<U, E> CompoundWalkStar<U, E> for (LTerm<U, E>, LTerm<U, E>)
where
    U: User,
    E: Engine<U>,
{
    fn compound_walk_star(&self, smap: &SMap<U, E>) -> Self {
        (smap.walk_star(&self.0), smap.walk_star(&self.1))
    }
}

impl<U, E> Into<LTerm<U, E>> for (LTerm<U, E>, LTerm<U, E>)
where
    U: User,
    E: Engine<U>,
{
    fn into(self) -> LTerm<U, E> {
        LTerm::from(Rc::new(self) as Rc<dyn CompoundObject<U, E>>)
    }
}

impl<U, E> Upcast<U, E, LTerm<U, E>> for (LTerm<U, E>, LTerm<U, E>)
where
    U: User,
    E: Engine<U>,
{
    #[inline]
    fn to_super<K: Borrow<Self>>(k: &K) -> LTerm<U, E> {
        Into::into(Clone::clone(k.borrow()))
    }

    #[inline]
    fn into_super(self) -> LTerm<U, E> {
        Into::into(self)
    }
}

impl<U, E> Downcast<U, E> for (LTerm<U, E>, LTerm<U, E>)
where
    U: User,
    E: Engine<U>,
{
    type SubType = Self;

    #[inline]
    fn into_sub(self) -> Self::SubType {
        self
    }
}
