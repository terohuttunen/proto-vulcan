use std::borrow::Borrow;
use std::cmp::{max, min};
use std::iter::Iterator;
use std::ops::RangeInclusive;
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
}
