extern crate proto_vulcan;
use proto_vulcan::goal::{AnyGoal, DFSGoal, Goal, InferredGoal};
//use proto_vulcan::operator::cond;
use proto_vulcan::operator::matche;
use proto_vulcan::prelude::*;
//use proto_vulcan::relation::membero;

fn test_succeed_bfs<U: User, E: Engine<U>>() -> Goal<U, E> {
    proto_vulcan!(true)
}

fn test_fail_bfs<U: User, E: Engine<U>>() -> Goal<U, E> {
    proto_vulcan!(false)
}

fn test_succeed_dfs<U: User, E: Engine<U>>() -> DFSGoal<U, E> {
    proto_vulcan!(true)
}

fn test_fail_dfs<U: User, E: Engine<U>>() -> DFSGoal<U, E> {
    proto_vulcan!(false)
}

fn test_succeed_inferred<U: User, E: Engine<U>, G: AnyGoal<U, E>>() -> InferredGoal<U, E, G> {
    proto_vulcan!(true)
}

fn test_fail_inferred<U: User, E: Engine<U>, G: AnyGoal<U, E>>() -> InferredGoal<U, E, G> {
    proto_vulcan!(false)
}

fn test_fresh_bfs<U: User, E: Engine<U>>(x: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(|y| { /*x == y*/ })
}

fn test_fresh_dfs<U: User, E: Engine<U>>(x: LTerm<U, E>) -> DFSGoal<U, E> {
    proto_vulcan!(|y| { x == y })
}

fn test_fresh_inferred<U: User, E: Engine<U>, G: AnyGoal<U, E>>(
    x: LTerm<U, E>,
) -> InferredGoal<U, E, G> {
    proto_vulcan!(|y| { x == y })
}

fn test_conjunction_bfs<U: User, E: Engine<U>>(x: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(|y, z| { [x == y, x != z] })
}

fn test_conjunction_dfs<U: User, E: Engine<U>>(x: LTerm<U, E>) -> DFSGoal<U, E> {
    proto_vulcan!(|y, z| { [x == y, x != z] })
}

fn test_conjunction_inferred<U: User, E: Engine<U>, G: AnyGoal<U, E>>(
    x: LTerm<U, E>,
) -> InferredGoal<U, E, G> {
    proto_vulcan!(|y, z| { [x == y, x != z] })
}

fn test_relation_bfs<U: User, E: Engine<U>>() -> Goal<U, E> {
    proto_vulcan!(|x, y, z| {
        [
            test_succeed_bfs(),
            test_succeed_dfs(),
            test_succeed_inferred(),
        ]
    })
}

fn test_relation_dfs<U: User, E: Engine<U>>() -> DFSGoal<U, E> {
    proto_vulcan!(|x, y, z| {
        [
            //test_succeed_bfs(),
            test_succeed_dfs(),
            test_succeed_inferred(),
        ]
    })
}

fn test_relation_inferred<U: User, E: Engine<U>, G: AnyGoal<U, E>>() -> InferredGoal<U, E, G> {
    proto_vulcan!(|x, y, z| {
        [
            //test_succeed_bfs(),
            //test_succeed_dfs(),
            test_succeed_inferred(),
        ]
    })
}

fn test_fngoal_bfs<U: User, E: Engine<U>>(x: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(
        fngoal | solver,
        state | {
            let g: Goal<U, E> = proto_vulcan!(true);
            g.solve(solver, state)
        }
    )
}

fn test_fngoal_dfs<U: User, E: Engine<U>>(x: LTerm<U, E>) -> DFSGoal<U, E> {
    proto_vulcan!(
        fngoal | solver,
        state | {
            let g: DFSGoal<U, E> = proto_vulcan!(true);
            g.solve(solver, state)
        }
    )
}

fn test_fngoal_inferred<U: User, E: Engine<U>, G: AnyGoal<U, E>>(
    x: LTerm<U, E>,
) -> InferredGoal<U, E, G> {
    proto_vulcan!(
        fngoal | solver,
        state | {
            let g: InferredGoal<U, E, G> = proto_vulcan!(true);
            g.solve(solver, state)
        }
    )
}

fn test_project_bfs<U: User, E: Engine<U>>() -> Goal<U, E> {
    proto_vulcan!(|x, y, z| {
        project | x | {
            [
                test_succeed_bfs(),
                test_succeed_dfs(),
                test_succeed_inferred(),
            ]
        }
    })
}

fn test_project_dfs<U: User, E: Engine<U>>() -> DFSGoal<U, E> {
    proto_vulcan!(|x, y, z| {
        project | x | {
            [
                //test_succeed_bfs(),
                test_succeed_dfs(),
                test_succeed_inferred(),
            ]
        }
    })
}

fn test_project_inferred<U: User, E: Engine<U>, G: AnyGoal<U, E>>() -> InferredGoal<U, E, G> {
    proto_vulcan!(|x, y, z| {
        project | x | {
            [
                //test_succeed_bfs(),
                //test_succeed_dfs(),
                test_succeed_inferred(),
            ]
        }
    })
}

fn test_match_bfs<U: User, E: Engine<U>>(x: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(|x, y, z| {
        match x {
            0 => test_succeed_bfs(),
            1 => test_succeed_dfs(),
            2 => test_succeed_inferred(),
        }
    })
}

fn test_match_dfs<U: User, E: Engine<U>>(x: LTerm<U, E>) -> DFSGoal<U, E> {
    proto_vulcan!(|x, y, z| {
        match x {
            //0 => test_succeed_bfs(),
            1 => test_succeed_dfs(),
            2 => test_succeed_inferred(),
        }
    })
}

fn test_match_inferred<U: User, E: Engine<U>, G: AnyGoal<U, E>>(
    x: LTerm<U, E>,
) -> InferredGoal<U, E, G> {
    proto_vulcan!(|x, y, z| {
        match x {
            //0 => test_succeed_bfs(),
            //1 => test_succeed_dfs(),
            2 => test_succeed_inferred(),
        }
    })
}

fn test_matche_bfs<U: User, E: Engine<U>>(x: LTerm<U, E>) -> Goal<U, E> {
    proto_vulcan!(|x, y, z| {
        matche x {
            0 => test_succeed_bfs(),
            1 => test_succeed_dfs(),
            2 => test_succeed_inferred(),
        }
    })
}

fn main() {
    /*
    let foo: DFSGoal<DefaultUser, DefaultEngine<DefaultUser>> = test_relation_dfs();
    println!("{:#?}", foo);
    */
    /*
    let query = proto_vulcan_query!(|q| {
        conde {
            //test(),
            membero(q, [1, 2, 3]),
            membero(q, [4, 5, 6]),
            membero(q, [7, 8, 9]),
        }
    });

    for result in query.run() {
        println!("{}", result);
    }
    */
}
