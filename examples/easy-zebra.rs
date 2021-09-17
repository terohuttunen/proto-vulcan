extern crate proto_vulcan;
use proto_vulcan::prelude::*;
use proto_vulcan::relation::member;
use std::time::Instant;

fn righto<U: User, E: Engine<U>>(x: LTerm<U, E>, y: LTerm<U, E>, l: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan_closure!(
        match l {
            [first, second | _] => {
                first == y,
                second == x,
            },
            [_ | rest] => righto(x, y, rest),
        }
    )
}

fn easy_zebrao<U: User, E: Engine<U>>(houses: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!([
        // Italian lives in the second house
        [_, ["italian", _], _] == houses,
        // Spanish lives right next to red house
        righto(["spanish", _], [_, "red"], houses),
        // The Norwegian lives in the blue house
        member(["norwegian", "blue"], houses)
    ])
}

fn main() {
    let zebra = proto_vulcan_query!(|houses| { easy_zebrao(houses) });

    let start = Instant::now();
    let mut iter = zebra.run();
    let result = iter.next().unwrap();
    let duration = start.elapsed();
    println!("{}", result);
    println!("Time elapsed: {:?}", duration);
}
