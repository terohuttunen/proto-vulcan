use crate::lterm::LTerm;
use std::any::Any;
use std::borrow::Borrow;
use std::cmp::{max, min};
use std::iter::Iterator;
use std::ops::RangeInclusive;
use std::slice::Iter;
use std::vec::IntoIter;

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
                }
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
                }
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

    /// Union of two finite domains (for join operation in lattice)
    pub fn union<T: Borrow<FiniteDomain>>(&self, other: T) -> FiniteDomain {
        let mut union = vec![];
        let mut siter = self.iter();
        let mut oiter = other.borrow().iter();
        let mut maybe_s = siter.next();
        let mut maybe_o = oiter.next();

        loop {
            match (maybe_s, maybe_o) {
                (Some(s), Some(o)) if s < o => {
                    union.push(s);
                    maybe_s = siter.next();
                }
                (Some(s), Some(o)) if s == o => {
                    union.push(s);
                    maybe_s = siter.next();
                    maybe_o = oiter.next();
                }
                (Some(s), Some(o)) if s > o => {
                    union.push(o);
                    maybe_o = oiter.next();
                }
                (Some(s), None) => {
                    union.push(s);
                    maybe_s = siter.next();
                }
                (None, Some(o)) => {
                    union.push(o);
                    maybe_o = oiter.next();
                }
                _ => break,
            }
        }

        if union.is_empty() {
            panic!("Union resulted in empty domain");
        }

        FiniteDomain::Sparse(union)
    }

    /// Check if this domain contains all elements of another domain (subsumption)
    pub fn contains_domain<T: Borrow<FiniteDomain>>(&self, other: T) -> bool {
        // self contains other if other ∩ self == other
        if let Some(intersection) = self.intersect(other.borrow()) {
            intersection.equivalent(other.borrow())
        } else {
            false
        }
    }

    /// Check if two domains are equivalent (contain same elements)
    pub fn equivalent<T: Borrow<FiniteDomain>>(&self, other: T) -> bool {
        // Two domains are equivalent if their difference is empty in both directions
        self.diff(other.borrow()).is_none() && other.borrow().diff(self).is_none()
    }

    /// Check if domain is empty
    pub fn is_empty(&self) -> bool {
        match self {
            FiniteDomain::Interval(r) => r.is_empty(),
            FiniteDomain::Sparse(v) => v.is_empty(),
        }
    }

    /// Get the number of elements in the domain
    pub fn len(&self) -> usize {
        match self {
            FiniteDomain::Interval(r) => {
                if r.is_empty() {
                    0
                } else {
                    (r.end() - r.start()) as usize + 1
                }
            }
            FiniteDomain::Sparse(v) => v.len(),
        }
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

    pub fn into_iter(self) -> FiniteDomainIntoIter {
        match self {
            FiniteDomain::Interval(r) => FiniteDomainIntoIter::IntervalIter(r.clone().into_iter()),
            FiniteDomain::Sparse(v) => FiniteDomainIntoIter::SparseIter(v.clone().into_iter()),
        }
    }
}

impl PartialEq for FiniteDomain {
    fn eq(&self, other: &FiniteDomain) -> bool {
        self.diff(other).is_none()
    }
}

impl std::fmt::Display for FiniteDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FiniteDomain::Interval(r) => {
                write!(f, "{}..{}", r.start(), r.end())
            }
            FiniteDomain::Sparse(v) => {
                if v.len() <= 10 {
                    write!(
                        f,
                        "{{{}}}",
                        v.iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                } else {
                    write!(
                        f,
                        "{{{}, ..., {} ({} elements)}}",
                        v[0],
                        v[v.len() - 1],
                        v.len()
                    )
                }
            }
        }
    }
}

/// Implementation of DomainValue trait for FiniteDomain
/// This makes FiniteDomain a proper lattice-based domain value
impl crate::state::dstore::DomainValue for FiniteDomain {
    fn domain_type(&self) -> &str {
        "clpfd" // CLPFD - Constraint Logic Programming over Finite Domains
    }

    /// Meet operation (greatest lower bound) - intersection of finite domains
    fn meet(
        &self,
        other: &dyn crate::state::dstore::DomainValue,
    ) -> Option<Box<dyn crate::state::dstore::DomainValue>> {
        if let Some(other_fd) = other.as_any().downcast_ref::<FiniteDomain>() {
            self.intersect(other_fd)
                .map(|fd| Box::new(fd) as Box<dyn crate::state::dstore::DomainValue>)
        } else {
            None // Different domain types cannot be intersected
        }
    }

    /// Join operation (least upper bound) - union of finite domains for widening
    fn join(
        &self,
        other: &dyn crate::state::dstore::DomainValue,
    ) -> Option<Box<dyn crate::state::dstore::DomainValue>> {
        if let Some(other_fd) = other.as_any().downcast_ref::<FiniteDomain>() {
            Some(Box::new(self.union(other_fd)) as Box<dyn crate::state::dstore::DomainValue>)
        } else {
            None // Different domain types
        }
    }

    /// Check if this domain subsumes another (lattice ordering)
    /// For finite domains: this subsumes other if other ⊆ this
    fn subsumes(&self, other: &dyn crate::state::dstore::DomainValue) -> bool {
        if let Some(other_fd) = other.as_any().downcast_ref::<FiniteDomain>() {
            self.contains_domain(other_fd)
        } else {
            false // Different domain types
        }
    }

    /// Apply domain-specific constraint propagation

    fn is_singleton(&self) -> bool {
        self.is_singleton()
    }

    fn singleton_lterm(&self) -> Option<LTerm> {
        if self.is_singleton() {
            Some(LTerm::from(self.min()))
        } else {
            None
        }
    }

    fn contains_lterm(&self, value: &LTerm) -> bool {
        // For CLPFD, check if the LTerm is a number and if it's in our domain
        if let Some(num) = value.get_number() {
            self.contains(num)
        } else {
            false // Non-numeric values are not in finite integer domains
        }
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn size(&self) -> Option<usize> {
        Some(self.len())
    }

    fn clone_box(&self) -> Box<dyn crate::state::dstore::DomainValue> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn equivalent(&self, other: &dyn crate::state::dstore::DomainValue) -> bool {
        if let Some(other_fd) = other.as_any().downcast_ref::<FiniteDomain>() {
            self.equivalent(other_fd)
        } else {
            false
        }
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

#[derive(Clone)]
pub enum FiniteDomainIntoIter {
    IntervalIter(RangeInclusive<isize>),
    SparseIter(IntoIter<isize>),
}

impl Iterator for FiniteDomainIntoIter {
    type Item = isize;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            FiniteDomainIntoIter::IntervalIter(r) => r.next(),
            FiniteDomainIntoIter::SparseIter(v) => v.next(),
        }
    }
}

impl DoubleEndedIterator for FiniteDomainIntoIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            FiniteDomainIntoIter::IntervalIter(r) => r.next_back(),
            FiniteDomainIntoIter::SparseIter(v) => v.next_back(),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_finitedomain_1() {
        // min, max
        let fd = FiniteDomain::from(vec![-1, 2, 3, 4]);
        assert_eq!(fd.min(), -1);
        assert_eq!(fd.max(), 4);

        let fd = FiniteDomain::from(0);
        assert_eq!(fd.min(), 0);
        assert_eq!(fd.max(), 0);
    }

    #[test]
    fn test_finitedomain_2() {
        // copy_before interval
        let fd = FiniteDomain::from(1..=8);
        let before = fd.copy_before(|x| *x > 6).unwrap();
        assert_eq!(before.min(), 1);
        assert_eq!(before.max(), 6);

        // If the predicate is never true in the finite domain, copy all
        let before = fd.copy_before(|x| *x < 0).unwrap();
        assert_eq!(before, fd);

        // If the predicate is always true, then copy none
        assert!(fd.copy_before(|x| *x > -1).is_none());
    }

    #[test]
    fn test_finitedomain_3() {
        // copy_before sparse
        let fd = FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let before = fd.copy_before(|x| *x > 6).unwrap();
        assert_eq!(before.min(), 1);
        assert_eq!(before.max(), 6);

        // If the predicate is never true in the finite domain, copy all
        let before = fd.copy_before(|x| *x < 0).unwrap();
        assert_eq!(before, fd);

        // If the predicate is always true, then copy none
        assert!(fd.copy_before(|x| *x > -1).is_none());
    }

    #[test]
    fn test_finitedomain_4() {
        // drop_before interval
        let fd = FiniteDomain::from(1..=8);
        let before = fd.drop_before(|x| *x > 6).unwrap();
        assert_eq!(before.min(), 7);
        assert_eq!(before.max(), 8);

        // If the predicate is never true in the finite domain, copy none
        assert!(fd.drop_before(|x| *x > 10).is_none());

        // If the predicate is always true, then copy all
        let after = fd.drop_before(|x| *x > 0).unwrap();
        assert_eq!(after, fd);
    }

    #[test]
    fn test_finitedomain_5() {
        // drop_before sparse
        let fd = FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let before = fd.drop_before(|x| *x > 6).unwrap();
        assert_eq!(before.min(), 7);
        assert_eq!(before.max(), 8);

        // If the predicate is never true in the finite domain, copy none
        assert!(fd.drop_before(|x| *x > 10).is_none());

        // If the predicate is always true, then copy all
        let after = fd.drop_before(|x| *x > 0).unwrap();
        assert_eq!(after, fd);
    }

    #[test]
    fn test_finitedomain_6() {
        // intersect interval with interval
        let a = FiniteDomain::from(1..=6);
        let b = FiniteDomain::from(4..=8);
        let c = FiniteDomain::from(10..=12);

        // Intersection of overlapping intervals is an interval
        let isect = a.intersect(&b).unwrap();
        assert_eq!(isect, FiniteDomain::from(4..=6));

        // Intesection of disjoint intervals is None
        assert!(a.intersect(&c).is_none());
    }

    #[test]
    fn test_finitedomain_7() {
        // intersect interval with sparse
        let a = FiniteDomain::from(1..=6);
        let b = FiniteDomain::from(vec![4, 5, 6, 7, 8]);
        let c = FiniteDomain::from(vec![10, 11, 12]);

        // Intersection of overlapping interval and sparse is a sparse
        let isect = a.intersect(&b).unwrap();
        assert_eq!(isect, FiniteDomain::from(vec![4, 5, 6]));

        let isect = b.intersect(&a).unwrap();
        assert_eq!(isect, FiniteDomain::from(vec![4, 5, 6]));

        // Intesection of disjoint intervals is None
        assert!(a.intersect(&c).is_none());
        assert!(c.intersect(&a).is_none());
    }

    #[test]
    fn test_finitedomain_8() {
        // intersect sparse with sparse
        let a = FiniteDomain::from(vec![1, 2, 3, 4, 5, 6]);
        let b = FiniteDomain::from(vec![4, 5, 6, 7, 8]);
        let c = FiniteDomain::from(vec![10, 11, 12]);

        // Intersection of overlapping sparse domains is a sparse
        let isect = a.intersect(&b).unwrap();
        assert_eq!(isect, FiniteDomain::from(vec![4, 5, 6]));

        // Intesection of disjoint intervals is None
        assert!(a.intersect(&c).is_none());
    }

    #[test]
    fn test_finitedomain_union_interval_interval() {
        // union of overlapping intervals
        let a = FiniteDomain::from(1..=6);
        let b = FiniteDomain::from(4..=8);

        let union = a.union(&b);
        assert_eq!(union, FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 7, 8]));

        let union_rev = b.union(&a);
        assert_eq!(union_rev, FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 7, 8]));

        // union of disjoint intervals
        let c = FiniteDomain::from(10..=12);
        let union_disjoint = a.union(&c);
        assert_eq!(
            union_disjoint,
            FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 10, 11, 12])
        );

        // union of adjacent intervals
        let d = FiniteDomain::from(7..=9);
        let union_adjacent = a.union(&d);
        assert_eq!(
            union_adjacent,
            FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9])
        );

        // union with identical interval
        let union_same = a.union(&a);
        assert_eq!(union_same, FiniteDomain::from(vec![1, 2, 3, 4, 5, 6]));
    }

    #[test]
    fn test_finitedomain_union_interval_sparse() {
        // union of interval with sparse
        let interval = FiniteDomain::from(1..=6);
        let sparse = FiniteDomain::from(vec![4, 5, 6, 7, 8]);

        let union = interval.union(&sparse);
        assert_eq!(union, FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 7, 8]));

        let union_rev = sparse.union(&interval);
        assert_eq!(union_rev, FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 7, 8]));

        // union of interval with disjoint sparse
        let sparse_disjoint = FiniteDomain::from(vec![10, 11, 12]);
        let union_disjoint = interval.union(&sparse_disjoint);
        assert_eq!(
            union_disjoint,
            FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 10, 11, 12])
        );

        // union of interval with sparse subset
        let sparse_subset = FiniteDomain::from(vec![2, 4]);
        let union_subset = interval.union(&sparse_subset);
        assert_eq!(union_subset, FiniteDomain::from(vec![1, 2, 3, 4, 5, 6]));

        // union of interval with sparse superset
        let sparse_superset = FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let union_superset = interval.union(&sparse_superset);
        assert_eq!(
            union_superset,
            FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9])
        );
    }

    #[test]
    fn test_finitedomain_union_sparse_sparse() {
        // union of overlapping sparse domains
        let a = FiniteDomain::from(vec![1, 2, 3, 4, 5, 6]);
        let b = FiniteDomain::from(vec![4, 5, 6, 7, 8]);

        let union = a.union(&b);
        assert_eq!(union, FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 7, 8]));

        let union_rev = b.union(&a);
        assert_eq!(union_rev, FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 7, 8]));

        // union of disjoint sparse domains
        let c = FiniteDomain::from(vec![10, 11, 12]);
        let union_disjoint = a.union(&c);
        assert_eq!(
            union_disjoint,
            FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 10, 11, 12])
        );

        // union of identical sparse domains
        let union_same = a.union(&a);
        assert_eq!(union_same, FiniteDomain::from(vec![1, 2, 3, 4, 5, 6]));

        // union with empty-like case (single elements)
        let single1 = FiniteDomain::from(vec![1]);
        let single2 = FiniteDomain::from(vec![5]);
        let union_singles = single1.union(&single2);
        assert_eq!(union_singles, FiniteDomain::from(vec![1, 5]));
    }

    #[test]
    fn test_finitedomain_union_edge_cases() {
        // union maintains sorted order regardless of input order
        let unsorted1 = FiniteDomain::from(vec![3, 1, 5, 2, 4]);
        let unsorted2 = FiniteDomain::from(vec![8, 6, 7]);

        let union = unsorted1.union(&unsorted2);
        assert_eq!(union, FiniteDomain::from(vec![1, 2, 3, 4, 5, 6, 7, 8]));

        // union with duplicates in sparse (though FiniteDomain::from should handle this)
        let with_gaps1 = FiniteDomain::from(vec![1, 3, 5]);
        let with_gaps2 = FiniteDomain::from(vec![2, 4, 6]);

        let union_gaps = with_gaps1.union(&with_gaps2);
        assert_eq!(union_gaps, FiniteDomain::from(vec![1, 2, 3, 4, 5, 6]));

        // union where one domain is subset of other
        let subset = FiniteDomain::from(vec![2, 4]);
        let superset = FiniteDomain::from(vec![1, 2, 3, 4, 5]);

        let union_subset = subset.union(&superset);
        assert_eq!(union_subset, FiniteDomain::from(vec![1, 2, 3, 4, 5]));

        let union_superset = superset.union(&subset);
        assert_eq!(union_superset, FiniteDomain::from(vec![1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_finitedomain_union_properties() {
        // Test mathematical properties of union
        let a = FiniteDomain::from(1..=5);
        let b = FiniteDomain::from(vec![3, 4, 5, 6, 7]);
        let c = FiniteDomain::from(vec![6, 7, 8, 9]);

        // Commutativity: A ∪ B = B ∪ A
        assert_eq!(a.union(&b), b.union(&a));

        // Associativity: (A ∪ B) ∪ C = A ∪ (B ∪ C)
        let left_assoc = a.union(&b).union(&c);
        let right_assoc = a.union(&b.union(&c));
        assert_eq!(left_assoc, right_assoc);

        // Idempotence: A ∪ A = A
        assert_eq!(a.union(&a), a);

        // Size property: |A ∪ B| >= max(|A|, |B|)
        let union_size = a.union(&b).len();
        assert!(union_size >= a.len().max(b.len()));

        // For disjoint sets: |A ∪ B| = |A| + |B|
        let disjoint1 = FiniteDomain::from(vec![1, 2, 3]);
        let disjoint2 = FiniteDomain::from(vec![7, 8, 9]);
        let disjoint_union = disjoint1.union(&disjoint2);
        assert_eq!(disjoint_union.len(), disjoint1.len() + disjoint2.len());
    }
}
