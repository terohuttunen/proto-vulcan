# proto-vulcan
<!-- CI status -->
<a href="https://github.com/terohuttunen/proto-vulcan/actions">
  <img src="https://github.com/terohuttunen/proto-vulcan/workflows/CI/badge.svg"
    alt="CI Status" />
</a>
<!-- Codecov.io coverage -->
<a href="https://codecov.io/gh/terohuttunen/proto-vulcan">
  <img src="https://codecov.io/gh/terohuttunen/proto-vulcan/branch/master/graph/badge.svg?token=MR666G7GE9"
    alt='Coverage Status' />
  
</a>
<!-- Crates version -->
<a href="https://crates.io/crates/proto-vulcan">
  <img src="https://img.shields.io/crates/v/proto-vulcan.svg"
    alt="Crates.io version" />
</a>
<!-- docs.rs docs -->
<a href="https://docs.rs/proto-vulcan">
  <img src="https://img.shields.io/badge/docs.rs-latest-informational.svg"
    alt="docs.rs docs" />
</a>
<a href=''>
  <img src='https://img.shields.io/badge/license-MIT%2FApache--2.0-informational.svg'
    alt='MIT/APACHE-2.0' />
</a>
<!-- rustc version -->
<a href=''>
  <img src='https://img.shields.io/badge/rustc-1.54.0+-informational.svg'
    alt='Required rustc minimum version' />
</a>

A relational logic programming language embedded in Rust. It started as a yet another 
[`miniKanren`](http://minikanren.org), but has already evolved into its own language with miniKanren at its core.

In addition to core miniKanren language, proto-vulcan currently provides support for:
* miniKanren-like breadth-first and Prolog-like depth-first search.
* Compound types ([Example](examples/tree-nodes.rs))
* Disequality constraints CLP(Tree)
* Finite-domain constraints CLP(FD)
* Various operators: anyo, conda, condu, onceo, project
* Pattern matching: match, matche, matcha, matchu
* Writing goals in Rust embedded inline within proto-vulcan
* User extension interface

The language is embedded into Rust with macros which parse the language syntax and convert it
into Rust. The language looks a lot like Rust, but isn't. For example, fresh variables are
presented with Rust closure syntax, and pattern matching looks like Rust match.


# Example
```rust
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
        println!("q = {}", result.q);
    }
}
```
The example program produces three solutions:
```text
q = 1
q = 2
q = 3
```

## Embedding in Rust
To embed proto-vulcan in Rust, four macros are used: `proto_vulcan!`, `proto_vulcan_closure!`,
`proto_vulcan_query!`, and `lterm!`.

  * `proto_vulcan!(<goal>)` declares a Proto-vulcan goal, and returns a Rust
    variable of type `Goal`.
  * `proto_vulcan_closure!(<goal>)` declares a Proto-vulcan goal, and returns a Rust
    variable of type `Goal`. The goal expression is evaluated lazily when the goal
    is evaluated. The closure takes ownership of all variables referenced within the closure.
  * `proto_vulcan_query!(|a, b, c| { <goal> })` defines a Proto-vulcan query with query-variables
    `a`, `b` and `c`. The returned value is a `Query`-struct, that when `run`, produces an
    iterator that can be used to iterate over valid solutions to the logic program. The iterator
    returns a struct with fields named after the query variables.
  * `lterm!(<tree-term>)` declares a logic tree-term in Rust code, which can be passed to
    proto-vulcan program within proto_vulcan! or proto_vulcan_query!, or compared with results.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
