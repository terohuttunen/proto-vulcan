extern crate proto_vulcan;
use proto_vulcan::proto_vulcan;
use proto_vulcan::relation::diseqfd;
use proto_vulcan::relation::distinctfd;
use proto_vulcan::relation::infdrange;
use proto_vulcan::relation::plusfd;
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
    let s = Rc::clone(s);
    let r = Rc::clone(r);
    proto_vulcan_closure!(
        match r {
            [] | [_] => ,
            [_, second | rest] => {
                s == [],
                diagonalso(#n, #i + 1, #i + 2, rest, [second | rest]),
            },
            [qi | _] => {
                |qj, tail| {
                    s == [qj | tail],
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
        let queens = Rc::clone(queens);
        let l = Rc::clone(l);
        proto_vulcan_closure!(|x| {
            infdrange(x, #&(1..=n)),
            nqueenso(queens, #n, #i - 1, [x | l])
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
