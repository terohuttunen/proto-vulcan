# Proto-Vulcan Migration Guide

This guide helps you migrate your code from the old Proto-Vulcan macro syntax to the new interpreter-based syntax used in `.pv` files.

## Query Syntax Changes

The primary change is moving from the `proto_vulcan_query!` macro in Rust to writing logic directly in `.pv` files.

### Old Syntax (in Rust)

The old syntax was always wrapped in a `proto_vulcan_query!` macro call inside a Rust file.

```rust
// Old query to find a member of a list
let query = proto_vulcan_query!(|q| {
    |x, y, z| {
        member(q, [x, y, z]),
        x == 1,
        y == 2,
        z == 3
    }
});

// To get results, you would run the query and iterate:
let mut iter = query.run();
assert_eq!(iter.next().unwrap().q, lterm!(1));
assert_eq!(iter.next().unwrap().q, lterm!(2));
assert_eq!(iter.next().unwrap().q, lterm!(3));
```

Key features of the old syntax:
-   Queries are defined via the `proto_vulcan_query!` macro.
-   Top-level variables (`q` in this case) are the ones you can inspect in the results.
-   Fresh logic variables are introduced with the `|x, y, z| { ... }` syntax.
-   Goals are comma-separated.

### New Syntax (in `.pv` files)

The new syntax lives in `.pv` files and is much cleaner. The equivalent logic would be defined in a `.pv` file, often as a relation.

```prolog
// new_file.pv
rel my_query(q) {
    |x, y, z| {
        member(q, [x, y, z]),
        x == 1,
        y == 2,
        z == 3
    }
}
```

Key features of the new syntax:
-   No `proto_vulcan_query!` macro. Logic is written directly.
-   Logic is typically encapsulated in relations (`rel`).
-   The syntax for fresh variables and goals remains very similar.

## Logical Operators: Conjunction and Disjunction

The way to express AND and OR logic has been made more explicit in the new syntax with the introduction of `all` and `any` blocks.

### Old Syntax (in Rust)

-   **Conjunction (AND)**: Goals were implicitly joined by an AND when listed sequentially with commas. This was often done inside brackets `[...]` within a `conde` block.
-   **Disjunction (OR)**: The `conde` operator was used for disjunction. Each top-level argument to `conde` represented a separate OR branch.

```rust
// Old syntax example
let query = proto_vulcan_query!(|q| {
    conde {
        // Branch 1: q must be 1 AND a member of [1, 2]
        [q == 1, member(q, [1, 2])],

        // Branch 2: OR q must be 3
        q == 3
    }
});
// This query will yield q=1 and then q=3.
```

### New Syntax (in `.pv` files)

The new syntax provides clearer, dedicated blocks for these operations.

-   **Conjunction (AND)**: The `all { ... }` block explicitly groups goals that must all be true.
-   **Disjunction (OR)**: The `any { ... }` block explicitly groups goals where any one can be true. While `conde` still exists for backward compatibility, `any` is preferred.

```prolog
// new_file.pv
rel my_logical_query(q) {
    any {
        // Branch 1
        all {
            q == 1,
            member(q, [1, 2])
        },

        // Branch 2
        all {
            q == 3
        }
    }
}
```

The new `all`/`any` syntax improves readability and removes ambiguity compared to the old structure.

## Pattern Matching

The core logic of pattern matching is the same, but the syntax in the new interpreter is cleaner and more aligned with native Rust.

### Old Syntax (`matche`)

In the old macro-based system, `matche` was a common way to do pattern matching, which behaved like a disjunction (`conde`).

```rust
// From src/relation/member1.rs
pub fn member1<U, E, G>(x: LTerm<U, E>, l: LTerm<U, E>) -> InferredGoal<G>
where
    U: User,
    E: Engine<U>,
    G: AnyGoal<U, E>,
{
    proto_vulcan_closure!(match l {
        [head | _] => head == x,
        [head | rest] => [head != x, member1(x, rest)],
    })
}
```
*Note: The example above uses `proto_vulcan_closure!`, but pattern matching could also appear inside `proto_vulcan_query!`.*

### New Syntax (`match`)

In `.pv` files, you use a `match` statement that is syntactically closer to Rust's `match`.

```prolog
// The equivalent of the member1 relation
rel member1(x, l) {
    match l {
        [head | _] => { head == x },
        [head | rest] => {
            head != x,
            member1(x, rest)
        }
    }
}
```

Key differences:
-   The new `match` statement is a keyword, not a macro-based operator.
-   The arms of the match statement `=>` must be followed by a goal block `{ ... }`.

## Testing Changes

The old testing methodology was fundamentally different from the new one. The previous version of this guide was incorrect on this topic.

### Old Testing Method (in Rust)

Old tests were standard Rust tests (`#[test]`). They would:
1.  Define a query with `proto_vulcan_query!`.
2.  Execute the query with `.run()`.
3.  Iterate over the results.
4.  Use standard Rust `assert!` macros to verify each result.

```rust
// From src/relation/member1.rs
#[test]
fn test_member1_2() {
    let query = proto_vulcan_query!(|q| { member1(q, [1, 2, 3, 4, 5]) });
    let mut iter = query.run();
    assert_eq!(iter.next().unwrap().q, 1);
    assert_eq!(iter.next().unwrap().q, 2);
    assert_eq!(iter.next().unwrap().q, 3);
    assert_eq!(iter.next().unwrap().q, 4);
    assert_eq!(iter.next().unwrap().q, 5);
    assert!(iter.next().is_none());
}
```

### New Testing Method (in `.pv` files)

New tests are defined as relations within `.pv` files and are annotated with `@test`. They use built-in assertion goals.

```prolog
// in tests/language_features.pv
@test
rel test_member() {
    |x| {
        member(x, [1, 2, 3]),
        // The test runner will check all possible solutions for x
        // and expect assertions to hold for each.
        // This is a conceptual example; direct assertion on
        // multiple values requires a more complex structure.
    }
}

@test
rel test_simple_assertion() {
    assert_eq(1, 1),
    assert_neq("a", "b")
}
```

Key differences:
-   Tests move from Rust files to `.pv` files.
-   Tests are relations marked with `@test`.
-   Verification is done with assertion goals (`assert_eq`, `assert_neq`) inside the relation, not by iterating in Rust.
-   The test runner handles the execution and result checking automatically. 