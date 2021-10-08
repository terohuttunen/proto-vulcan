extern crate proto_vulcan;
use proto_vulcan::prelude::*;
use proto_vulcan::relation::diseqfd;
use proto_vulcan::relation::distinctfd;
use proto_vulcan::relation::infdrange;
use proto_vulcan::relation::plusfd;
use std::ops::RangeInclusive;

fn diago<U: User, E: Engine<U>>(
    qi: LTerm<U, E>,
    qj: LTerm<U, E>,
    d: LTerm<U, E>,
    range: &RangeInclusive<isize>,
) -> Goal<U, E> {
    proto_vulcan!(
        |qi_plus_d, qj_plus_d| {
            infdrange([qi_plus_d, qj_plus_d], {range}),
            plusfd(qi, d, qi_plus_d),
            diseqfd(qi_plus_d, qj),
            plusfd(qj, d, qj_plus_d),
            diseqfd(qj_plus_d, qi)
        }
    )
}

fn diagonalso<U: User, E: Engine<U>>(
    n: isize,
    i: isize,
    j: isize,
    s: LTerm<U, E>,
    r: LTerm<U, E>,
) -> Goal<U, E> {
    proto_vulcan_closure!(
        match r {
            [] | [_] => ,
            [_, second | rest] => {
                s == [],
                diagonalso({n}, {i + 1}, {i + 2}, rest, [second | rest]),
            },
            [qi | _] => {
                |qj, tail| {
                    s == [qj | tail],
                    diago(qi, qj, {j - i}, &(0..=2 * n)),
                    diagonalso({n}, {i}, {j + 1}, tail, r),
                }
            }
        }
    )
}

fn nqueenso<U: User, E: Engine<U>>(
    queens: LTerm<U, E>,
    n: isize,
    i: isize,
    l: LTerm<U, E>,
) -> Goal<U, E> {
    if i == 0 {
        proto_vulcan!(|ltail| {
            l == [_ | ltail],
            [distinctfd(l), diagonalso({n}, {0isize}, {1isize}, ltail, l), queens == l]
        })
    } else {
        proto_vulcan_closure!(|x| {
            infdrange(x, &(1..=n)),
            nqueenso(queens, {n}, {i - 1}, [x | l])
        })
    }
}

fn main() {
    let n: isize = 8;
    let query = proto_vulcan_query!(|queens| { nqueenso(queens, { n }, { n }, []) });

    for (i, result) in query.run().enumerate() {
        println!("{}: {}", i, result.queens);
    }
}
