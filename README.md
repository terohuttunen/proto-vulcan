# Proto-Vulcan

[![CI Status](https://github.com/terohuttunen/proto-vulcan/workflows/CI/badge.svg)](https://github.com/terohuttunen/proto-vulcan/actions)
[![Coverage Status](https://codecov.io/gh/terohuttunen/proto-vulcan/branch/master/graph/badge.svg?token=MR666G7GE9)](https://codecov.io/gh/terohuttunen/proto-vulcan)
[![Crates.io](https://img.shields.io/crates/v/proto-vulcan.svg)](https://crates.io/crates/proto-vulcan)
[![Documentation](https://img.shields.io/badge/docs.rs-latest-blue.svg)](https://docs.rs/proto-vulcan)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

A modern relational logic programming language inspired by [miniKanren](http://minikanren.org). Proto-Vulcan features a native interpreter with powerful constraint solving, pattern matching, and a clean syntax for logical reasoning.

## ✨ Features

- **🔍 Logic Programming** - Solve problems through relationships and unification
- **🎯 Pattern Matching** - Advanced pattern matching with guards using `if` clauses
- **⚡ Constraint Solving** - CLP(FD) finite domains and CLP(Z) integer arithmetic
- **🏗️ Modern Syntax** - Structs, enums, implementation blocks, and methods
- **🔄 Search Control** - Explicit breadth-first and depth-first search strategies
- **📦 Module System** - Organize code with imports and visibility controls
- **🧪 Built-in Testing** - Comprehensive test framework with assertions
- **📚 Standard Library** - Lists, file I/O, environment access, and path utilities

## 🚀 Quick Start

### Installation

```bash
git clone https://github.com/terohuttunen/proto-vulcan
cd proto-vulcan
cargo build --release
```

### Hello World

Create `hello.pv`:

```prolog
use std::list::*;

@main
rel hello(greeting) {
    member(greeting, ["Hello", "Proto-Vulcan", "World!"])
}
```

Run it:

```bash
$ cargo run -- hello.pv
Query: hello(greeting)
Solutions:
  1: greeting = "Hello"
  2: greeting = "Proto-Vulcan"
  3: greeting = "World!"
```

### Logic Programming Example

```prolog
// family.pv
rel parent(parent, child) {
    any {
        all { parent == "Alice", child == "Bob" },
        all { parent == "Bob", child == "Charlie" },
        all { parent == "Alice", child == "David" }
    }
}

rel grandparent(gp, gc) {
    |p| {
        parent(gp, p),
        parent(p, gc)
    }
}

@main
rel family_query(gp, gc) {
    grandparent(gp, gc)
}
```

```bash
$ cargo run -- family.pv
Query: family_query(gp, gc)
Solutions:
  1: gc = "Charlie", gp = "Alice"
```

## 📋 Language Overview

### Relations and Fresh Variables

```prolog
use std::list::*;

// Define relations
rel append(l, s, ls) {
    match [l, s, ls] {
        [[], x, x] => true,
        [[head | tail], list2, [head | result]] => {
            append(tail, list2, result)
        }
    }
}

// Use fresh variables with |var| syntax
rel example() {
    |x, y, result| {
        append([1, 2], [3, 4], result),
        result == [1, 2, 3, 4]
    }
}
```

### Structs and Implementation Blocks

```prolog
struct Point { x: Number, y: Number }

impl Point {
    rel new(x, y, result) {
        result == Point { x: x, y: y }
    }
    
    rel is_origin(self, result) {
        |x, y| {
            Point { x: x, y: y } == self,
            any {
                all { x == 0, y == 0, result == true },
                all { any { x != 0, y != 0 }, result == false }
            }
        }
    }
}

@main
rel point_demo() {
    |origin, is_at_origin| {
        Point::new(0, 0, origin),
        origin.is_origin(is_at_origin),
        is_at_origin == true
    }
}
```

### Pattern Matching with Guards

```prolog
rel classify_number(n, category) {
    match n {
        x if constraint(domain="clpfd") { x > 0 } => category == "positive",
        x if constraint(domain="clpfd") { x < 0 } => category == "negative",
        0 => category == "zero"
    }
}

@main
rel number_test(result) {
    classify_number(5, result)
}
```

### Constraint Programming

```prolog
// Simple constraint example
@main
rel constraint_demo(x, y, sum) {
    constraint(domain="clpfd") {
        x in 1..10,
        y in 5..15,
        sum == x + y,
        sum < 20
    }
}

// N-Queens puzzle
rel n_queens(n, solution) {
    |queens| {
        length(queens, n),
        constraint(domain="clpfd") {
            queens in 1..n,
            alldiff queens
        },
        solution == queens
    }
}
```

## 🛠️ CLI Usage

### Basic Execution

```bash
# Run a program
cargo run -- examples/easy-zebra.pv

# Execute specific queries  
cargo run -- --query "member(X, [1, 2, 3])" std/list.pv

# Limit output
cargo run -- --limit 5 --query "append(X, Y, [1, 2, 3])" std/list.pv
```

### Output Formats

```bash
# Table format
cargo run -- --format table examples/easy-zebra.pv

# JSON output
cargo run -- --format json --query "member(X, [a, b, c])" std/list.pv

# Numbered output (default)
cargo run -- --format numbered examples/easy-zebra.pv
```

### Debugging and Testing

```bash
# Enable tracing
cargo run -- --trace --query "append([1], [2], X)" std/list.pv

# Run tests
cargo run -- test
cargo run -- test --file tests/list_tests.pv
cargo run -- test --filter "member_*"
```

## 📖 Documentation

- **[📚 Programmer's Guide](docs/PROGRAMMER_GUIDE.md)** - Complete language reference with examples
- **[🔧 Development Guide](CLAUDE.md)** - Build instructions and development workflow

## 🧪 Testing Framework

Proto-Vulcan includes a powerful built-in test framework:

```prolog
use std::list::*;

// Test with expected results
@test(expected = [1, 2, 3])
rel test_member_basic(x) {
    member(x, [1, 2, 3])
}

// Test with assertions
@test
rel test_append() {
    |result| {
        append([1, 2], [3, 4], result),
        assert_eq(result, [1, 2, 3, 4])
    }
}

// Test constraints
@test
rel test_constraint() {
    |x| {
        constraint(domain="clpfd") {
            x in 1..10,
            x > 5,
            x < 8
        },
        any {
            x == 6,
            x == 7
        }
    }
}
```

## 🎯 Examples

### Logic Puzzles

The classic Einstein's Zebra puzzle:

```prolog
// examples/easy-zebra.pv
use std::list::*;

rel righto(x, y, l) {
    match l {
        [first, second | _] => {
            first == y,
            second == x
        },
        [_ | rest] => righto(x, y, rest)
    }
}

@main
rel easy_zebra(houses) {
    // Italian lives in the second house
    [_, ["italian", _], _] == houses,
    // Spanish lives right next to red house  
    righto(["spanish", _], [_, "red"], houses),
    // Norwegian lives in the blue house
    member(["norwegian", "blue"], houses)
}
```

### Constraint Solving

```bash
# Run constraint examples (requires clpfd feature)
cargo run --example sudoku --features clpfd
cargo run --example n-queens --features clpfd
```

## 🔧 Development

### Building

```bash
# Basic build
cargo build

# With constraint programming features
cargo build --features "clpfd,clpz"

# All features
cargo build --features "core,extras,clpfd,clpz,debugger"
```

### Testing

```bash
# Rust tests
cargo test

# Proto-Vulcan language tests
cargo run -- test

# Test specific features
cargo test --features clpfd
```

### Examples

```bash
# Run built-in examples
cargo run --example sudoku --features clpfd
cargo run --example n-queens --features clpfd

# Run .pv file examples  
cargo run -- examples/easy-zebra.pv
cargo run -- examples/medium-zebra.pv
```

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you shall be dual licensed as above, without any additional terms or conditions.

---

<div align="center">
  <b>🚀 Happy Logic Programming! 🚀</b>
</div>