extern crate proto_vulcan;
use proto_vulcan::relation::membero;
use proto_vulcan::*;
use std::rc::Rc;
use std::time::Instant;

fn righto(x: &Rc<LTerm>, y: &Rc<LTerm>, l: &Rc<LTerm>) -> Rc<dyn Goal> {
    let x = Rc::clone(x);
    let y = Rc::clone(y);
    proto_vulcan!(
        match l {
            [first, second | _] => {
                first == y,
                second == x,
            },
            [_ | rest] => closure { righto(x, y, rest) },
        }
    )
}

fn easy_zebrao(houses: &Rc<LTerm>) -> Rc<dyn Goal> {
    proto_vulcan!([
        // Italian lives in the second house
        [_, ["italian", _], _] == houses,
        // Spanish lives right next to red house
        righto(["spanish", _], [_, "red"], houses),
        // The Norwegian lives in the blue house
        membero(["norwegian", "blue"], houses)
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
