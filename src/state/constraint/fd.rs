use super::{BaseConstraint, Constraint, FiniteDomainConstraint};
use crate::lterm::{LTerm, LTermInner};
use crate::lvalue::LValue;
use crate::state::{SResult, State, User};
use std::borrow::Borrow;
use std::cmp::{max, min};
use std::iter::Iterator;
use std::ops::RangeInclusive;
use std::rc::Rc;
use std::slice::Iter;

#[derive(Debug, Clone)]
pub enum FiniteDomain {
    Interval(RangeInclusive<isize>),
    Sparse(Vec<isize>),
}

impl FiniteDomain {
    pub fn is_singleton(&self) -> bool {
        match self {
            FiniteDomain::Interval(r) => (r.end() - r.start()).saturating_add(1) == 1,
            FiniteDomain::Sparse(v) => v.len() == 1,
        }
    }

    pub fn singleton_value(&self) -> Option<isize> {
        if self.is_singleton() {
            Some(self.min())
        } else {
            None
        }
    }

    pub fn min(&self) -> isize {
        match self {
            FiniteDomain::Interval(r) => *r.start(),
            FiniteDomain::Sparse(v) => v.first().copied().unwrap(),
        }
    }

    pub fn max(&self) -> isize {
        match self {
            FiniteDomain::Interval(r) => *r.end(),
            FiniteDomain::Sparse(v) => v.last().copied().unwrap(),
        }
    }

    pub fn copy_before<P: FnMut(&isize) -> bool>(&self, mut predicate: P) -> Option<FiniteDomain> {
        match self {
            FiniteDomain::Interval(r) => match r.clone().into_iter().find(predicate) {
                Some(u) => {
                    let r = *r.start()..=u.saturating_sub(1);
                    if r.is_empty() {
                        None
                    } else {
                        Some(FiniteDomain::Interval(r))
                    }
                },
                None => Some(self.clone()),
            },
            FiniteDomain::Sparse(v) => {
                let v: Vec<isize> = v.iter().copied().take_while(|u| !predicate(u)).collect();
                if v.is_empty() {
                    None
                } else {
                    Some(FiniteDomain::Sparse(v))
                }
            }
        }
    }

    pub fn drop_before<P: FnMut(&isize) -> bool>(&self, mut predicate: P) -> Option<FiniteDomain> {
        match self {
            FiniteDomain::Interval(r) => match r.clone().into_iter().find(predicate) {
                Some(u) => {
                    let r = u..=*r.end();
                    Some(FiniteDomain::Interval(r))
                },
                None => None,
            },
            FiniteDomain::Sparse(v) => {
                let v: Vec<isize> = v.iter().copied().skip_while(|u| !predicate(u)).collect();
                if v.is_empty() {
                    None
                } else {
                    Some(FiniteDomain::Sparse(v))
                }
            }
        }
    }

    pub fn intersect<T: Borrow<FiniteDomain>>(&self, other: T) -> Option<FiniteDomain> {
        match (self, other.borrow()) {
            (FiniteDomain::Interval(rself), FiniteDomain::Interval(rother)) => {
                // Intersection between two interval domains always results in
                // another interval domain.
                let max_start = max(*rself.start(), *rother.start());
                let min_end = min(*rself.end(), *rother.end());
                if max_start <= min_end {
                    Some(FiniteDomain::Interval(max_start..=min_end))
                } else {
                    None
                }
            }
            (FiniteDomain::Sparse(v), FiniteDomain::Interval(r))
            | (FiniteDomain::Interval(r), FiniteDomain::Sparse(v)) => {
                // Intersection between sparse and interval domains results in sparse
                // domain; however, the interval domain does not need to be iterated over.
                let intersection = v
                    .iter()
                    .copied()
                    .skip_while(|u| u < r.start())
                    .take_while(|u| u <= r.end())
                    .collect::<Vec<isize>>();

                if intersection.is_empty() {
                    None
                } else {
                    Some(FiniteDomain::Sparse(intersection))
                }
            }
            _ => {
                let mut intersection = vec![];
                let mut siter = self.iter();
                let mut oiter = other.borrow().iter();
                let mut maybe_s = siter.next();
                let mut maybe_o = oiter.next();
                loop {
                    match (maybe_s, maybe_o) {
                        (Some(s), Some(o)) if s > o => maybe_o = oiter.next(),
                        (Some(s), Some(o)) if s == o => {
                            maybe_o = oiter.next();
                            maybe_s = siter.next();
                            intersection.push(s);
                        }
                        (Some(s), Some(o)) if s < o => maybe_s = siter.next(),
                        _ => break,
                    }
                }

                if intersection.is_empty() {
                    None
                } else {
                    Some(FiniteDomain::Sparse(intersection))
                }
            }
        }
    }

    pub fn diff<T: Borrow<FiniteDomain>>(&self, other: T) -> Option<FiniteDomain> {
        let mut difference = vec![];
        let mut siter = self.iter();
        let mut oiter = other.borrow().iter();
        let mut maybe_s = siter.next();
        let mut maybe_o = oiter.next();
        loop {
            match (maybe_s, maybe_o) {
                (Some(s), None) => {
                    maybe_s = siter.next();
                    difference.push(s);
                }
                (Some(s), Some(o)) if s < o => {
                    maybe_s = siter.next();
                    difference.push(s);
                }
                (Some(s), Some(o)) if s == o => {
                    maybe_s = siter.next();
                    maybe_o = oiter.next();
                }
                (Some(s), Some(o)) if s > o => {
                    maybe_o = oiter.next();
                }
                _ => break,
            }
        }

        if difference.is_empty() {
            None
        } else {
            Some(FiniteDomain::Sparse(difference))
        }
    }

    pub fn is_disjoint<T: Borrow<FiniteDomain>>(&self, other: T) -> bool {
        let other = other.borrow();
        if self.min() > other.max() || self.max() < other.min() {
            return true;
        }

        let mut siter = self.iter();
        let mut oiter = other.iter();
        let mut maybe_s = siter.next();
        let mut maybe_o = oiter.next();
        loop {
            match (maybe_s, maybe_o) {
                (Some(s), Some(o)) if s > o => maybe_o = oiter.next(),
                (Some(s), Some(o)) if s == o => {
                    return false;
                }
                (Some(s), Some(o)) if s < o => maybe_s = siter.next(),
                _ => break,
            }
        }

        true
    }

    pub fn contains(&self, u: isize) -> bool {
        match self {
            FiniteDomain::Interval(r) => r.contains(&u),
            FiniteDomain::Sparse(v) => v.binary_search(&u).is_ok(),
        }
    }

    pub fn iter(&self) -> FiniteDomainIter {
        match self {
            FiniteDomain::Interval(r) => FiniteDomainIter::IntervalIter(r.clone().into_iter()),
            FiniteDomain::Sparse(v) => FiniteDomainIter::SparseIter(v.iter()),
        }
    }
}

impl PartialEq for FiniteDomain {
    fn eq(&self, other: &FiniteDomain) -> bool {
        self.diff(other).is_none()
    }
}

pub enum FiniteDomainIter<'a> {
    IntervalIter(RangeInclusive<isize>),
    SparseIter(Iter<'a, isize>),
}

impl<'a> Iterator for FiniteDomainIter<'a> {
    type Item = isize;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            FiniteDomainIter::IntervalIter(r) => r.next(),
            FiniteDomainIter::SparseIter(v) => v.copied().next(),
        }
    }
}

impl<'a> DoubleEndedIterator for FiniteDomainIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            FiniteDomainIter::IntervalIter(r) => r.next_back(),
            FiniteDomainIter::SparseIter(v) => v.copied().next_back(),
        }
    }
}

impl From<Vec<isize>> for FiniteDomain {
    fn from(mut v: Vec<isize>) -> FiniteDomain {
        if v.is_empty() {
            panic!("Cannot construct empty finite domain");
        }
        v.sort();
        FiniteDomain::Sparse(v)
    }
}

impl From<RangeInclusive<isize>> for FiniteDomain {
    fn from(r: RangeInclusive<isize>) -> FiniteDomain {
        FiniteDomain::Interval(r)
    }
}

impl From<&RangeInclusive<isize>> for FiniteDomain {
    fn from(r: &RangeInclusive<isize>) -> FiniteDomain {
        FiniteDomain::Interval(r.clone())
    }
}

impl From<isize> for FiniteDomain {
    fn from(u: isize) -> FiniteDomain {
        FiniteDomain::from(u..=u)
    }
}

impl From<&[isize]> for FiniteDomain {
    fn from(a: &[isize]) -> FiniteDomain {
        let a = a.to_vec().to_owned();
        FiniteDomain::from(a)
    }
}

// Finite Domain Constraints
#[derive(Debug, Clone)]
pub struct LessThanOrEqualFdConstraint {
    u: LTerm,
    v: LTerm,
}

impl LessThanOrEqualFdConstraint {
    pub fn new<U: User>(u: LTerm, v: LTerm) -> Constraint<U> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        Constraint::FiniteDomain(Rc::new(LessThanOrEqualFdConstraint { u, v }))
    }
}

impl<U: User> BaseConstraint<U> for LessThanOrEqualFdConstraint {
    fn run(self: Rc<Self>, state: State<U>) -> SResult<U> {
        let smap = state.get_smap();
        let dstore = state.get_dstore();

        let uwalk = smap.walk(&self.u);
        let maybe_udomain = dstore.get(uwalk);

        let vwalk = smap.walk(&self.v);
        let maybe_vdomain = dstore.get(vwalk);

        match (maybe_udomain, maybe_vdomain) {
            (Some(udomain), Some(vdomain)) => {
                // Both variables of the constraints have assigned domains, we can evaluate
                // the constraint. The constraint implies that min(u) <= max(v).
                let vmax = vdomain.max();
                let umin = udomain.min();
                Ok(state
                    .process_domain(
                        &uwalk,
                        Rc::new(udomain.copy_before(|u| vmax < *u).ok_or(())?),
                    )?
                    .process_domain(
                        &vwalk,
                        Rc::new(vdomain.drop_before(|v| umin <= *v).ok_or(())?),
                    )?
                    .with_constraint(self))
            }
            (Some(udomain), None) if vwalk.is_number() => {
                // The variable `u` has an assigned domain, and variable `v` has been bound
                // to a number. After the number constraint has been applied to the domain,
                // the constraint is dropped.
                let v = vwalk.get_number().unwrap();
                Ok(state
                    .process_domain(&uwalk, Rc::new(udomain.copy_before(|u| v < *u).ok_or(())?))?)
            }
            (None, Some(vdomain)) if uwalk.is_number() => {
                // The variable `v` has an assigned domain, and variable `u` has been bound
                // to a number. After the number constraint has been applied to the domain,
                // the constraint is dropped.
                let u = uwalk.get_number().unwrap();
                Ok(state
                    .process_domain(&vwalk, Rc::new(vdomain.drop_before(|v| u <= *v).ok_or(())?))?)
            }
            (None, None) if uwalk.is_number() && vwalk.is_number() => {
                // Both variables are bound to numbers. Constraint is no longer needed if it
                // is not broken.
                let u = uwalk.get_number().unwrap();
                let v = vwalk.get_number().unwrap();
                if u <= v {
                    // Constraint was successful
                    Ok(state)
                } else {
                    // Constraint failed
                    Err(())
                }
            }
            _ => {
                // The variables do not yet have assigned domains, add constraint back to
                // the store waiting for the domains to be assigned later.
                Ok(state.with_constraint(self))
            }
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.u.clone(), self.v.clone()]
    }
}

impl std::fmt::Display for LessThanOrEqualFdConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

impl<U: User> FiniteDomainConstraint<U> for LessThanOrEqualFdConstraint {}

impl<U: User> From<Rc<LessThanOrEqualFdConstraint>> for Constraint<U> {
    fn from(c: Rc<LessThanOrEqualFdConstraint>) -> Constraint<U> {
        Constraint::FiniteDomain(c as Rc<dyn FiniteDomainConstraint<U>>)
    }
}

#[derive(Debug)]
pub struct PlusFdConstraint {
    u: LTerm,
    v: LTerm,
    w: LTerm,
}

impl PlusFdConstraint {
    pub fn new<U: User>(u: LTerm, v: LTerm, w: LTerm) -> Constraint<U> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        Constraint::FiniteDomain(Rc::new(PlusFdConstraint { u, v, w }))
    }
}

impl<U: User> BaseConstraint<U> for PlusFdConstraint {
    fn run(self: Rc<Self>, state: State<U>) -> SResult<U> {
        let smap = state.get_smap();
        let dstore = state.get_dstore();

        let uwalk = smap.walk(&self.u);
        let singleton_udomain;
        let maybe_udomain = match uwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(uwalk),
            LTermInner::Val(LValue::Number(u)) => {
                singleton_udomain = Rc::new(FiniteDomain::from(*u));
                Some(&singleton_udomain)
            }
            _ => None,
        };

        let vwalk = smap.walk(&self.v);
        let singleton_vdomain;
        let maybe_vdomain = match vwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(vwalk),
            LTermInner::Val(LValue::Number(v)) => {
                singleton_vdomain = Rc::new(FiniteDomain::from(*v));
                Some(&singleton_vdomain)
            }
            _ => None,
        };

        let wwalk = smap.walk(&self.w);
        let singleton_wdomain;
        let maybe_wdomain = match wwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(wwalk),
            LTermInner::Val(LValue::Number(w)) => {
                singleton_wdomain = Rc::new(FiniteDomain::from(*w));
                Some(&singleton_wdomain)
            }
            _ => None,
        };

        // If all operators are bound to numbers, then we can drop the constraint or fail if
        // constraint is not fulfilled.
        if uwalk.is_number() && vwalk.is_number() && wwalk.is_number() {
            if uwalk.get_number().unwrap() + vwalk.get_number().unwrap()
                == wwalk.get_number().unwrap()
            {
                return Ok(state);
            } else {
                return Err(());
            }
        }

        match (maybe_udomain, maybe_vdomain, maybe_wdomain) {
            (Some(udomain), Some(vdomain), Some(wdomain)) => {
                let umin = udomain.min();
                let umax = udomain.max();
                let vmin = vdomain.min();
                let vmax = vdomain.max();
                let wmin = wdomain.min();
                let wmax = wdomain.max();
                // The constraint is: u + v = w
                //
                // Given domains for u and v, we can then deduce that the domain of w must be
                // in range [umin + vmin .. umax + vmax]. The constraining domain is built and
                // intersected with the current domain of w in .process_domain()-call.
                //
                // Same application of constraining domain is done for the other two variables.
                //
                // The constraint is not dropped until all variables converge into numbers.
                Ok(state
                    .process_domain(
                        &wwalk,
                        Rc::new(FiniteDomain::from(
                            umin.saturating_add(vmin)..=umax.saturating_add(vmax),
                        )),
                    )?
                    .process_domain(
                        &uwalk,
                        Rc::new(FiniteDomain::from(
                            wmin.saturating_sub(vmax)..=wmax.saturating_sub(vmin),
                        )),
                    )?
                    .process_domain(
                        &vwalk,
                        Rc::new(FiniteDomain::from(
                            wmin.saturating_sub(umax)..=wmax.saturating_sub(umin),
                        )),
                    )?
                    .with_constraint(self))
            }
            // If all operators do not yet have domains, then keep the constraint until it can
            // be used to constrain some domains.
            _ => Ok(state.with_constraint(self)),
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.u.clone(), self.v.clone(), self.w.clone()]
    }
}

impl std::fmt::Display for PlusFdConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

impl<U: User> FiniteDomainConstraint<U> for PlusFdConstraint {}

impl<U: User> From<Rc<PlusFdConstraint>> for Constraint<U> {
    fn from(c: Rc<PlusFdConstraint>) -> Constraint<U> {
        Constraint::FiniteDomain(c as Rc<dyn FiniteDomainConstraint<U>>)
    }
}

#[derive(Debug)]
pub struct MinusFdConstraint {
    u: LTerm,
    v: LTerm,
    w: LTerm,
}

impl MinusFdConstraint {
    pub fn new<U: User>(u: LTerm, v: LTerm, w: LTerm) -> Constraint<U> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        Constraint::FiniteDomain(Rc::new(MinusFdConstraint { u, v, w }))
    }
}

impl<U: User> BaseConstraint<U> for MinusFdConstraint {
    fn run(self: Rc<Self>, state: State<U>) -> SResult<U> {
        let smap = state.get_smap();
        let dstore = state.get_dstore();

        let uwalk = smap.walk(&self.u);
        let singleton_udomain;
        let maybe_udomain = match uwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(uwalk),
            LTermInner::Val(LValue::Number(u)) => {
                singleton_udomain = Rc::new(FiniteDomain::from(*u));
                Some(&singleton_udomain)
            }
            _ => None,
        };

        let vwalk = smap.walk(&self.v);
        let singleton_vdomain;
        let maybe_vdomain = match vwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(vwalk),
            LTermInner::Val(LValue::Number(v)) => {
                singleton_vdomain = Rc::new(FiniteDomain::from(*v));
                Some(&singleton_vdomain)
            }
            _ => None,
        };

        let wwalk = smap.walk(&self.w);
        let singleton_wdomain;
        let maybe_wdomain = match wwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(wwalk),
            LTermInner::Val(LValue::Number(w)) => {
                singleton_wdomain = Rc::new(FiniteDomain::from(*w));
                Some(&singleton_wdomain)
            }
            _ => None,
        };

        // If all operators are bound to numbers, then we can drop the constraint or fail if
        // constraint is not fulfilled.
        if uwalk.is_number() && vwalk.is_number() && wwalk.is_number() {
            if uwalk.get_number().unwrap() - vwalk.get_number().unwrap()
                == wwalk.get_number().unwrap()
            {
                return Ok(state);
            } else {
                return Err(());
            }
        }

        match (maybe_udomain, maybe_vdomain, maybe_wdomain) {
            (Some(udomain), Some(vdomain), Some(wdomain)) => {
                let umin = udomain.min();
                let umax = udomain.max();
                let vmin = vdomain.min();
                let vmax = vdomain.max();
                let wmin = wdomain.min();
                let wmax = wdomain.max();
                // The constraint is: u - v = w  <=>  u = w + v  <=>  v = u - w
                //
                // Given domains for u and v, we can then deduce that the domain of w must be
                // in range [umin - vmax .. umax + vmin]. The constraining domain is built and
                // intersected with the current domain of w in .process_domain()-call.
                //
                // Same application of constraining domain is done for the other two variables.
                //   w = u - v  =>  [umin - vmax .. umax - vmin]
                //   u = w + v  =>  [wmin + vmin .. wmax + vmax]
                //   v = u - w  =>  [umin - wmax .. umax - wmin]
                //
                // The constraint is not dropped until all variables converge into numbers.
                Ok(state
                    .process_domain(
                        &wwalk,
                        Rc::new(FiniteDomain::from(
                            umin.saturating_sub(vmax)..=umax.saturating_sub(vmin),
                        )),
                    )?
                    .process_domain(
                        &uwalk,
                        Rc::new(FiniteDomain::from(
                            wmin.saturating_add(vmin)..=wmax.saturating_add(vmax),
                        )),
                    )?
                    .process_domain(
                        &vwalk,
                        Rc::new(FiniteDomain::from(
                            umin.saturating_sub(wmax)..=umax.saturating_sub(wmin),
                        )),
                    )?
                    .with_constraint(self))
            }
            // If all operators do not yet have domains, then keep the constraint until it can
            // be used to constrain some domains.
            _ => Ok(state.with_constraint(self)),
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.u.clone(), self.v.clone(), self.w.clone()]
    }
}

impl std::fmt::Display for MinusFdConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

impl<U: User> FiniteDomainConstraint<U> for MinusFdConstraint {}

impl<U: User> From<Rc<MinusFdConstraint>> for Constraint<U> {
    fn from(c: Rc<MinusFdConstraint>) -> Constraint<U> {
        Constraint::FiniteDomain(c as Rc<dyn FiniteDomainConstraint<U>>)
    }
}

#[derive(Debug)]
pub struct TimesFdConstraint {
    u: LTerm,
    v: LTerm,
    w: LTerm,
}

impl TimesFdConstraint {
    pub fn new<U: User>(u: LTerm, v: LTerm, w: LTerm) -> Constraint<U> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        assert!(w.is_var() || w.is_number());
        Constraint::FiniteDomain(Rc::new(TimesFdConstraint { u, v, w }))
    }
}

impl<U: User> BaseConstraint<U> for TimesFdConstraint {
    fn run(self: Rc<Self>, state: State<U>) -> SResult<U> {
        let smap = state.get_smap();
        let dstore = state.get_dstore();

        let uwalk = smap.walk(&self.u);
        let singleton_udomain;
        let maybe_udomain = match uwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(uwalk),
            LTermInner::Val(LValue::Number(u)) => {
                singleton_udomain = Rc::new(FiniteDomain::from(*u));
                Some(&singleton_udomain)
            }
            _ => None,
        };

        let vwalk = smap.walk(&self.v);
        let singleton_vdomain;
        let maybe_vdomain = match vwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(vwalk),
            LTermInner::Val(LValue::Number(v)) => {
                singleton_vdomain = Rc::new(FiniteDomain::from(*v));
                Some(&singleton_vdomain)
            }
            _ => None,
        };

        let wwalk = smap.walk(&self.w);
        let singleton_wdomain;
        let maybe_wdomain = match wwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(wwalk),
            LTermInner::Val(LValue::Number(w)) => {
                singleton_wdomain = Rc::new(FiniteDomain::from(*w));
                Some(&singleton_wdomain)
            }
            _ => None,
        };

        // If all operators are bound to numbers, then we can drop the constraint or fail if
        // constraint is not fulfilled.
        if uwalk.is_number() && vwalk.is_number() && wwalk.is_number() {
            if uwalk.get_number().unwrap() * vwalk.get_number().unwrap()
                == wwalk.get_number().unwrap()
            {
                return Ok(state);
            } else {
                return Err(());
            }
        }

        match (maybe_udomain, maybe_vdomain, maybe_wdomain) {
            (Some(udomain), Some(vdomain), Some(wdomain)) => {
                let umin = udomain.min();
                let umax = udomain.max();
                let vmin = vdomain.min();
                let vmax = vdomain.max();
                let wmin = wdomain.min();
                let wmax = wdomain.max();
                // The constraint is: u * v = w  <=>  u = w / v  <=>  v = w / u
                //
                // Given domains for u and v, we can then deduce that the domain of w must be
                // in range [umin - vmax .. umax + vmin]. The constraining domain is built and
                // intersected with the current domain of w in .process_domain()-call.
                //
                // Same application of constraining domain is done for the other two variables.
                //   w = u * v  =>  [umin * vmin .. umax * vmax]
                //   u = w / v  =>  [wmin / vmax .. wmax / vmin]
                //   v = w / u  =>  [wmin / umax .. wmax / umin]
                //
                // The constraint is not dropped until all variables converge into numbers.
                Ok(state
                    .process_domain(
                        &wwalk,
                        Rc::new(FiniteDomain::from(
                            umin.saturating_mul(vmin)..=umax.saturating_mul(vmax),
                        )),
                    )?
                    .process_domain(
                        &uwalk,
                        Rc::new(FiniteDomain::from(
                            wmin.checked_div(vmax).unwrap_or(umin)
                                ..=wmax.checked_div(vmin).unwrap_or(umax),
                        )),
                    )?
                    .process_domain(
                        &vwalk,
                        Rc::new(FiniteDomain::from(
                            wmin.checked_div(umax).unwrap_or(vmin)
                                ..=wmax.checked_div(umin).unwrap_or(vmax),
                        )),
                    )?
                    .with_constraint(self))
            }
            // If all operators do not yet have domains, then keep the constraint until it can
            // be used to constrain some domains.
            _ => Ok(state.with_constraint(self)),
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.u.clone(), self.v.clone(), self.w.clone()]
    }
}

impl std::fmt::Display for TimesFdConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

impl<U: User> FiniteDomainConstraint<U> for TimesFdConstraint {}

impl<U: User> From<Rc<TimesFdConstraint>> for Constraint<U> {
    fn from(c: Rc<TimesFdConstraint>) -> Constraint<U> {
        Constraint::FiniteDomain(c as Rc<dyn FiniteDomainConstraint<U>>)
    }
}

#[derive(Debug)]
pub struct DiseqFdConstraint {
    u: LTerm,
    v: LTerm,
}

impl DiseqFdConstraint {
    pub fn new<U: User>(u: LTerm, v: LTerm) -> Constraint<U> {
        assert!(u.is_var() || u.is_number());
        assert!(v.is_var() || v.is_number());
        Constraint::FiniteDomain(Rc::new(DiseqFdConstraint { u, v }))
    }
}

impl<U: User> BaseConstraint<U> for DiseqFdConstraint {
    fn run(self: Rc<Self>, state: State<U>) -> SResult<U> {
        let smap = state.get_smap();
        let dstore = state.get_dstore();

        let u = self.u.clone();
        let uwalk = smap.walk(&u);
        let singleton_udomain;
        let maybe_udomain = match uwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(uwalk),
            LTermInner::Val(LValue::Number(u)) => {
                singleton_udomain = Rc::new(FiniteDomain::from(*u));
                Some(&singleton_udomain)
            }
            _ => None,
        };

        let v = self.v.clone();
        let vwalk = smap.walk(&v);
        let singleton_vdomain;
        let maybe_vdomain = match vwalk.as_ref() {
            LTermInner::Var(_, _) => dstore.get(vwalk),
            LTermInner::Val(LValue::Number(v)) => {
                singleton_vdomain = Rc::new(FiniteDomain::from(*v));
                Some(&singleton_vdomain)
            }
            _ => None,
        };

        match (maybe_udomain, maybe_vdomain) {
            (Some(udomain), Some(vdomain)) if udomain.is_singleton() && vdomain.is_singleton() => {
                // Both variables have singleton domains. If values are same, the constraint
                // fails in the current state and is dropped; if the values are different, the constraint
                // succeeds and is dropped.
                if udomain.min() == vdomain.min() {
                    Err(())
                } else {
                    Ok(state)
                }
            }
            (Some(udomain), Some(vdomain)) if udomain.is_disjoint(vdomain.as_ref()) => {
                // When the domains are disjoint, the constraint can never be violated.
                // Constraint can be dropped.
                Ok(state)
            }
            (Some(udomain), Some(vdomain)) => {
                // The domains are not both singleton or disjoint. The constraints are kept
                // until they can be resolved into singleton, or until they become disjoint.
                let state = state.with_constraint(self);
                if udomain.is_singleton() {
                    state.process_domain(vwalk, Rc::new(vdomain.diff(udomain.as_ref()).ok_or(())?))
                } else if vdomain.is_singleton() {
                    state.process_domain(uwalk, Rc::new(udomain.diff(vdomain.as_ref()).ok_or(())?))
                } else {
                    Ok(state)
                }
            }
            _ => {
                // One or both of the variables do not yet have domains. Keep the constraint
                // for later.
                Ok(state.with_constraint(self))
            }
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.u.clone(), self.v.clone()]
    }
}

impl std::fmt::Display for DiseqFdConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

impl<U: User> FiniteDomainConstraint<U> for DiseqFdConstraint {}

impl<U: User> From<Rc<DiseqFdConstraint>> for Constraint<U> {
    fn from(c: Rc<DiseqFdConstraint>) -> Constraint<U> {
        Constraint::FiniteDomain(c as Rc<dyn FiniteDomainConstraint<U>>)
    }
}

#[derive(Debug)]
pub struct DistinctFdConstraint {
    u: LTerm,
}

impl DistinctFdConstraint {
    pub fn new<U: User>(u: LTerm) -> Constraint<U> {
        assert!(u.is_list());
        Constraint::FiniteDomain(Rc::new(DistinctFdConstraint { u }))
    }
}

impl<U: User> BaseConstraint<U> for DistinctFdConstraint {
    fn run(self: Rc<Self>, state: State<U>) -> SResult<U> {
        let smap = state.get_smap();

        let v = smap.walk(&self.u);
        match v.as_ref() {
            LTermInner::Var(_, _) => {
                // The term has not yet been associated with a list of terms that we want
                // to constrain, keep the constraint for later.
                Ok(state.with_constraint(self))
            }
            LTermInner::Empty | LTermInner::Cons(_, _) => {
                // Partition the list of terms to unresolved variables in `x` and constants in `n`.
                let (x, n): (LTerm, LTerm) = v.iter().cloned().partition(|v| v.is_var());

                // Convert list of LTerm constants to Vec<usize>
                let mut n = n
                    .iter()
                    .map(|t| match t.as_ref() {
                        LTermInner::Val(LValue::Number(u)) => *u,
                        _ => panic!("Invalid constant constraint {:?}", t),
                    })
                    .collect::<Vec<isize>>();

                // Sort the array so that we can find duplicates with a simple scan
                n.sort_unstable();

                // See if there are any duplicate values in the sorted array.
                let mut it = n.iter();
                let no_duplicates = match it.next() {
                    Some(first) => it
                        .scan(first, |previous, current| {
                            let cmp = *previous < current;
                            *previous = current;
                            Some(cmp)
                        })
                        .all(|cmp| cmp),
                    None => true,
                };

                if no_duplicates {
                    // There are no duplicate constant constraints. Create a new constraint
                    // to follow the fulfillment of the variable domain constraints.
                    let c = DistinctFd2Constraint::new(self.u.clone(), x, n);
                    Ok(state.with_constraint(c))
                } else {
                    // If there are duplicate constants in the array, then the constraint is
                    // already violated.
                    Err(())
                }
            }
            _ => panic!(
                "Cannot constrain {:?}. The variable must be grounded to a list of terms.",
                v
            ),
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.u.clone()]
    }
}

impl std::fmt::Display for DistinctFdConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

impl<U: User> FiniteDomainConstraint<U> for DistinctFdConstraint {}

impl<U: User> From<Rc<DistinctFdConstraint>> for Constraint<U> {
    fn from(c: Rc<DistinctFdConstraint>) -> Constraint<U> {
        Constraint::FiniteDomain(c as Rc<dyn FiniteDomainConstraint<U>>)
    }
}

#[derive(Debug, Clone)]
struct DistinctFd2Constraint {
    u: LTerm,
    y: LTerm,
    n: Vec<isize>,
}

impl DistinctFd2Constraint {
    pub fn new<U: User>(u: LTerm, y: LTerm, n: Vec<isize>) -> Constraint<U> {
        assert!(u.is_list());
        assert!(y.is_list());
        Constraint::FiniteDomain(Rc::new(DistinctFd2Constraint { u, y, n }))
    }
}

impl<U: User> BaseConstraint<U> for DistinctFd2Constraint {
    fn run(mut self: Rc<Self>, state: State<U>) -> SResult<U> {
        let smap = state.get_smap();

        let mut x = LTerm::empty_list();
        let mut mself = Rc::make_mut(&mut self);
        for y in mself.y.into_iter() {
            let ywalk = smap.walk(&y);
            match ywalk.as_ref() {
                LTermInner::Var(_, _) => {
                    // Terms that walk to variables cannot be resolved to values yet. Such terms
                    // are moved from y to x, where they will become the new y on next run of
                    // constraints.
                    x.extend(Some(y.clone()));
                }
                LTermInner::Val(val) => {
                    // A variable has been associated with a value and can be moved from y to n.
                    match val {
                        LValue::Number(u) => {
                            match mself.n.binary_search(u) {
                                Ok(_) => {
                                    // Duplicate invalidates the constraint
                                    return Err(());
                                }
                                Err(pos) => {
                                    // Add the previously unseen value to the list of constant
                                    // constraints.
                                    mself.n.insert(pos, *u);
                                }
                            }
                        }
                        _ => panic!("Invalid value {:?} in constraint", val),
                    }
                }
                _ => panic!("Invalid LTerm  {:?} in constraint", ywalk),
            }
        }

        // Create a new all-diff constraint with (hopefully) less unassociated variables in y and
        // more constants in n.
        mself.y = x.clone();
        if mself.n.is_empty() {
            Ok(state.with_constraint(self))
        } else {
            let ndomain = Rc::new(FiniteDomain::from(mself.n.clone()));
            state.with_constraint(self).exclude_from_domain(&x, ndomain)
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        self.u.iter().cloned().collect()
    }
}

impl std::fmt::Display for DistinctFd2Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "")
    }
}

impl<U: User> FiniteDomainConstraint<U> for DistinctFd2Constraint {}

impl<U: User> From<Rc<DistinctFd2Constraint>> for Constraint<U> {
    fn from(c: Rc<DistinctFd2Constraint>) -> Constraint<U> {
        Constraint::FiniteDomain(c as Rc<dyn FiniteDomainConstraint<U>>)
    }
}
