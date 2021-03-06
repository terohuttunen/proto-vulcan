extern crate proto_vulcan;
use proto_vulcan::prelude::*;

fn main() {
    let query = proto_vulcan_query!(|x, y| {
        [x, 1] != [2, y],
    });

    for result in query.run() {
        println!("{}", result);
    }
}
