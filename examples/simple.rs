extern crate proto_vulcan;
use proto_vulcan::prelude::*;
use proto_vulcan::user::DefaultUser;

fn main() {
    let query = proto_vulcan_query!(|q| {
        conde {
            q == 1,
            q == 2,
            q == 3,
        }
    });

    let user_state = DefaultUser::new();
    let user_globals = ();
    /*
    for result in query.run_with_debugger(user_state, user_globals) {
        println!("{}", result);
    }
    */
}
