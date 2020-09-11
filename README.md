# proto-vulcan
A [`miniKanren`]-family relational logic programming language embedded in Rust.

In addition to core miniKanren language, proto-vulcan currently provides support for:
* Disequality constraints CLP(Tree)
* Finite-domain constraints CLP(FD)
* Various operators: anyo, conda, condu, onceo, project
* Writing goals in Rust embedded inline within proto-vulcan
* User extension interface

The language is embedded into Rust with macros which parse the language syntax and convert it
into Rust. For parsing the language syntax, it uses the [`tt-call`] library.

[`miniKanren`]: http://minikanren.org
[`tt-call`]: http://github.com/dtolnay/tt-call

Due to heavy macro-usage, the default recursion limit is not sufficient. It can be increased
with:
```rust
#![recursion_limit = "512"]
```

# Example
```rust
#![recursion_limit = "512"]
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

New relations can be defined as Rust-functions using `proto_vulcan!`-macro.
```rust
use proto_vulcan::*;
use proto_vulcan::relation::emptyo;
use proto_vulcan::relation::conso;
use std::rc::Rc;

pub fn appendo<U: UserState>(l: &Rc<LTerm>, s: &Rc<LTerm>, ls: &Rc<LTerm>) -> Rc<dyn Goal<U>> {
    let s = Rc::clone(s);
    proto_vulcan!(
        conde {
            [s == ls, emptyo(l)],
            |a, d, res| {
                conso(a, d, l),
                conso(a, res, ls),
                closure {
                    appendo(d, s, res)
                }
            }
        }
    )
}
```
More examples in documentation.

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
