use crate::user::{EmptyUser, User};
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

/// Logic Term
#[derive(Clone, Debug)]
pub enum LTermInner<U = EmptyUser>
where
    U: User,
{
    /// Literal value
    Val(LValue),

    /// Variable (uid, name)
    Var(VarID, &'static str),

    // Unifiable user defined item. PartialEq and Hash are derived from the Rc pointer.
    User(<U as User>::UserTerm),

    // Empty list
    Empty,

    /// Non-empty list
    Cons(LTerm<U>, LTerm<U>),

    // Projection variable. A Projection variable will cause panic if it is tested for equality
    // or a hash is computed. To use in substitutions, it must be projected first to non-Projection
    // kind LTerm.
    Projection(LTerm<U>),
}

#[derive(Clone)]
pub struct LTerm<U = EmptyUser>
where
    U: User,
{
    inner: Rc<LTermInner<U>>,
}

impl<U: User> LTerm<U> {
    pub fn ptr_eq(this: &LTerm<U>, other: &LTerm<U>) -> bool {
        Rc::ptr_eq(&this.inner, &other.inner)
    }

    pub fn var(name: &'static str) -> LTerm<U> {
        if name == "_" {
            panic!("Error: Invalid variable name. Name \"_\" is reserved for any-variables.")
        }

        LTerm {
            inner: Rc::new(LTermInner::Var(VarID::new(), name)),
        }
    }

    pub fn any() -> LTerm<U> {
        LTerm {
            inner: Rc::new(LTermInner::Var(VarID::new(), "_")),
        }
    }

    pub fn user(u: U::UserTerm) -> LTerm<U> {
        LTerm {
            inner: Rc::new(LTermInner::User(u)),
        }
    }

    /// Constructs an empty list
    ///
    pub fn empty_list() -> LTerm<U> {
        LTerm {
            inner: Rc::new(LTermInner::Empty),
        }
    }

    /// Constructs a LTerm list with a single element
    ///
    pub fn singleton(u: LTerm<U>) -> LTerm<U> {
        LTerm {
            inner: Rc::new(LTermInner::Cons(u, LTerm::empty_list())),
        }
    }

    pub fn projection(u: LTerm<U>) -> LTerm<U> {
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
        F: FnOnce(&LTerm<U>) -> LTerm<U>,
    {
        match self.as_ref() {
            LTermInner::Projection(p) => {
                let ptr: *const LTermInner<U> = self.inner.as_ref();
                let projected = f(p).into_inner();
                let _ = unsafe {
                    let mut_ptr = ptr as *mut LTermInner<U>;
                    std::ptr::replace(mut_ptr, projected.as_ref().clone())
                };
            }
            _ => panic!("Cannot project non-Projection LTerm."),
        }
    }

    pub fn into_inner(self) -> Rc<LTermInner<U>> {
        self.inner
    }

    /// Construct a list cell
    pub fn cons(head: LTerm<U>, tail: LTerm<U>) -> LTerm<U> {
        LTerm {
            inner: Rc::new(LTermInner::Cons(head, tail)),
        }
    }

    pub fn from_vec(l: Vec<LTerm<U>>) -> LTerm<U> {
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

    pub fn from_array(a: &[LTerm<U>]) -> LTerm<U> {
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

    pub fn improper_from_vec(mut h: Vec<LTerm<U>>) -> LTerm<U> {
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

    pub fn improper_from_array(h: &[LTerm<U>]) -> LTerm<U> {
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

    pub fn contains<T: Borrow<LTerm<U>>>(&self, v: &T) -> bool {
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
            LTermInner::<U>::Var(_, _) => true,
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

    pub fn get_projection(&self) -> Option<&LTerm<U>> {
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

    pub fn head(&self) -> Option<&LTerm<U>> {
        match self.as_ref() {
            LTermInner::Cons(head, _) => Some(head),
            _ => None,
        }
    }

    pub fn tail(&self) -> Option<&LTerm<U>> {
        match self.as_ref() {
            LTermInner::Cons(_, tail) => Some(tail),
            _ => None,
        }
    }

    pub fn head_mut(&mut self) -> Option<&mut LTerm<U>> {
        match self.as_mut() {
            LTermInner::Cons(head, _) => Some(head),
            _ => None,
        }
    }

    pub fn tail_mut(&mut self) -> Option<&mut LTerm<U>> {
        match self.as_mut() {
            LTermInner::Cons(_, tail) => Some(tail),
            _ => None,
        }
    }

    pub fn iter(&self) -> LTermIter<'_, U> {
        LTermIter::new(self)
    }

    pub fn iter_mut(&mut self) -> LTermIterMut<'_, U> {
        LTermIterMut::new(self)
    }

    /// Recursively find all `any` variables referenced by the LTerm.
    pub fn anyvars(self: &LTerm<U>) -> Vec<LTerm<U>> {
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

impl<U: User> From<LTermInner<U>> for LTerm<U> {
    fn from(inner: LTermInner<U>) -> LTerm<U> {
        LTerm {
            inner: Rc::new(inner),
        }
    }
}

impl<U: User> AsRef<LTermInner<U>> for LTerm<U> {
    fn as_ref(&self) -> &LTermInner<U> {
        &self.inner
    }
}

impl<U: User> AsMut<LTermInner<U>> for LTerm<U> {
    fn as_mut(&mut self) -> &mut LTermInner<U> {
        Rc::make_mut(&mut self.inner)
    }
}

impl<U: User> fmt::Debug for LTerm<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_ref() {
            LTermInner::Val(val) => write!(f, "{:?}", val),
            LTermInner::Var(uid, name) => write!(f, "Var({:?}, {:?})", uid, name),
            LTermInner::User(user) => write!(f, "User({:?})", user),
            LTermInner::Projection(p) => write!(f, "Projection({:?})", p),
            LTermInner::Empty => write!(f, "Empty"),
            LTermInner::Cons(head, tail) => write!(f, "({:?}, {:?})", head, tail),
        }
    }
}

impl<U: User> fmt::Display for LTerm<U> {
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
        }
    }
}

impl<U: User> Hash for LTerm<U> {
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
        }
    }
}

impl<U: User> PartialEq<LTerm<U>> for LTerm<U> {
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
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LValue> for LTerm<U> {
    fn eq(&self, other: &LValue) -> bool {
        match self.as_ref() {
            LTermInner::Val(v) => v == other,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LTerm<U>> for LValue {
    fn eq(&self, other: &LTerm<U>) -> bool {
        match other.as_ref() {
            LTermInner::Val(v) => v == self,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<bool> for LTerm<U> {
    fn eq(&self, other: &bool) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Bool(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LTerm<U>> for bool {
    fn eq(&self, other: &LTerm<U>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Bool(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<isize> for LTerm<U> {
    fn eq(&self, other: &isize) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Number(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LTerm<U>> for isize {
    fn eq(&self, other: &LTerm<U>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Number(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<char> for LTerm<U> {
    fn eq(&self, other: &char) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Char(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LTerm<U>> for char {
    fn eq(&self, other: &LTerm<U>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Char(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<String> for LTerm<U> {
    fn eq(&self, other: &String) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LTerm<U>> for String {
    fn eq(&self, other: &LTerm<U>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<str> for LTerm<U> {
    fn eq(&self, other: &str) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LTerm<U>> for str {
    fn eq(&self, other: &LTerm<U>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<&str> for LTerm<U> {
    fn eq(&self, other: &&str) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl<U: User> PartialEq<LTerm<U>> for &str {
    fn eq(&self, other: &LTerm<U>) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl<U: User> Eq for LTerm<U> {}

impl<U: User> Default for LTerm<U> {
    fn default() -> Self {
        LTerm::from(LTermInner::Empty)
    }
}

impl<U: User> FromIterator<LTerm<U>> for LTerm<U> {
    fn from_iter<T: IntoIterator<Item = LTerm<U>>>(iter: T) -> Self {
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

impl<U: User> Extend<LTerm<U>> for LTerm<U> {
    fn extend<T: IntoIterator<Item = LTerm<U>>>(&mut self, coll: T) {
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
        let mut extension: LTerm<U> = coll.into_iter().collect();
        std::mem::swap(tail.as_mut(), extension.as_mut());
    }
}

#[derive(Clone, Debug)]
pub struct LTermIter<'a, U: User> {
    maybe_next: Option<&'a LTerm<U>>,
}

impl<'a, U: User> LTermIter<'a, U> {
    pub fn new(u: &'a LTerm<U>) -> LTermIter<'a, U> {
        LTermIter {
            maybe_next: Some(u),
        }
    }
}

impl<'a, U: User> Iterator for LTermIter<'a, U> {
    type Item = &'a LTerm<U>;

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

impl<'a, U: User> std::iter::FusedIterator for LTermIter<'a, U> {}

impl<'a, U: User> IntoIterator for &'a LTerm<U> {
    type Item = &'a LTerm<U>;
    type IntoIter = LTermIter<'a, U>;

    fn into_iter(self) -> LTermIter<'a, U> {
        LTermIter::new(self)
    }
}

#[derive(Debug)]
pub struct LTermIterMut<'a, U: User> {
    maybe_next: Option<&'a mut LTerm<U>>,
}

impl<'a, U: User> LTermIterMut<'a, U> {
    pub fn new(u: &'a mut LTerm<U>) -> LTermIterMut<'a, U> {
        LTermIterMut {
            maybe_next: Some(u),
        }
    }
}

impl<'a, U: User> Iterator for LTermIterMut<'a, U> {
    type Item = &'a mut LTerm<U>;

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

impl<'a, U: User> IntoIterator for &'a mut LTerm<U> {
    type Item = &'a mut LTerm<U>;
    type IntoIter = LTermIterMut<'a, U>;

    fn into_iter(self) -> LTermIterMut<'a, U> {
        LTermIterMut::new(self)
    }
}

impl<'a, U: User> std::iter::FusedIterator for LTermIterMut<'a, U> {}

impl<T, U: User> From<T> for LTerm<U>
where
    T: Into<LValue>,
{
    fn from(u: T) -> LTerm<U> {
        LTerm::from(LTermInner::Val(u.into()))
    }
}

impl<U: User> Index<usize> for LTerm<U> {
    type Output = LTerm<U>;

    fn index(&self, index: usize) -> &Self::Output {
        self.iter().nth(index).unwrap()
    }
}

impl<U: User> IndexMut<usize> for LTerm<U> {
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
        let mut u = LTerm::<EmptyUser>::var("x");
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
        let _ = LTerm::<EmptyUser>::var("_");
    }

    #[test]
    fn test_lterm_val_1() {
        let mut u: LTerm<EmptyUser> = lterm!(1);
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
        let mut u: LTerm<EmptyUser> = lterm!(true);
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
        let u: LTerm<EmptyUser> = lterm!([]);
        let mut iter = u.iter();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_2() {
        let u: LTerm<EmptyUser> = lterm!([1]);
        let mut iter = u.iter();
        assert_eq!(iter.next().unwrap(), &1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_3() {
        let u: LTerm<EmptyUser> = lterm!([1, 2, 3]);
        let mut iter = u.iter();
        assert_eq!(iter.next().unwrap(), &1);
        assert_eq!(iter.next().unwrap(), &2);
        assert_eq!(iter.next().unwrap(), &3);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_4() {
        let u: LTerm<EmptyUser> = lterm!([1, 2 | 3]);
        let mut iter = u.iter();
        assert_eq!(iter.next().unwrap(), &1);
        assert_eq!(iter.next().unwrap(), &2);
        assert_eq!(iter.next().unwrap(), &3);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_5() {
        let u: LTerm<EmptyUser> = lterm!([1, 2, 3]);
        let mut iter = IntoIterator::into_iter(&u);
        assert_eq!(iter.next().unwrap(), &1);
        assert_eq!(iter.next().unwrap(), &2);
        assert_eq!(iter.next().unwrap(), &3);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_mut_1() {
        let mut u: LTerm<EmptyUser> = lterm!([1, 2, 3]);
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
        let mut u: LTerm<EmptyUser> = lterm!([1, 2, 3]);
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
        let v: Vec<LTerm<EmptyUser>> = vec![lterm!(1), lterm!(2), lterm!(3)];
        let u: LTerm<EmptyUser> = LTerm::from_iter(v);
        assert!(u == lterm!([1, 2, 3]));
    }

    #[test]
    fn test_lterm_extend_1() {
        let v = vec![lterm!(1), lterm!(2), lterm!(3)];
        let mut u: LTerm<EmptyUser> = lterm!([]);
        u.extend(v);
        assert!(u == lterm!([1, 2, 3]));
    }

    #[test]
    fn test_lterm_extend_2() {
        let v = vec![lterm!(1), lterm!(2), lterm!(3)];
        let mut u: LTerm<EmptyUser> = lterm!([4, 5, 6]);
        u.extend(v);
        assert!(u == lterm!([4, 5, 6, 1, 2, 3]));
    }

    #[test]
    fn test_lterm_eq_1() {
        // LTerm vs. LTerm
        assert_eq!(lterm!(1) as LTerm<EmptyUser>, lterm!(1));
        assert_eq!(lterm!(true) as LTerm<EmptyUser>, lterm!(true));
        assert_eq!(lterm!("foo") as LTerm<EmptyUser>, lterm!("foo"));
        assert_eq!(lterm!('a') as LTerm<EmptyUser>, lterm!('a'));
        assert_eq!(lterm!([1, 2, 3]) as LTerm<EmptyUser>, lterm!([1, 2, 3]));
        assert_ne!(lterm!(1) as LTerm<EmptyUser>, lterm!(2));
        assert_ne!(lterm!(1) as LTerm<EmptyUser>, lterm!(true));
        assert_ne!(lterm!(1) as LTerm<EmptyUser>, lterm!('a'));
        assert_ne!(lterm!(1) as LTerm<EmptyUser>, lterm!([]));
        assert_ne!(lterm!(1) as LTerm<EmptyUser>, lterm!([1]));
        assert_ne!(lterm!(1) as LTerm<EmptyUser>, lterm!("true"));
        let u: LTerm<EmptyUser> = LTerm::var("x");
        let v: LTerm<EmptyUser> = LTerm::var("x");
        assert_eq!(u, u);
        assert_ne!(u, v);
    }

    #[test]
    fn test_lterm_eq_2() {
        // LTerm vs. Rust constant
        assert_eq!(lterm!(1) as LTerm<EmptyUser>, 1);
        assert_ne!(lterm!(1) as LTerm<EmptyUser>, 2);
        assert_ne!(lterm!(1) as LTerm<EmptyUser>, true);
        assert_eq!(1, lterm!(1) as LTerm<EmptyUser>);
        assert_ne!(2, lterm!(1) as LTerm<EmptyUser>);
        assert_ne!(true, lterm!(1) as LTerm<EmptyUser>);
        assert_eq!(lterm!("proto-vulcan") as LTerm<EmptyUser>, "proto-vulcan");
        assert_ne!(lterm!(["proto-vulcan"]) as LTerm<EmptyUser>, "proto-vulcan");
        assert_eq!("proto-vulcan", lterm!("proto-vulcan") as LTerm<EmptyUser>);
        assert_ne!("proto-vulcan", lterm!(["proto-vulcan"]) as LTerm<EmptyUser>);
        assert_eq!(
            lterm!("proto-vulcan") as LTerm<EmptyUser>,
            "proto-vulcan"[0..]
        );
        assert_ne!(
            lterm!(["proto-vulcan"]) as LTerm<EmptyUser>,
            "proto-vulcan"[0..]
        );
        assert_eq!(
            "proto-vulcan"[0..],
            lterm!("proto-vulcan") as LTerm<EmptyUser>
        );
        assert_ne!(
            "proto-vulcan"[0..],
            lterm!(["proto-vulcan"]) as LTerm<EmptyUser>
        );
        assert_eq!(
            lterm!("proto-vulcan") as LTerm<EmptyUser>,
            String::from("proto-vulcan")
        );
        assert_ne!(
            lterm!(["proto-vulcan"]) as LTerm<EmptyUser>,
            String::from("proto-vulcan")
        );
        assert_eq!(
            String::from("proto-vulcan"),
            lterm!("proto-vulcan") as LTerm<EmptyUser>
        );
        assert_ne!(
            String::from("proto-vulcan"),
            lterm!(["proto-vulcan"]) as LTerm<EmptyUser>
        );
        assert_eq!(lterm!('a') as LTerm<EmptyUser>, 'a');
        assert_ne!('b', lterm!('a') as LTerm<EmptyUser>);
        assert_ne!(lterm!(['a']) as LTerm<EmptyUser>, 'a');
        assert_ne!('a', lterm!(['a']) as LTerm<EmptyUser>);
        assert_ne!(lterm!(1) as LTerm<EmptyUser>, lterm!([1]));
        assert_ne!(lterm!([1]), lterm!(1) as LTerm<EmptyUser>);
    }

    #[test]
    fn test_lterm_eq_3() {
        // LTerm vs. LValue
        assert_eq!(lterm!(1) as LTerm<EmptyUser>, LValue::from(1));
        assert_ne!(lterm!(1) as LTerm<EmptyUser>, LValue::from(2));
        assert_eq!(LValue::from(1), lterm!(1) as LTerm<EmptyUser>);
        assert_ne!(LValue::from(2), lterm!(1) as LTerm<EmptyUser>);
        assert_ne!(LValue::from(1), lterm!([1]) as LTerm<EmptyUser>);
        assert_ne!(lterm!([1]) as LTerm<EmptyUser>, LValue::from(1));
    }

    #[test]
    #[should_panic]
    fn test_lterm_projection_1() {
        // Comparison with projection panics
        let u: LTerm<EmptyUser> = LTerm::var("x");
        let v = LTerm::projection(u.clone());
        assert_eq!(u, v);
    }

    #[test]
    #[should_panic]
    fn test_lterm_projection_2() {
        // Comparison with projection panics
        let u: LTerm<EmptyUser> = LTerm::var("x");
        let v = LTerm::projection(u.clone());
        assert_eq!(v, u);
    }

    #[test]
    #[should_panic]
    fn test_lterm_projection_3() {
        // Hash of projection panics
        let mut t = HashMap::new();
        let u: LTerm<EmptyUser> = LTerm::var("x");
        let v = LTerm::projection(u.clone());
        t.insert(v, lterm!(1) as LTerm<EmptyUser>);
    }

    #[test]
    fn test_lterm_index_1() {
        let u: LTerm<EmptyUser> = lterm!([1, [2], false]);
        assert_eq!(u[0], 1);
        assert_eq!(u[1], lterm!([2]));
        assert_eq!(u[2], false);
    }

    #[test]
    fn test_lterm_index_mut_1() {
        let mut u: LTerm<EmptyUser> = lterm!([0, 0, 0]);
        u[0] = lterm!(1);
        u[1] = lterm!([2]);
        u[2] = lterm!(false);
        assert_eq!(u[0], 1);
        assert_eq!(u[1], lterm!([2]));
        assert_eq!(u[2], false);
    }

    #[test]
    fn test_lterm_display() {
        assert_eq!(format!("{}", lterm!(1234) as LTerm<EmptyUser>), "1234");
        assert_eq!(format!("{}", lterm!(-1234) as LTerm<EmptyUser>), "-1234");
        assert_eq!(format!("{}", lterm!(true) as LTerm<EmptyUser>), "true");
        assert_eq!(format!("{}", lterm!(false) as LTerm<EmptyUser>), "false");
        assert_eq!(format!("{}", LTerm::var("x") as LTerm<EmptyUser>), "x");
        assert_eq!(format!("{}", lterm!([]) as LTerm<EmptyUser>), "[]");
        assert_eq!(
            format!("{}", lterm!([1, [2], true, 'a']) as LTerm<EmptyUser>),
            "[1, [2], true, 'a']"
        );
        assert_eq!(
            format!("{}", lterm!([1, 2 | 3]) as LTerm<EmptyUser>),
            "[1, 2 | 3]"
        );
        let u = LTerm::var("x");
        assert_eq!(
            format!("{}", LTerm::projection(u) as LTerm<EmptyUser>),
            "Projection(x)"
        );
    }
}
