# proto-vulcan
<!-- CI status -->
<a href="https://github.com/terohuttunen/proto-vulcan/actions">
  <img src="https://github.com/terohuttunen/proto-vulcan/workflows/CI/badge.svg"
    alt="CI Status" />
</a>
<!-- coveralls.io coverage -->
<a href='https://coveralls.io/github/terohuttunen/proto-vulcan'>
  <img src='https://coveralls.io/repos/github/terohuttunen/proto-vulcan/badge.svg'
    alt='Coverage Status' />
</a>
<!-- Crates version -->
<a href="https://crates.io/crates/proto-vulcan">
  <img src="https://img.shields.io/crates/v/proto-vulcan.svg"
    alt="Crates.io version" />
</a>
<!-- docs.rs docs -->
<a href="https://docs.rs/proto-vulcan">
  <img src="https://img.shields.io/badge/docs-latest-blue.svg"
    alt="docs.rs docs" />
</a>
<a href=''>
  <img src='https://img.shields.io/badge/license-MIT%2FAPACHE_2.0-yellow.svg'
    alt='MIT/APACHE-2.0' />
</a>

A [`miniKanren`]-family relational logic programming language embedded in Rust.

In addition to core miniKanren language, proto-vulcan currently provides support for:
* Disequality constraints CLP(Tree)
* Finite-domain constraints CLP(FD)
* Various operators: anyo, conda, condu, onceo, project
* Pattern matching: matche, matcha, matchu
* Writing goals in Rust embedded inline within proto-vulcan
* User extension interface

The language is embedded into Rust with macros which parse the language syntax and convert it
into Rust.

[`miniKanren`]: http://minikanren.org


# Example
```rust
extern crate proto_vulcan;
use proto_vulcan::*;

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

New relations can be defined as Rust-functions using `proto_vulcan!` and
`proto_vulcan_closure!`-macros. Expressions within `proto_vulcan!` are
evaluated immediately, whereas expressions within `proto_vulcan_closure!`
are stored into a closure and evaluated later. Recursive relations must
use the latter variant.
```rust
use proto_vulcan::*;

pub fn appendo(l: LTerm, s: LTerm, ls: LTerm) -> Goal {
    proto_vulcan_closure!(
        match [l, s, ls] {
            [[], x, x] => ,
            [[x | l1], l2, [x | l3]] => appendo(l1, l2, l3),
        }
    )
}
```
More examples in [documentation](https://docs.rs/proto-vulcan/).

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
