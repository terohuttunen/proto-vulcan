extern crate proto_vulcan;
use proto_vulcan::prelude::*;

#[compound]
struct TreeNode(LTerm, TreeNode, TreeNode);

// A relation between a tree and a list of its nodes in
// unnamed form TreeNode(name, left, right)
fn tree_nodes_unnamed<U: User>(
    node: TreeNode<U>,
    d: LTerm<U>,
    (list, rest): (LTerm<U>, LTerm<U>),
) -> Goal<U> {
    proto_vulcan_closure!(match node {
        [] => list == rest,
        TreeNode(name, left, right) => |ls0, ls1, ls2| {
            [_|ls0] == d,
            tree_nodes_unnamed(left, ls0, (list, ls1)),
            ls1 == [name | ls2],
            tree_nodes_unnamed(right, ls0, (ls2, rest))
        },
    })
}

#[compound]
struct NamedNode {
    name: LTerm,
    left: NamedNode,
    right: NamedNode,
}

// A relation between a tree and a list of its nodes in
// named form TreeNode { name, left, right }
fn tree_nodes_named<U: User>(
    node: NamedNode<U>,
    d: LTerm<U>,
    (list, rest): (LTerm<U>, LTerm<U>),
) -> Goal<U> {
    proto_vulcan_closure!(match node {
        [] => list == rest,
        NamedNode { name, left, right } => |ls0, ls1, ls2| {
            [_|ls0] == d,
            tree_nodes_named(left, ls0, (list, ls1)),
            ls1 == [name | ls2],
            tree_nodes_named(right, ls0, (ls2, rest))
        },
    })
}

// A relation between a tree and a list of its nodes in
// untyped form [name, left, right]
fn tree_nodes<U: User>(node: LTerm<U>, d: LTerm<U>, (list, rest): (LTerm<U>, LTerm<U>)) -> Goal<U> {
    proto_vulcan_closure!(match node {
        [] => list == rest,
        [name, left, right] => |ls0, ls1, ls2| {
            [_|ls0] == d,
            tree_nodes(left, ls0, (list, ls1)),
            ls1 == [name | ls2],
            tree_nodes(right, ls0, (ls2, rest))
        },
    })
}

fn main() {
    // Collect nodes from a tree into a list difference (q, [])
    let query = proto_vulcan_query!(|q| {
        tree_nodes_unnamed(
            TreeNode(
                "a",
                TreeNode("b", [], TreeNode("c", [], [])),
                TreeNode("d", [], []),
            ),
            q,
            (q, []),
        )
    });

    println!("Tree nodes: ");
    let mut result_iter = query.run();
    let result = result_iter.next().unwrap();
    println!("{}", result);

    let list = (*result.q).clone();

    // Generate all unnamed trees that would form the list of nodes
    let query = proto_vulcan_query!(|q: TreeNode| { tree_nodes_unnamed(q, list, (list, [])) });
    println!("Unnamed trees: ");
    for result in query.run() {
        println!("{}", result.q);
    }

    // Generate all named trees that would form the list of nodes
    let query = proto_vulcan_query!(|q: NamedNode| { tree_nodes_named(q, list, (list, [])) });
    println!("Named trees: ");
    for result in query.run() {
        println!("{}", result.q);
    }

    // Generate all untyped trees that would form the list of nodes
    let query = proto_vulcan_query!(|q| { tree_nodes(q, list, (list, [])) });
    println!("Trees: ");
    for result in query.run() {
        println!("{}", result.q);
    }
}
