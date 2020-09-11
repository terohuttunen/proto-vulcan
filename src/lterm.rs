use crate::state::UserUnify;
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
#[derive(Clone)]
pub enum LTerm {
    /// Literal value
    Val(LValue),

    /// Variable (uid, name)
    Var(VarID, &'static str),

    // Unifiable user defined item. PartialEq and Hash are derived from the Rc pointer.
    User(Rc<dyn UserUnify>),

    // Empty list
    Empty,

    /// Non-empty list
    Cons(Rc<LTerm>, Rc<LTerm>),

    // Projection variable. A Projection variable will cause panic if it is tested for equality
    // or a hash is computed. To use in substitutions, it must be projected first to non-Projection
    // kind LTerm.
    Projection(Rc<LTerm>),
}

impl LTerm {
    pub fn var(name: &'static str) -> Rc<LTerm> {
        if name == "_" {
            panic!("Error: Invalid variable name. Name \"_\" is reserved for any-variables.")
        }
        // Obtain unique identifier for the Any-kind variable
        Rc::new(LTerm::Var(VarID::new(), name))
    }

    pub fn any() -> Rc<LTerm> {
        Rc::new(LTerm::Var(VarID::new(), "_"))
    }

    /// Constructs an empty list
    ///
    pub fn empty_list() -> Rc<LTerm> {
        Rc::new(LTerm::Empty)
    }

    /// Constructs a LTerm list with a single element
    ///
    pub fn singleton(u: Rc<LTerm>) -> Rc<LTerm> {
        Rc::new(LTerm::Cons(u, LTerm::empty_list()))
    }

    pub fn projection(u: Rc<LTerm>) -> Rc<LTerm> {
        match u.as_ref() {
            LTerm::Var(_, _) => Rc::new(LTerm::Projection(u)),
            _ => unreachable!(),
        }
    }

    /// Convert LTerm::Projection into non-Projection kind LTerm using the projection function `f`
    /// that is applied to the projection variable.
    pub fn project<F>(&self, f: F)
    where
        F: FnOnce(&Rc<LTerm>) -> LTerm,
    {
        match self {
            LTerm::Projection(p) => unsafe {
                let ptr: *const LTerm = self;
                let mut_ptr = ptr as *mut LTerm;
                let old = std::ptr::replace(mut_ptr, f(p));
                drop(old)
            },
            _ => panic!("Cannot project non-Projection LTerm."),
        }
    }

    /// Construct a list cell
    pub fn cons(head: Rc<LTerm>, tail: Rc<LTerm>) -> Rc<LTerm> {
        Rc::new(LTerm::Cons(head, tail))
    }

    pub fn from_vec(l: Vec<Rc<LTerm>>) -> Rc<LTerm> {
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

    pub fn from_array(a: &[Rc<LTerm>]) -> Rc<LTerm> {
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

    pub fn improper_from_vec(mut h: Vec<Rc<LTerm>>) -> Rc<LTerm> {
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

    pub fn improper_from_array(h: &[Rc<LTerm>]) -> Rc<LTerm> {
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

    pub fn uncons(&mut self) -> Option<Rc<LTerm>> {
        match std::mem::replace(self, LTerm::Empty) {
            LTerm::Empty => None,
            LTerm::Cons(head, mut tail) => {
                std::mem::swap(self, Rc::make_mut(&mut tail));
                Some(head)
            }
            _ => panic!("Only lists can be unconstructed"),
        }
    }

    pub fn contains<T: Borrow<LTerm>>(&self, v: &T) -> bool {
        let v = v.borrow();
        self.iter().any(|u| u.as_ref() == v)
    }

    pub fn is_val(&self) -> bool {
        match self {
            LTerm::Val(_) => true,
            _ => false,
        }
    }

    pub fn is_bool(&self) -> bool {
        match self {
            LTerm::Val(LValue::Bool(_)) => true,
            _ => false,
        }
    }

    pub fn get_bool(&self) -> Option<bool> {
        match self {
            LTerm::Val(LValue::Bool(u)) => Some(*u),
            _ => None,
        }
    }

    pub fn is_number(&self) -> bool {
        match self {
            LTerm::Val(LValue::Number(_)) => true,
            _ => false,
        }
    }

    pub fn get_number(&self) -> Option<isize> {
        match self {
            LTerm::Val(LValue::Number(u)) => Some(*u),
            _ => None,
        }
    }

    pub fn is_var(&self) -> bool {
        match self {
            LTerm::Var(_, _) => true,
            _ => false,
        }
    }

    pub fn is_any(&self) -> bool {
        match self {
            LTerm::Var(_, "_") => true,
            _ => false,
        }
    }

    pub fn is_user(&self) -> bool {
        match self {
            LTerm::User(_) => true,
            _ => false,
        }
    }

    pub fn get_user(&self) -> Option<&Rc<dyn UserUnify>> {
        match self {
            LTerm::User(u) => Some(u),
            _ => None,
        }
    }

    pub fn is_projection(&self) -> bool {
        match self {
            LTerm::Projection(_) => true,
            _ => false,
        }
    }

    pub fn get_projection(&self) -> Option<&Rc<LTerm>> {
        match self {
            LTerm::Projection(p) => Some(p),
            _ => None,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            LTerm::Empty => true,
            LTerm::Cons(_, _) => true,
            _ => false,
        }
    }

    pub fn is_improper(&self) -> bool {
        match self {
            LTerm::Empty => false,
            LTerm::Cons(_, tail) => {
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
        match self {
            LTerm::Empty => true,
            _ => false,
        }
    }

    pub fn is_non_empty_list(&self) -> bool {
        match self {
            LTerm::Cons(_, _) => true,
            _ => false,
        }
    }

    pub fn head(&self) -> Option<&Rc<LTerm>> {
        match self {
            LTerm::Cons(head, _) => Some(head),
            _ => None,
        }
    }

    pub fn tail(&self) -> Option<&Rc<LTerm>> {
        match self {
            LTerm::Cons(_, tail) => Some(tail),
            _ => None,
        }
    }

    pub fn iter(&self) -> LTermIter<'_> {
        LTermIter::new(self)
    }

    /// Recursively find all `any` variables referenced by the LTerm.
    pub fn anyvars(self: &Rc<LTerm>) -> Vec<Rc<LTerm>> {
        match self.as_ref() {
            LTerm::Cons(head, tail) => {
                let mut vars = head.anyvars();
                for t in tail.iter() {
                    let tvars = t.anyvars();
                    vars.extend(tvars);
                }
                vars
            }
            _ => {
                if self.is_any() {
                    vec![Rc::clone(self)]
                } else {
                    vec![]
                }
            }
        }
    }
}

impl fmt::Debug for LTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LTerm::Val(val) => write!(f, "{:?}", val),
            LTerm::Var(uid, name) => write!(f, "Var({:?}, {:?})", uid, name),
            LTerm::User(user) => write!(f, "User({:?})", user),
            LTerm::Projection(p) => write!(f, "Projection({:?})", p),
            LTerm::Empty => write!(f, "Empty"),
            LTerm::Cons(head, tail) => write!(f, "({:?}, {:?})", head, tail),
        }
    }
}

impl fmt::Display for LTerm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LTerm::Val(val) => write!(f, "{}", val),
            LTerm::Var(uid, name) => {
                if self.is_any() {
                    write!(f, "{}.{}", name, uid)
                } else {
                    write!(f, "{}", name)
                }
            }
            LTerm::User(user) => write!(f, "User({:?})", user),
            LTerm::Projection(p) => write!(f, "Projection({})", p),
            LTerm::Empty => write!(f, "[]"),
            LTerm::Cons(_, _) => {
                if self.is_improper() {
                    write!(f, "(")?;
                    for (count, v) in self.iter().enumerate() {
                        if count != 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", v)?;
                    }
                    write!(f, ")")
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
        match self {
            LTerm::Val(val) => val.hash(state),
            LTerm::Var(uid, _) => uid.hash(state),
            LTerm::User(user) => Rc::as_ptr(user).hash(state),
            LTerm::Projection(_) => panic!("Cannot compute hash for LTerm::Projection."),
            LTerm::Empty => ().hash(state),
            LTerm::Cons(head, tail) => {
                head.hash(state);
                tail.hash(state);
            }
        }
    }
}

impl PartialEq<LTerm> for LTerm {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LTerm::Var(self_uid, _), LTerm::Var(other_uid, _)) => self_uid == other_uid,
            (LTerm::Val(self_val), LTerm::Val(other_val)) => self_val == other_val,
            (LTerm::User(self_user), LTerm::User(other_user)) => Rc::ptr_eq(self_user, other_user),
            (LTerm::Projection(_), _) => panic!("Cannot compare LTerm::Projection."),
            (LTerm::Empty, LTerm::Empty) => true,
            (LTerm::Cons(self_head, self_tail), LTerm::Cons(other_head, other_tail)) => {
                (self_head == other_head) & (self_tail == other_tail)
            }
            _ => false,
        }
    }
}

impl PartialEq<Rc<LTerm>> for LTerm {
    fn eq(&self, other: &Rc<LTerm>) -> bool {
        self == &*other
    }
}

impl PartialEq<LTerm> for Rc<LTerm> {
    fn eq(&self, other: &LTerm) -> bool {
        &*self == other
    }
}

impl PartialEq<LValue> for LTerm {
    fn eq(&self, other: &LValue) -> bool {
        match self {
            LTerm::Val(v) => v == other,
            _ => false,
        }
    }
}

impl PartialEq<LTerm> for LValue {
    fn eq(&self, other: &LTerm) -> bool {
        match other {
            LTerm::Val(v) => v == self,
            _ => false,
        }
    }
}

impl PartialEq<bool> for LTerm {
    fn eq(&self, other: &bool) -> bool {
        match self {
            LTerm::Val(LValue::Bool(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LTerm> for bool {
    fn eq(&self, other: &LTerm) -> bool {
        match other {
            LTerm::Val(LValue::Bool(x)) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<isize> for LTerm {
    fn eq(&self, other: &isize) -> bool {
        match self {
            LTerm::Val(LValue::Number(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LTerm> for isize {
    fn eq(&self, other: &LTerm) -> bool {
        match other {
            LTerm::Val(LValue::Number(x)) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<char> for LTerm {
    fn eq(&self, other: &char) -> bool {
        match self {
            LTerm::Val(LValue::Char(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LTerm> for char {
    fn eq(&self, other: &LTerm) -> bool {
        match other {
            LTerm::Val(LValue::Char(x)) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<String> for LTerm {
    fn eq(&self, other: &String) -> bool {
        match self {
            LTerm::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LTerm> for String {
    fn eq(&self, other: &LTerm) -> bool {
        match other {
            LTerm::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<&str> for LTerm {
    fn eq(&self, other: &&str) -> bool {
        match self {
            LTerm::Val(LValue::String(x)) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LTerm> for &str {
    fn eq(&self, other: &LTerm) -> bool {
        match other {
            LTerm::Val(LValue::String(x)) => x == self,
            _ => false,
        }
    }
}

impl Eq for LTerm {}

impl Default for LTerm {
    fn default() -> Self {
        LTerm::Empty
    }
}

impl FromIterator<Rc<LTerm>> for LTerm {
    // Because it is easier to build cons-list in reverse order, this inverts the order of
    // the original iterator.
    fn from_iter<T: IntoIterator<Item = Rc<LTerm>>>(iter: T) -> Self {
        let mut c = LTerm::Empty;
        for elem in iter {
            c = LTerm::Cons(elem, Rc::new(c));
        }
        c
    }
}

impl Extend<Rc<LTerm>> for LTerm {
    fn extend<T: IntoIterator<Item = Rc<LTerm>>>(&mut self, iter: T) {
        if !self.is_list() {
            panic!("Only list type (Empty or Cons) LTerms can be extended.");
        }

        match self {
            LTerm::Cons(_, tail) => {
                // Something to temporarily replace the tail with
                let mut sentinel = LTerm::empty_list();

                for elem in iter {
                    // Replace tail with sentinel
                    let t = std::mem::replace(tail, sentinel);

                    // Create new LTerm with contains old tail and replace sentinel with the
                    // new LTerm.
                    let n = LTerm::cons(elem, t);
                    sentinel = std::mem::replace(tail, n);
                }
            }
            LTerm::Empty => {
                *self = LTerm::from_iter(iter);
            }
            _ => unreachable!(),
        }
    }
}

pub struct LTermIter<'a> {
    maybe_u: Option<&'a LTerm>,
    maybe_last_improper: Option<&'a Rc<LTerm>>,
}

impl<'a> LTermIter<'a> {
    pub fn new(u: &'a LTerm) -> LTermIter<'a> {
        LTermIter {
            maybe_u: Some(u),
            maybe_last_improper: None,
        }
    }
}

impl<'a> Iterator for LTermIter<'a> {
    type Item = &'a Rc<LTerm>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.maybe_last_improper.take() {
            Some(u) => return Some(u),
            None => (),
        };

        // Take u from iterator
        let u = match self.maybe_u.take() {
            Some(u) => u,
            None => return None,
        };

        // Replace u in iterator with its tail and return head
        match u {
            LTerm::Cons(head, tail) => {
                if tail.is_list() {
                    let _ = self.maybe_u.replace(tail);
                } else {
                    let _ = self.maybe_last_improper.replace(tail);
                }
                Some(head)
            }
            _ => None,
        }
    }
}

impl<'a> IntoIterator for &'a LTerm {
    type Item = &'a Rc<LTerm>;
    type IntoIter = LTermIter<'a>;

    fn into_iter(self) -> LTermIter<'a> {
        LTermIter::new(self)
    }
}

impl Iterator for LTerm {
    type Item = Rc<LTerm>;

    fn next(&mut self) -> Option<Self::Item> {
        self.uncons()
    }
}

impl<T> From<T> for LTerm
where
    T: Into<LValue>,
{
    fn from(u: T) -> LTerm {
        LTerm::Val(u.into())
    }
}

#[macro_export]
macro_rules! lterm {
    // Pass-through section
    ( # $( $h:tt )+ ) => { $( $h )+ };
    ( # $e:expr ) => { $e };

    // Logic terms inside lists must be cloned if they are from reference
    (@cloning true ) => { ::std::rc::Rc::new($crate::lterm::LTerm::from(true)) };
    (@cloning false ) => { ::std::rc::Rc::new($crate::lterm::LTerm::from(false)) };
    (@cloning $e:ident ) => { ::std::rc::Rc::clone(&$e) };
    (@cloning _ ) => { $crate::lterm::LTerm::any() };
    (@cloning ( $( $h:tt ),+ ) ) => { $crate::lterm::LTerm::improper_from_array( &[ $( lterm!(@cloning $h) ),+ ] ) };
    (@cloning [ $($t:tt),* ] ) => { $crate::lterm::LTerm::from_array( &[ $( lterm!(@cloning $t) ),* ] ) };
    (@cloning $l:literal ) => { ::std::rc::Rc::new($crate::lterm::LTerm::from($l)) };
    (@cloning $e:expr ) => { ::std::rc::Rc::clone(&$e) };

    ( true ) => { ::std::rc::Rc::new($crate::lterm::LTerm::from(true)) };
    ( false ) => { ::std::rc::Rc::new($crate::lterm::LTerm::from(false)) };
    ( $e:ident ) => { $e };
    ( _ ) => { $crate::lterm::LTerm::any() };
    ( ( $( $h:tt ),+ ) ) => { $crate::lterm::LTerm::improper_from_array( &[ $( lterm!(@cloning $h) ),+ ] ) };
    ( [ $($t:tt),* ] ) => { $crate::lterm::LTerm::from_array( &[ $( lterm!(@cloning $t) ),* ] ) };
    ( $l:literal ) => { std::rc::Rc::new($crate::lterm::LTerm::from($l)) };
    ( $e:expr ) => { std::rc::Rc::new($crate::lterm::LTerm::from($e)) };
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
        assert_eq!(iter.next().unwrap(), &lterm!(1));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_iter_3() {
        let u = lterm!([1, 2, 3]);
        let mut iter = u.iter();
        assert_eq!(iter.next().unwrap(), &lterm!(1));
        assert_eq!(iter.next().unwrap(), &lterm!(2));
        assert_eq!(iter.next().unwrap(), &lterm!(3));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_lterm_from_iter_1() {
        let v = vec![lterm!(1), lterm!(2), lterm!(3)];
        let u = Rc::new(LTerm::from_iter(v));
        assert!(u == lterm!([3, 2, 1]));
    }

    #[test]
    fn test_lterm_extend_1() {
        let v = vec![lterm!(1), lterm!(2), lterm!(3)];
        let mut u = lterm!([]);
        Rc::make_mut(&mut u).extend(v);
        assert!(u == lterm!([3, 2, 1]));
    }

    #[test]
    fn test_lterm_extend_2() {
        let v = vec![lterm!(1), lterm!(2), lterm!(3)];
        let mut u = lterm!([4, 5, 6]);
        Rc::make_mut(&mut u).extend(v);
        assert!(u == lterm!([4, 3, 2, 1, 5, 6]));
    }
}
