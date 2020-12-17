use crate::user::UserUnify;
use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
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
pub enum LTermInner {
    /// Literal value
    Val(LValue),

    /// Variable (uid, name)
    Var(VarID, &'static str),

    // Unifiable user defined item. PartialEq and Hash are derived from the Rc pointer.
    User(Rc<dyn UserUnify>),

    // Empty list
    Empty,

    /// Non-empty list
    Cons(LTerm, LTerm),

    // Projection variable. A Projection variable will cause panic if it is tested for equality
    // or a hash is computed. To use in substitutions, it must be projected first to non-Projection
    // kind LTerm.
    Projection(LTerm),
}

#[derive(Clone)]
pub struct LTerm {
    inner: Rc<LTermInner>,
}

impl LTerm {
    pub fn ptr_eq(this: &LTerm, other: &LTerm) -> bool {
        Rc::ptr_eq(&this.inner, &other.inner)
    }

    pub fn var(name: &'static str) -> LTerm {
        if name == "_" {
            panic!("Error: Invalid variable name. Name \"_\" is reserved for any-variables.")
        }

        LTerm {
            inner: Rc::new(LTermInner::Var(VarID::new(), name)),
        }
    }

    pub fn any() -> LTerm {
        LTerm {
            inner: Rc::new(LTermInner::Var(VarID::new(), "_")),
        }
    }

    pub fn user(u: Rc<dyn UserUnify>) -> LTerm {
        LTerm {
            inner: Rc::new(LTermInner::User(u)),
        }
    }

    /// Constructs an empty list
    ///
    pub fn empty_list() -> LTerm {
        LTerm {
            inner: Rc::new(LTermInner::Empty),
        }
    }

    /// Constructs a LTerm list with a single element
    ///
    pub fn singleton(u: LTerm) -> LTerm {
        LTerm {
            inner: Rc::new(LTermInner::Cons(u, LTerm::empty_list())),
        }
    }

    pub fn projection(u: LTerm) -> LTerm {
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
        F: FnOnce(&LTerm) -> LTerm,
    {
        match self.as_ref() {
            LTermInner::Projection(p) => {
                let ptr: *const LTermInner = self.inner.as_ref();
                let projected = f(p).into_inner();
                let _ = unsafe {
                    let mut_ptr = ptr as *mut LTermInner;
                    std::ptr::replace(mut_ptr, projected.as_ref().clone())
                };
            }
            _ => panic!("Cannot project non-Projection LTerm."),
        }
    }

    pub fn into_inner(self) -> Rc<LTermInner> {
        self.inner
    }

    /// Construct a list cell
    pub fn cons(head: LTerm, tail: LTerm) -> LTerm {
        LTerm {
            inner: Rc::new(LTermInner::Cons(head, tail)),
        }
    }

    pub fn from_vec(l: Vec<LTerm>) -> LTerm {
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

    pub fn from_array(a: &[LTerm]) -> LTerm {
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

    pub fn improper_from_vec(mut h: Vec<LTerm>) -> LTerm {
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

    pub fn improper_from_array(h: &[LTerm]) -> LTerm {
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

    pub fn contains<T: Borrow<LTerm>>(&self, v: &T) -> bool {
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
            LTermInner::Var(_, _) => true,
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

    pub fn get_user(&self) -> Option<&Rc<dyn UserUnify>> {
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

    pub fn get_projection(&self) -> Option<&LTerm> {
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

    pub fn head(&self) -> Option<&LTerm> {
        match self.as_ref() {
            LTermInner::Cons(head, _) => Some(head),
            _ => None,
        }
    }

    pub fn tail(&self) -> Option<&LTerm> {
        match self.as_ref() {
            LTermInner::Cons(_, tail) => Some(tail),
            _ => None,
        }
    }

    pub fn head_mut(&mut self) -> Option<&mut LTerm> {
        match self.as_mut() {
            LTermInner::Cons(head, _) => Some(head),
            _ => None,
        }
    }

    pub fn tail_mut(&mut self) -> Option<&mut LTerm> {
        match self.as_mut() {
            LTermInner::Cons(_, tail) => Some(tail),
            _ => None,
        }
    }

    pub fn iter(&self) -> LTermIter<'_> {
        LTermIter::new(self)
    }

    /// Recursively find all `any` variables referenced by the LTerm.
    pub fn anyvars(self: &LTerm) -> Vec<LTerm> {
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

impl From<LTermInner> for LTerm {
    fn from(inner: LTermInner) -> LTerm {
        LTerm {
            inner: Rc::new(inner),
        }
    }
}

impl AsRef<LTermInner> for LTerm {
    fn as_ref(&self) -> &LTermInner {
        &self.inner
    }
}

impl AsMut<LTermInner> for LTerm {
    fn as_mut(&mut self) -> &mut LTermInner {
        Rc::make_mut(&mut self.inner)
    }
}

impl fmt::Debug for LTerm {
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

impl fmt::Display for LTerm {
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

impl Hash for LTerm {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.as_ref() {
            LTermInner::Val(val) => val.hash(state),
            LTermInner::Var(uid, _) => uid.hash(state),
            LTermInner::User(user) => Rc::as_ptr(user).hash(state),
            LTermInner::Projection(_) => panic!("Cannot compute hash for LTerm::Projection."),
            LTermInner::Empty => ().hash(state),
            LTermInner::Cons(head, tail) => {
                head.hash(state);
                tail.hash(state);
            }
        }
    }
}

impl PartialEq<LTerm> for LTerm {
    fn eq(&self, other: &Self) -> bool {
        match (self.as_ref(), other.as_ref()) {
            (LTermInner::Var(self_uid, _), LTermInner::Var(other_uid, _)) => self_uid == other_uid,
            (LTermInner::Val(self_val), LTermInner::Val(other_val)) => self_val == other_val,
            (LTermInner::User(self_user), LTermInner::User(other_user)) => {
                Rc::ptr_eq(self_user, other_user)
            }
            (LTermInner::Projection(_), _) => panic!("Cannot compare LTerm::Projection."),
            (LTermInner::Empty, LTermInner::Empty) => true,
            (LTermInner::Cons(self_head, self_tail), LTermInner::Cons(other_head, other_tail)) => {
                (self_head == other_head) & (self_tail == other_tail)
            }
            _ => false,
        }
    }
}

impl PartialEq<LValue> for LTerm {
    fn eq(&self, other: &LValue) -> bool {
        match self.as_ref() {
            LTermInner::Val(v) => v == other,
            _ => false,
        }
    }
}

impl PartialEq<LTerm> for LValue {
    fn eq(&self, other: &LTerm) -> bool {
        match other.as_ref() {
            LTermInner::Val(v) => v == self,
            _ => false,
        }
    }
}


impl PartialEq<bool> for LTerm {
    fn eq(&self, other: &bool) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Bool(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LTerm> for bool {
    fn eq(&self, other: &LTerm) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Bool(x)) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<isize> for LTerm {
    fn eq(&self, other: &isize) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Number(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LTerm> for isize {
    fn eq(&self, other: &LTerm) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Number(x)) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<char> for LTerm {
    fn eq(&self, other: &char) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::Char(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LTerm> for char {
    fn eq(&self, other: &LTerm) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::Char(x)) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<String> for LTerm {
    fn eq(&self, other: &String) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LTerm> for String {
    fn eq(&self, other: &LTerm) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<&str> for LTerm {
    fn eq(&self, other: &&str) -> bool {
        match self.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LTerm> for &str {
    fn eq(&self, other: &LTerm) -> bool {
        match other.as_ref() {
            LTermInner::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl Eq for LTerm {}

impl Default for LTerm {
    fn default() -> Self {
        LTerm::from(LTermInner::Empty)
    }
}

impl FromIterator<LTerm> for LTerm {
    fn from_iter<T: IntoIterator<Item = LTerm>>(iter: T) -> Self {
        let mut list_head = LTerm::empty_list();
        let mut list_tail = &mut list_head;
        for elem in iter {
            let _ = std::mem::replace(list_tail.as_mut(), LTermInner::Cons(elem, LTerm::empty_list()));
            list_tail = list_tail.tail_mut().unwrap();
        }
        list_head
    }
}

impl Extend<LTerm> for LTerm {
    fn extend<T: IntoIterator<Item = LTerm>>(&mut self, coll: T) {
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
        let mut extension: LTerm = coll.into_iter().collect();
        std::mem::swap(tail.as_mut(), extension.as_mut());
    }
}

#[derive(Clone, Debug)]
pub struct LTermIter<'a> {
    maybe_next: Option<&'a LTerm>,
}

impl<'a> LTermIter<'a> {
    pub fn new(u: &'a LTerm) -> LTermIter<'a> {
        LTermIter {
            maybe_next: Some(u),
        }
    }
}

impl<'a> Iterator for LTermIter<'a> {
    type Item = &'a LTerm;

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
            },
            Some(LTermInner::Empty) => {
                self.maybe_next = None;
                None
            },
            Some(_) => {
                // If the list is improper, it ends in non-cons term.
                self.maybe_next.take()
            },
            _ => None, // Iterator is finished
        }
    }
}

impl<'a> std::iter::FusedIterator for LTermIter<'a> {}

impl<'a> IntoIterator for &'a LTerm {
    type Item = &'a LTerm;
    type IntoIter = LTermIter<'a>;

    fn into_iter(self) -> LTermIter<'a> {
        LTermIter::new(self)
    }
}

impl<T> From<T> for LTerm
where
    T: Into<LValue>,
{
    fn from(u: T) -> LTerm {
        LTerm::from(LTermInner::Val(u.into()))
    }
}

impl std::ops::Index<usize> for LTerm {
    type Output = LTerm;

    fn index(&self, index: usize) -> &Self::Output {
        self.iter().nth(index).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_lterm_iter_1() {
        let u = lterm!([]);
        let mut iter = u.iter();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_2() {
        let u = lterm!([1]);
        let mut iter = u.iter();
        assert_eq!(iter.next().unwrap(), &1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_3() {
        let u = lterm!([1, 2, 3]);
        let mut iter = u.iter();
        assert_eq!(iter.next().unwrap(), &1);
        assert_eq!(iter.next().unwrap(), &2);
        assert_eq!(iter.next().unwrap(), &3);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_4() {
        let u = lterm!([1, 2 | 3]);
        let mut iter = u.iter();
        assert_eq!(iter.next().unwrap(), &1);
        assert_eq!(iter.next().unwrap(), &2);
        assert_eq!(iter.next().unwrap(), &3);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_from_iter_1() {
        let v = vec![lterm!(1), lterm!(2), lterm!(3)];
        let u = LTerm::from_iter(v);
        assert!(u == lterm!([1, 2, 3]));
    }

    #[test]
    fn test_lterm_extend_1() {
        let v = vec![lterm!(1), lterm!(2), lterm!(3)];
        let mut u = lterm!([]);
        u.extend(v);
        assert!(u == lterm!([1, 2, 3]));
    }

    #[test]
    fn test_lterm_extend_2() {
        let v = vec![lterm!(1), lterm!(2), lterm!(3)];
        let mut u = lterm!([4, 5, 6]);
        u.extend(v);
        assert!(u == lterm!([4, 5, 6, 1, 2, 3]));
    }
}
