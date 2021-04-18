extern crate proto_vulcan;
use proto_vulcan::prelude::*;

fn main() {
    let query = proto_vulcan_query!(|q| {
        conde {
            q == 1,
            q == 2,
            q == 3,
        }
    });

    for result in query.run() {
        println!("{}", result);
    }
}
