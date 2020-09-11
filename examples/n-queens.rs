#![recursion_limit = "512"]
extern crate proto_vulcan;
use proto_vulcan::relation::diseqfd;
use proto_vulcan::relation::distinctfd;
use proto_vulcan::relation::firsto;
use proto_vulcan::relation::infdrange;
use proto_vulcan::relation::plusfd;
use proto_vulcan::relation::resto;
use proto_vulcan::*;
use std::ops::RangeInclusive;
use std::rc::Rc;

fn diago(
    qi: &Rc<LTerm>,
    qj: &Rc<LTerm>,
    d: &Rc<LTerm>,
    range: &RangeInclusive<isize>,
) -> Rc<dyn Goal> {
    proto_vulcan!(
        |qi_plus_d, qj_plus_d| {
            infdrange([qi_plus_d, qj_plus_d], #range),
            plusfd(qi, d, qi_plus_d),
            diseqfd(qi_plus_d, qj),
            plusfd(qj, d, qj_plus_d),
            diseqfd(qj_plus_d, qi)
        }
    )
}

fn diagonalso(n: isize, i: isize, j: isize, s: &Rc<LTerm>, r: &Rc<LTerm>) -> Rc<dyn Goal> {
    /* Slightly faster but uglier non-relational version: */
    /*
    if r.is_empty() || r.tail().unwrap().is_empty() {
        proto_vulcan!(true)
    } else if s.is_empty() {
        let tail = r.tail().unwrap();
        let tail_tail = r.tail().unwrap().tail().unwrap();
        proto_vulcan!(
            diagonalso(#n, #i + 1, #i + 2, tail_tail, tail)
        )
    } else {
        let qi = r.head().unwrap();
        let qj = s.head().unwrap();
        proto_vulcan!([
            diago(qi, qj, (j - i), #&(0..=2 * n)),
            diagonalso(#n, #i, #j + 1, #s.tail().unwrap(), r)
        ])
    }
    */
    /* Fully relational version: */
    let s = Rc::clone(s);
    let r = Rc::clone(r);
    proto_vulcan!(
        closure {
            conde {
                r == [],
                |tail| {
                    resto(r, tail),
                    tail == [],
                },
                |tail, tail_tail| {
                    s == [],
                    resto(r, tail),
                    resto(tail, tail_tail),
                    diagonalso(#n, #i + 1, #i + 2, tail_tail, tail),
                },
                |qi, qj, tail| {
                    firsto(r, qi),
                    firsto(s, qj),
                    resto(s, tail),
                    diago(qi, qj, (j - i), #&(0..=2 * n)),
                    diagonalso(#n, #i, #j + 1, tail, r),
                }
            }
        }
    )
}

fn nqueenso(queens: &Rc<LTerm>, n: isize, i: isize, l: &Rc<LTerm>) -> Rc<dyn Goal> {
    if i == 0 {
        proto_vulcan!([distinctfd(l), diagonalso(#n, #0, #1, #l.tail().unwrap(), #l), queens == l])
    } else {
        proto_vulcan!(|x| {
            infdrange(x, #&(1..=n)),
            nqueenso(queens, #n, #i - 1, #&LTerm::cons(Rc::clone(&x), Rc::clone(l)))
        })
    }
}

fn main() {
    let n = 8;
    let query = proto_vulcan_query!(|queens| {
        nqueenso(queens, #n, #n, [])
    });

    for (i, result) in query.run().enumerate() {
        println!("{}: {}", i, result.queens);
    }
}
