use crate::compound::CompoundObject;
use crate::user::{DefaultUser, User};
use crate::engine::{Engine, DefaultEngine};
use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::ops::{Index, IndexMut};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::vec::Vec;

pub use crate::lvalue::LValue;

static UNIQUE_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct VarID(usize);

impl VarID {
    pub fn new() -> VarID {
        let id = UNIQUE_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        VarID(id)
    }
}

impl fmt::Display for VarID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Logic Term.
#[derive(Derivative, Debug)]
#[derivative(Clone(bound="U: User"))]
pub enum LTermInner<U, E>
where
    U: User,
    E: Engine<U>,
{
    /// Literal value
    Val(LValue),

    /// Variable (uid, name)
    Var(VarID, &'static str),

    // User defined item
    User(<U as User>::UserTerm),

    // Empty list
    Empty,

    /// Non-empty list
    Cons(LTerm<U, E>, LTerm<U, E>),

    // Projection variable. A Projection variable will cause panic if it is tested for equality
    // or a hash is computed. To use in substitutions, it must be projected first to non-Projection
    // kind LTerm.
    Projection(LTerm<U, E>),

    // Compound object
    Compound(Rc<dyn CompoundObject<U, E>>),
}

#[derive(Derivative)]
#[derivative(Clone(bound="U: User"))]
pub struct LTerm<U = DefaultUser, E = DefaultEngine<U>>
where
    U: User,
    E: Engine<U>,
{
    inner: Rc<LTermInner<U, E>>,
}

impl<U, E> LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn ptr_eq(this: &LTerm<U, E>, other: &LTerm<U, E>) -> bool {
        Rc::ptr_eq(&this.inner, &other.inner)
    }

    pub fn var(name: &'static str) -> LTerm<U, E> {
        if name == "_" {
            panic!("Error: Invalid variable name. Name \"_\" is reserved for any-variables.")
        }

        LTerm {
            inner: Rc::new(LTermInner::Var(VarID::new(), name)),
        }
    }

    pub fn any() -> LTerm<U, E> {
        LTerm {
            inner: Rc::new(LTermInner::Var(VarID::new(), "_")),
        }
    }

    pub fn user(u: U::UserTerm) -> LTerm<U, E> {
        LTerm {
            inner: Rc::new(LTermInner::User(u)),
        }
    }

    /// Constructs an empty list
    ///
    pub fn empty_list() -> LTerm<U, E> {
        LTerm {
            inner: Rc::new(LTermInner::Empty),
        }
    }

    /// Constructs a LTerm list with a single element
    ///
    pub fn singleton(u: LTerm<U, E>) -> LTerm<U, E> {
        LTerm {
            inner: Rc::new(LTermInner::Cons(u, LTerm::empty_list())),
        }
    }

    pub fn projection(u: LTerm<U, E>) -> LTerm<U, E> {
        match u.as_ref() {
            LTermInner::Var(_, _) => LTerm {
                inner: Rc::new(LTermInner::Projection(u)),
            },
            _ => unreachable!(),
        }
    }

    /// Convert LTerm::Projection into non-Projection kind LTerm using the projection function `f`
    /// that is applied to the projection variable.
    pub fn project<F>(&self, f: F)
    where
        F: FnOnce(&LTerm<U, E>) -> LTerm<U, E>,
    {
        match self.as_ref() {
            LTermInner::Projection(p) => {
                let ptr: *const LTermInner<U, E> = self.inner.as_ref();
                let projected = f(p).into_inner();
                let _ = unsafe {
                    let mut_ptr = ptr as *mut LTermInner<U, E>;
                    std::ptr::replace(mut_ptr, projected.as_ref().clone())
                };
            }
            _ => panic!("Cannot project non-Projection LTerm."),
        }
    }

    pub fn into_inner(self) -> Rc<LTermInner<U, E>> {
        self.inner
    }

    /// Construct a list cell
    pub fn cons(head: LTerm<U, E>, tail: LTerm<U, E>) -> LTerm<U, E> {
        LTerm {
            inner: Rc::new(LTermInner::Cons(head, tail)),
        }
    }

    pub fn from_vec(l: Vec<LTerm<U, E>>) -> LTerm<U, E> {
        if l.is_empty() {
            LTerm::empty_list()
        } else {
            let mut c = LTerm::empty_list();
            for t in l.into_iter().rev() {
                c = LTerm::cons(t, c);
            }
            c
        }
    }

    pub fn from_array(a: &[LTerm<U, E>]) -> LTerm<U, E> {
        if a.is_empty() {
            LTerm::empty_list()
        } else {
            let mut c = LTerm::empty_list();
            for t in a.to_vec().into_iter().rev() {
                c = LTerm::cons(t, c);
            }
            c
        }
    }

    pub fn improper_from_vec(mut h: Vec<LTerm<U, E>>) -> LTerm<U, E> {
        if h.is_empty() {
            panic!("Improper list must have at least one element");
        } else {
            let mut c = h.pop().unwrap();
            for s in h.into_iter().rev() {
                c = LTerm::cons(s, c);
            }
            c
        }
    }

    pub fn improper_from_array(h: &[LTerm<U, E>]) -> LTerm<U, E> {
        let mut h = h.to_vec();
        if h.is_empty() {
            panic!("Improper list must have at least one element");
        } else {
            let mut c = h.pop().unwrap();
            for s in h.into_iter().rev() {
                c = LTerm::cons(s, c);
            }
            c
        }
    }

    pub fn contains<T: Borrow<LTerm<U, E>>>(&self, v: &T) -> bool {
        let v = v.borrow();
        self.iter().any(|u| u == v)
    }

    pub fn is_val(&self) -> bool {
        match self.as_ref() {
            LTermInner::Val(_) => true,
            _ => false,
        }
    }

    pub fn is_bool(&self) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Bool(_)) => true,
            _ => false,
        }
    }

    pub fn get_bool(&self) -> Option<bool> {
        match self.as_ref() {
            LTermInner::Val(LValue::Bool(u)) => Some(*u),
            _ => None,
        }
    }

    pub fn is_number(&self) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Number(_)) => true,
            _ => false,
        }
    }

    pub fn get_number(&self) -> Option<isize> {
        match self.as_ref() {
            LTermInner::Val(LValue::Number(u)) => Some(*u),
            _ => None,
        }
    }

    pub fn is_var(&self) -> bool {
        match self.as_ref() {
            LTermInner::<U, E>::Var(_, _) => true,
            _ => false,
        }
    }

    pub fn is_any(&self) -> bool {
        match self.as_ref() {
            LTermInner::Var(_, "_") => true,
            _ => false,
        }
    }

    pub fn is_user(&self) -> bool {
        match self.as_ref() {
            LTermInner::User(_) => true,
            _ => false,
        }
    }

    pub fn get_user(&self) -> Option<&U::UserTerm> {
        match self.as_ref() {
            LTermInner::User(u) => Some(u),
            _ => None,
        }
    }

    pub fn is_projection(&self) -> bool {
        match self.as_ref() {
            LTermInner::Projection(_) => true,
            _ => false,
        }
    }

    pub fn get_projection(&self) -> Option<&LTerm<U, E>> {
        match self.as_ref() {
            LTermInner::Projection(p) => Some(p),
            _ => None,
        }
    }

    pub fn is_list(&self) -> bool {
        match self.as_ref() {
            LTermInner::Empty => true,
            LTermInner::Cons(_, _) => true,
            _ => false,
        }
    }

    pub fn is_improper(&self) -> bool {
        match self.as_ref() {
            LTermInner::Empty => false,
            LTermInner::Cons(_, tail) => {
                if tail.is_empty() {
                    false
                } else {
                    if tail.is_list() {
                        tail.is_improper()
                    } else {
                        true
                    }
                }
            }
            _ => false,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self.as_ref() {
            LTermInner::Empty => true,
            _ => false,
        }
    }

    pub fn is_non_empty_list(&self) -> bool {
        match self.as_ref() {
            LTermInner::Cons(_, _) => true,
            _ => false,
        }
    }

    pub fn head(&self) -> Option<&LTerm<U, E>> {
        match self.as_ref() {
            LTermInner::Cons(head, _) => Some(head),
            _ => None,
        }
    }

    pub fn tail(&self) -> Option<&LTerm<U, E>> {
        match self.as_ref() {
            LTermInner::Cons(_, tail) => Some(tail),
            _ => None,
        }
    }

    pub fn head_mut(&mut self) -> Option<&mut LTerm<U, E>> {
        match self.as_mut() {
            LTermInner::Cons(head, _) => Some(head),
            _ => None,
        }
    }

    pub fn tail_mut(&mut self) -> Option<&mut LTerm<U, E>> {
        match self.as_mut() {
            LTermInner::Cons(_, tail) => Some(tail),
            _ => None,
        }
    }

    pub fn iter(&self) -> LTermIter<'_, U, E> {
        LTermIter::new(self)
    }

    pub fn iter_mut(&mut self) -> LTermIterMut<'_, U, E> {
        LTermIterMut::new(self)
    }

    /// Recursively find all `any` variables referenced by the LTerm.
    pub fn anyvars(self: &LTerm<U, E>) -> Vec<LTerm<U, E>> {
        match self.as_ref() {
            LTermInner::Cons(head, tail) => {
                let mut vars = head.anyvars();
                for t in tail.iter() {
                    let tvars = t.anyvars();
                    vars.extend(tvars);
                }
                vars
            }
            _ => {
                if self.is_any() {
                    vec![self.clone()]
                } else {
                    vec![]
                }
            }
        }
    }
}

impl<U, E> From<Rc<dyn CompoundObject<U, E>>> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn from(u: Rc<dyn CompoundObject<U, E>>) -> LTerm<U, E> {
        LTerm::from(LTermInner::Compound(u))
    }
}

impl<U, E> From<&LTerm<U, E>> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn from(reference: &LTerm<U, E>) -> LTerm<U, E> {
        reference.clone()
    }
}

impl<U, E> From<LTermInner<U, E>> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn from(inner: LTermInner<U, E>) -> LTerm<U, E> {
        LTerm {
            inner: Rc::new(inner),
        }
    }
}

impl<U, E> From<isize> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn from(u: isize) -> LTerm<U, E> {
        LTerm::from(LTermInner::Val(LValue::Number(u)))
    }
}

impl<U, E> From<bool> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn from(u: bool) -> LTerm<U, E> {
        LTerm::from(LTermInner::Val(LValue::Bool(u)))
    }
}

impl<U, E> From<&str> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn from(u: &str) -> LTerm<U, E> {
        LTerm::from(LTermInner::Val(LValue::String(String::from(u))))
    }
}

impl<U, E> From<String> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn from(u: String) -> LTerm<U, E> {
        LTerm::from(LTermInner::Val(LValue::String(u)))
    }
}

impl<U, E> From<char> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn from(u: char) -> LTerm<U, E> {
        LTerm::from(LTermInner::Val(LValue::Char(u)))
    }
}

impl<U, E> AsRef<LTermInner<U, E>> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn as_ref(&self) -> &LTermInner<U, E> {
        &self.inner
    }
}

impl<U, E> AsRef<LTerm<U, E>> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn as_ref(&self) -> &LTerm<U, E> {
        self
    }
}

impl<U, E> AsMut<LTermInner<U, E>> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn as_mut(&mut self) -> &mut LTermInner<U, E> {
        Rc::make_mut(&mut self.inner)
    }
}

impl<U, E> fmt::Debug for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_ref() {
            LTermInner::Val(val) => write!(f, "{:?}", val),
            LTermInner::Var(uid, name) => write!(f, "Var({:?}, {:?})", uid, name),
            LTermInner::User(user) => write!(f, "User({:?})", user),
            LTermInner::Projection(p) => write!(f, "Projection({:?})", p),
            LTermInner::Empty => write!(f, "Empty"),
            LTermInner::Cons(head, tail) => write!(f, "({:?}, {:?})", head, tail),
            LTermInner::Compound(cf) => write!(f, "{:?}", cf),
        }
    }
}

impl<U, E> fmt::Display for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.as_ref() {
            LTermInner::Val(val) => write!(f, "{}", val),
            LTermInner::Var(uid, name) => {
                if self.is_any() {
                    write!(f, "{}.{}", name, uid)
                } else {
                    write!(f, "{}", name)
                }
            }
            LTermInner::User(user) => write!(f, "User({:?})", user),
            LTermInner::Projection(p) => write!(f, "Projection({})", p),
            LTermInner::Empty => write!(f, "[]"),
            LTermInner::Cons(_, _) => {
                if self.is_improper() {
                    let len = self.iter().count();
                    write!(f, "[")?;
                    for (count, v) in self.iter().enumerate() {
                        if count == 0 {
                            ()
                        } else if count > 0 && count < len - 1 {
                            write!(f, ", ")?;
                        } else {
                            write!(f, " | ")?;
                        }
                        write!(f, "{}", v)?;
                    }
                    write!(f, "]")
                } else {
                    write!(f, "[")?;
                    for (count, v) in self.iter().enumerate() {
                        if count != 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", v)?;
                    }
                    write!(f, "]")
                }
            }
            LTermInner::Compound(compound_term) => write!(f, "{:?}", compound_term),
        }
    }
}

impl<U, E> Hash for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.as_ref() {
            LTermInner::Val(val) => val.hash(state),
            LTermInner::Var(uid, _) => uid.hash(state),
            LTermInner::User(user) => user.hash(state),
            LTermInner::Projection(_) => panic!("Cannot compute hash for LTerm::Projection."),
            LTermInner::Empty => ().hash(state),
            LTermInner::Cons(head, tail) => {
                head.hash(state);
                tail.hash(state);
            }
            LTermInner::Compound(cf) => cf.compound_hash(state),
        }
    }
}

impl<U, E> PartialEq<LTerm<U, E>> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &Self) -> bool {
        match (self.as_ref(), other.as_ref()) {
            (LTermInner::Var(self_uid, _), LTermInner::Var(other_uid, _)) => self_uid == other_uid,
            (LTermInner::Val(self_val), LTermInner::Val(other_val)) => self_val == other_val,
            (LTermInner::User(self_user), LTermInner::User(other_user)) => self_user == other_user,
            (LTermInner::Projection(_), _) => panic!("Cannot compare LTerm::Projection."),
            (_, LTermInner::Projection(_)) => panic!("Cannot compare LTerm::Projection."),
            (LTermInner::Empty, LTermInner::Empty) => true,
            (LTermInner::Cons(self_head, self_tail), LTermInner::Cons(other_head, other_tail)) => {
                (self_head == other_head) & (self_tail == other_tail)
            }
            (LTermInner::Compound(self_cf), LTermInner::Compound(other_cf)) => {
                self_cf.compound_eq(other_cf.as_object())
            }
            _ => false,
        }
    }
}

impl<U, E> PartialEq<LValue> for LTerm<U, E>
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

impl<U, E> PartialEq<LTerm<U, E>> for LValue
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LTerm<U, E>) -> bool {
        match other.as_ref() {
            LTermInner::Val(v) => v == self,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<bool> for LTerm<U, E>
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

impl<U, E> PartialEq<LTerm<U, E>> for bool
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LTerm<U, E>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Bool(x)) => x == self,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<isize> for LTerm<U, E>
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

impl<U, E> PartialEq<LTerm<U, E>> for isize
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LTerm<U, E>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Number(x)) => x == self,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<char> for LTerm<U, E>
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

impl<U, E> PartialEq<LTerm<U, E>> for char
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LTerm<U, E>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Char(x)) => x == self,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<String> for LTerm<U, E>
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

impl<U, E> PartialEq<LTerm<U, E>> for String
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LTerm<U, E>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<str> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &str) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<LTerm<U, E>> for str
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LTerm<U, E>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl<U, E> PartialEq<&str> for LTerm<U, E>
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

impl<U, E> PartialEq<LTerm<U, E>> for &str
where
    U: User,
    E: Engine<U>,
{
    fn eq(&self, other: &LTerm<U, E>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl<U, E> Eq for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{}

impl<U, E> Default for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn default() -> Self {
        LTerm::from(LTermInner::Empty)
    }
}

impl<U, E> FromIterator<LTerm<U, E>> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn from_iter<T: IntoIterator<Item = LTerm<U, E>>>(iter: T) -> Self {
        let mut list_head = LTerm::empty_list();
        let mut list_tail = &mut list_head;
        for elem in iter {
            let _ = std::mem::replace(
                list_tail.as_mut(),
                LTermInner::Cons(elem, LTerm::empty_list()),
            );
            list_tail = list_tail.tail_mut().unwrap();
        }
        list_head
    }
}

impl<U, E> Extend<LTerm<U, E>> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn extend<T: IntoIterator<Item = LTerm<U, E>>>(&mut self, coll: T) {
        if !self.is_list() {
            panic!("Only list type (Empty or Cons) LTerms can be extended.");
        }

        // Find tail of the list
        let mut tail = self;
        loop {
            if tail.is_empty() {
                break;
            } else {
                tail = tail.tail_mut().unwrap();
            }
        }

        // Swap in extension as new tail.
        let mut extension: LTerm<U, E> = coll.into_iter().collect();
        std::mem::swap(tail.as_mut(), extension.as_mut());
    }
}

#[derive(Clone, Debug)]
pub struct LTermIter<'a, U, E>
where
    U: User,
    E: Engine<U>,
{
    maybe_next: Option<&'a LTerm<U, E>>,
}

impl<'a, U, E> LTermIter<'a, U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(u: &'a LTerm<U, E>) -> LTermIter<'a, U, E> {
        LTermIter {
            maybe_next: Some(u),
        }
    }
}

impl<'a, U, E> Iterator for LTermIter<'a, U, E>
where
    U: User,
    E: Engine<U>,
{
    type Item = &'a LTerm<U, E>;

    fn next(&mut self) -> Option<Self::Item> {
        // Replace maybe_next in iterator with its tail and return head
        match self.maybe_next.map(|x| x.as_ref()) {
            Some(LTermInner::Cons(head, tail)) => {
                if tail.is_empty() {
                    // The iterator has finished the list after this one
                    self.maybe_next = None;
                } else {
                    let _ = self.maybe_next.replace(tail);
                }

                Some(head)
            }
            Some(LTermInner::Empty) => {
                self.maybe_next = None;
                None
            }
            Some(_) => {
                // If the list is improper, it ends in non-cons term.
                self.maybe_next.take()
            }
            _ => None, // Iterator is finished
        }
    }
}

impl<'a, U, E> std::iter::FusedIterator for LTermIter<'a, U, E>
where
    U: User,
    E: Engine<U>,
{}

impl<'a, U, E> IntoIterator for &'a LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    type Item = &'a LTerm<U, E>;
    type IntoIter = LTermIter<'a, U, E>;

    fn into_iter(self) -> LTermIter<'a, U, E> {
        LTermIter::new(self)
    }
}

#[derive(Debug)]
pub struct LTermIterMut<'a, U, E>
where
    U: User,
    E: Engine<U>,
{
    maybe_next: Option<&'a mut LTerm<U, E>>,
}

impl<'a, U, E> LTermIterMut<'a, U, E>
where
    U: User,
    E: Engine<U>,
{
    pub fn new(u: &'a mut LTerm<U, E>) -> LTermIterMut<'a, U, E> {
        LTermIterMut {
            maybe_next: Some(u),
        }
    }
}

impl<'a, U, E> Iterator for LTermIterMut<'a, U, E>
where
    U: User,
    E: Engine<U>,
{
    type Item = &'a mut LTerm<U, E>;

    fn next(&mut self) -> Option<Self::Item> {
        // Replace maybe_next in iterator with its tail and return head
        match self.maybe_next.take().map(|x| x.as_mut()) {
            Some(LTermInner::Cons(head, tail)) => {
                if tail.is_empty() {
                    // The iterator has finished the list after this one
                    self.maybe_next = None;
                } else {
                    let _ = self.maybe_next.replace(tail);
                }

                Some(head)
            }
            Some(LTermInner::Empty) => {
                self.maybe_next = None;
                None
            }
            Some(_) => {
                // If the list is improper, it ends in non-cons term.
                self.maybe_next.take()
            }
            _ => None, // Iterator is finished
        }
    }
}

impl<'a, U, E> IntoIterator for &'a mut LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    type Item = &'a mut LTerm<U, E>;
    type IntoIter = LTermIterMut<'a, U, E>;

    fn into_iter(self) -> LTermIterMut<'a, U, E> {
        LTermIterMut::new(self)
    }
}

impl<'a, U, E> std::iter::FusedIterator for LTermIterMut<'a, U, E>
where
    U: User,
    E: Engine<U>,
{}

impl<U, E> Index<usize> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    type Output = LTerm<U, E>;

    fn index(&self, index: usize) -> &Self::Output {
        self.iter().nth(index).unwrap()
    }
}

impl<U, E> IndexMut<usize> for LTerm<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.iter_mut().nth(index).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_lterm_var_1() {
        let mut u = LTerm::<DefaultUser>::var("x");
        assert!(u.is_var());
        assert!(!u.is_val());
        assert!(!u.is_bool());
        assert!(!u.is_list());
        assert!(!u.is_empty());
        assert!(!u.is_non_empty_list());
        assert!(!u.is_user());
        assert!(!u.is_projection());
        assert!(u.tail().is_none());
        assert!(u.head().is_none());
        assert!(u.tail_mut().is_none());
        assert!(u.head_mut().is_none());
    }

    #[test]
    #[should_panic]
    fn test_lterm_var_2() {
        let _ = LTerm::<DefaultUser>::var("_");
    }

    #[test]
    fn test_lterm_val_1() {
        let mut u: LTerm<DefaultUser, DefaultEngine<DefaultUser>> = lterm!(1);
        assert!(u.is_val());
        assert!(!u.is_var());
        assert!(!u.is_bool());
        assert!(!u.is_list());
        assert!(!u.is_empty());
        assert!(!u.is_non_empty_list());
        assert!(!u.is_user());
        assert!(!u.is_projection());
        assert!(u.tail().is_none());
        assert!(u.head().is_none());
        assert!(u.tail_mut().is_none());
        assert!(u.head_mut().is_none());
    }

    #[test]
    fn test_lterm_val_2() {
        let mut u: LTerm<DefaultUser> = lterm!(true);
        assert!(u.is_val());
        assert!(!u.is_var());
        assert!(u.is_bool());
        assert!(!u.is_list());
        assert!(!u.is_empty());
        assert!(!u.is_non_empty_list());
        assert!(!u.is_user());
        assert!(!u.is_projection());
        assert!(u.tail().is_none());
        assert!(u.head().is_none());
        assert!(u.tail_mut().is_none());
        assert!(u.head_mut().is_none());
    }

    #[test]
    fn test_lterm_iter_1() {
        let u: LTerm<DefaultUser> = lterm!([]);
        let mut iter = u.iter();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_2() {
        let u: LTerm<DefaultUser> = lterm!([1]);
        let mut iter = u.iter();
        assert_eq!(iter.next().unwrap(), &1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_3() {
        let u: LTerm<DefaultUser> = lterm!([1, 2, 3]);
        let mut iter = u.iter();
        assert_eq!(iter.next().unwrap(), &1);
        assert_eq!(iter.next().unwrap(), &2);
        assert_eq!(iter.next().unwrap(), &3);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_4() {
        let u: LTerm<DefaultUser> = lterm!([1, 2 | 3]);
        let mut iter = u.iter();
        assert_eq!(iter.next().unwrap(), &1);
        assert_eq!(iter.next().unwrap(), &2);
        assert_eq!(iter.next().unwrap(), &3);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_5() {
        let u: LTerm<DefaultUser> = lterm!([1, 2, 3]);
        let mut iter = IntoIterator::into_iter(&u);
        assert_eq!(iter.next().unwrap(), &1);
        assert_eq!(iter.next().unwrap(), &2);
        assert_eq!(iter.next().unwrap(), &3);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_mut_1() {
        let mut u: LTerm<DefaultUser> = lterm!([1, 2, 3]);
        let iter = u.iter_mut();
        for x in iter {
            *x = lterm!(4);
        }
        let mut iter = u.iter();
        assert_eq!(iter.next().unwrap(), &4);
        assert_eq!(iter.next().unwrap(), &4);
        assert_eq!(iter.next().unwrap(), &4);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_mut_2() {
        let mut u: LTerm<DefaultUser> = lterm!([1, 2, 3]);
        for term in &mut u {
            *term = lterm!(5);
        }
        let mut iter = u.iter();
        assert_eq!(iter.next().unwrap(), &5);
        assert_eq!(iter.next().unwrap(), &5);
        assert_eq!(iter.next().unwrap(), &5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_from_iter_1() {
        let v: Vec<LTerm<DefaultUser>> = vec![lterm!(1), lterm!(2), lterm!(3)];
        let u: LTerm<DefaultUser> = LTerm::from_iter(v);
        assert!(u == lterm!([1, 2, 3]));
    }

    #[test]
    fn test_lterm_extend_1() {
        let v = vec![lterm!(1), lterm!(2), lterm!(3)];
        let mut u: LTerm<DefaultUser> = lterm!([]);
        u.extend(v);
        assert!(u == lterm!([1, 2, 3]));
    }

    #[test]
    fn test_lterm_extend_2() {
        let v = vec![lterm!(1), lterm!(2), lterm!(3)];
        let mut u: LTerm<DefaultUser> = lterm!([4, 5, 6]);
        u.extend(v);
        assert!(u == lterm!([4, 5, 6, 1, 2, 3]));
    }

    #[test]
    fn test_lterm_eq_1() {
        // LTerm vs. LTerm
        assert_eq!(lterm!(1) as LTerm<DefaultUser>, lterm!(1));
        assert_eq!(lterm!(true) as LTerm<DefaultUser>, lterm!(true));
        assert_eq!(lterm!("foo") as LTerm<DefaultUser>, lterm!("foo"));
        assert_eq!(lterm!('a') as LTerm<DefaultUser>, lterm!('a'));
        assert_eq!(lterm!([1, 2, 3]) as LTerm<DefaultUser>, lterm!([1, 2, 3]));
        assert_ne!(lterm!(1) as LTerm<DefaultUser>, lterm!(2));
        assert_ne!(lterm!(1) as LTerm<DefaultUser>, lterm!(true));
        assert_ne!(lterm!(1) as LTerm<DefaultUser>, lterm!('a'));
        assert_ne!(lterm!(1) as LTerm<DefaultUser>, lterm!([]));
        assert_ne!(lterm!(1) as LTerm<DefaultUser>, lterm!([1]));
        assert_ne!(lterm!(1) as LTerm<DefaultUser>, lterm!("true"));
        let u: LTerm<DefaultUser> = LTerm::var("x");
        let v: LTerm<DefaultUser> = LTerm::var("x");
        assert_eq!(u, u);
        assert_ne!(u, v);
    }

    #[test]
    fn test_lterm_eq_2() {
        // LTerm vs. Rust constant
        assert_eq!(lterm!(1) as LTerm<DefaultUser>, 1);
        assert_ne!(lterm!(1) as LTerm<DefaultUser>, 2);
        assert_ne!(lterm!(1) as LTerm<DefaultUser>, true);
        assert_eq!(1, lterm!(1) as LTerm<DefaultUser>);
        assert_ne!(2, lterm!(1) as LTerm<DefaultUser>);
        assert_ne!(true, lterm!(1) as LTerm<DefaultUser>);
        assert_eq!(lterm!("proto-vulcan") as LTerm<DefaultUser>, "proto-vulcan");
        assert_ne!(lterm!(["proto-vulcan"]) as LTerm<DefaultUser>, "proto-vulcan");
        assert_eq!("proto-vulcan", lterm!("proto-vulcan") as LTerm<DefaultUser>);
        assert_ne!("proto-vulcan", lterm!(["proto-vulcan"]) as LTerm<DefaultUser>);
        assert_eq!(
            lterm!("proto-vulcan") as LTerm<DefaultUser>,
            "proto-vulcan"[0..]
        );
        assert_ne!(
            lterm!(["proto-vulcan"]) as LTerm<DefaultUser>,
            "proto-vulcan"[0..]
        );
        assert_eq!(
            "proto-vulcan"[0..],
            lterm!("proto-vulcan") as LTerm<DefaultUser>
        );
        assert_ne!(
            "proto-vulcan"[0..],
            lterm!(["proto-vulcan"]) as LTerm<DefaultUser>
        );
        assert_eq!(
            lterm!("proto-vulcan") as LTerm<DefaultUser>,
            String::from("proto-vulcan")
        );
        assert_ne!(
            lterm!(["proto-vulcan"]) as LTerm<DefaultUser>,
            String::from("proto-vulcan")
        );
        assert_eq!(
            String::from("proto-vulcan"),
            lterm!("proto-vulcan") as LTerm<DefaultUser>
        );
        assert_ne!(
            String::from("proto-vulcan"),
            lterm!(["proto-vulcan"]) as LTerm<DefaultUser>
        );
        assert_eq!(lterm!('a') as LTerm<DefaultUser>, 'a');
        assert_ne!('b', lterm!('a') as LTerm<DefaultUser>);
        assert_ne!(lterm!(['a']) as LTerm<DefaultUser>, 'a');
        assert_ne!('a', lterm!(['a']) as LTerm<DefaultUser>);
        assert_ne!(lterm!(1) as LTerm<DefaultUser>, lterm!([1]));
        assert_ne!(lterm!([1]), lterm!(1) as LTerm<DefaultUser>);
    }

    #[test]
    fn test_lterm_eq_3() {
        // LTerm vs. LValue
        assert_eq!(lterm!(1) as LTerm<DefaultUser>, LValue::from(1));
        assert_ne!(lterm!(1) as LTerm<DefaultUser>, LValue::from(2));
        assert_eq!(LValue::from(1), lterm!(1) as LTerm<DefaultUser>);
        assert_ne!(LValue::from(2), lterm!(1) as LTerm<DefaultUser>);
        assert_ne!(LValue::from(1), lterm!([1]) as LTerm<DefaultUser>);
        assert_ne!(lterm!([1]) as LTerm<DefaultUser>, LValue::from(1));
    }

    #[test]
    #[should_panic]
    fn test_lterm_projection_1() {
        // Comparison with projection panics
        let u: LTerm<DefaultUser> = LTerm::var("x");
        let v = LTerm::projection(u.clone());
        assert_eq!(u, v);
    }

    #[test]
    #[should_panic]
    fn test_lterm_projection_2() {
        // Comparison with projection panics
        let u: LTerm<DefaultUser> = LTerm::var("x");
        let v = LTerm::projection(u.clone());
        assert_eq!(v, u);
    }

    #[test]
    #[should_panic]
    fn test_lterm_projection_3() {
        // Hash of projection panics
        let mut t = HashMap::new();
        let u: LTerm<DefaultUser> = LTerm::var("x");
        let v = LTerm::projection(u.clone());
        t.insert(v, lterm!(1) as LTerm<DefaultUser>);
    }

    #[test]
    fn test_lterm_index_1() {
        let u: LTerm<DefaultUser> = lterm!([1, [2], false]);
        assert_eq!(u[0], 1);
        assert_eq!(u[1], lterm!([2]));
        assert_eq!(u[2], false);
    }

    #[test]
    fn test_lterm_index_mut_1() {
        let mut u: LTerm<DefaultUser> = lterm!([0, 0, 0]);
        u[0] = lterm!(1);
        u[1] = lterm!([2]);
        u[2] = lterm!(false);
        assert_eq!(u[0], 1);
        assert_eq!(u[1], lterm!([2]));
        assert_eq!(u[2], false);
    }

    #[test]
    fn test_lterm_display() {
        assert_eq!(format!("{}", lterm!(1234) as LTerm<DefaultUser>), "1234");
        assert_eq!(format!("{}", lterm!(-1234) as LTerm<DefaultUser>), "-1234");
        assert_eq!(format!("{}", lterm!(true) as LTerm<DefaultUser>), "true");
        assert_eq!(format!("{}", lterm!(false) as LTerm<DefaultUser>), "false");
        assert_eq!(format!("{}", LTerm::var("x") as LTerm<DefaultUser>), "x");
        assert_eq!(format!("{}", lterm!([]) as LTerm<DefaultUser>), "[]");
        assert_eq!(
            format!("{}", lterm!([1, [2], true, 'a']) as LTerm<DefaultUser>),
            "[1, [2], true, 'a']"
        );
        assert_eq!(
            format!("{}", lterm!([1, 2 | 3]) as LTerm<DefaultUser>),
            "[1, 2 | 3]"
        );
        let u = LTerm::var("x");
        assert_eq!(
            format!("{}", LTerm::projection(u) as LTerm<DefaultUser>),
            "Projection(x)"
        );
    }
}
