# Proto-Vulcan Programmer's Guide

A comprehensive guide to programming in Proto-Vulcan, a relational logic programming language embedded in Rust.

## Table of Contents

1. [Introduction & Getting Started](#introduction--getting-started)
2. [Language Fundamentals](#language-fundamentals)
3. [Control Flow](#control-flow)
4. [Data Types](#data-types)
5. [Constraint Programming](#constraint-programming)
6. [Module System](#module-system)
7. [Testing System](#testing-system)
8. [Standard Library Reference](#standard-library-reference)
9. [Builtin Predicates](#builtin-predicates)
10. [Advanced Features](#advanced-features)
11. [CLI Reference](#cli-reference)
12. [Examples and Patterns](#examples-and-patterns)
13. [Appendices](#appendices)

---

## Introduction & Getting Started

### What is Proto-Vulcan?

Proto-Vulcan is a relational logic programming language inspired by [miniKanren](http://minikanren.org), but evolved into its own language with miniKanren at its core. It started as a Rust-embedded DSL but now includes a standalone interpreter that processes `.pv` files directly.

**Key Features:**
- Relational logic programming with unification and backtracking
- Constraint programming with CLP(FD) and CLP(Z) domains
- Pattern matching and algebraic data types
- Module system with imports and visibility controls
- Comprehensive testing framework
- Both breadth-first and depth-first search strategies
- Rich standard library for common operations

### Installation and Setup

Proto-Vulcan is built with Rust and requires a Rust toolchain to compile and run.

```bash
# Clone the repository
git clone https://github.com/terohuttunen/proto-vulcan
cd proto-vulcan

# Build the project
cargo build --release

# Run examples
cargo run --example sudoku --features clpfd
```

### Your First Program

Create a simple program in a `.pv` file:

```prolog
// hello.pv
use std::list::*;

@main
rel greet(message) {
    member(message, ["Hello", "World", "from", "Proto-Vulcan!"])
}
```

The `@main` attribute marks the entry point relation. When you run the file, Proto-Vulcan will execute this relation and display all its solutions.

### Running Programs

```bash
# Run a file with @main relation
cargo run -- hello.pv

# Run a specific query
cargo run -- --query "member(X, [1, 2, 3])" std/list.pv

# Control output format
cargo run -- --format table --limit 5 hello.pv
cargo run -- --format json --query "member(X, [1, 2, 3])" std/list.pv
```

---

## Language Fundamentals

### Relations

Relations are the fundamental building blocks of Proto-Vulcan programs. They define logical relationships between terms.

```prolog
// Basic relation definition
rel parent(parent, child) {
    any {
        all { parent == "Alice", child == "Bob" },
        all { parent == "Bob", child == "Charlie" },
        all { parent == "Alice", child == "David" }
    }
}

// Recursive relation
rel ancestor(ancestor, descendant) {
    any {
        // Base case: direct parent relationship
        parent(ancestor, descendant),
        
        // Recursive case: ancestor through intermediate
        |intermediate| {
            parent(ancestor, intermediate),
            ancestor(intermediate, descendant)
        }
    }
}
```

**Relation Syntax:**
- `rel name(param1, param2) { body }` - defines a relation
- `pub rel name(params) { body }` - public relation (exported from module)
- Parameters can be input, output, or bidirectional
- Body contains goals that define when the relation succeeds

### Goals

Goals are logical statements that can either succeed or fail. They're the building blocks within relation bodies.

**Types of Goals:**
- **Unification**: `X == Y` - succeeds if X and Y can be unified
- **Disequality**: `X != Y` - succeeds if X and Y cannot be unified
- **Relation calls**: `parent(X, "Bob")` - calls another relation
- **Compound goals**: `all { goal1, goal2 }`, `any { goal1, goal2 }`

```prolog
rel example_goals(x, y) {
    all {
        x == "Alice",           // Unification goal
        parent(x, y),           // Relation call goal
        y != "Unknown"          // Disequality goal
    }
}
```

### Variables

Proto-Vulcan has two types of variables:

1. **Relation Parameters**: Named in the relation signature
2. **Fresh Variables**: Introduced within goals using `|var1, var2| { ... }`

```prolog
rel grandparent(grandparent, grandchild) {
    |parent| {              // Fresh variable
        parent(grandparent, parent),
        parent(parent, grandchild)
    }
}

// Multiple fresh variables
rel complex_relation(result) {
    |x, y, z| {
        some_relation(x, y),
        another_relation(y, z),
        result == [x, y, z]
    }
}
```

**Variable Scoping Rules:**
- Relation parameters are visible throughout the relation body
- Fresh variables are scoped to their introducing block
- Variables can be unified multiple times within their scope
- Underscore `_` represents anonymous/don't-care variables

### Terms

Terms represent data in Proto-Vulcan:

```prolog
// Atoms (constants)
rel atoms_example() {
    x == 42,                    // Number
    y == "Hello",               // String  
    z == true                   // Boolean
}

// Compound terms
rel compound_example() {
    point == Point(10, 20),     // Struct
    list == [1, 2, 3],         // List
    nested == [Point(0, 0), Point(1, 1)]  // Nested structures
}
```

### Unification

Unification (`==`) is the core operation that matches and binds variables to values or structures.

```prolog
rel unification_examples() {
    // Simple unification
    X == 42,
    
    // List unification with head/tail
    [Head | Tail] == [1, 2, 3],  // Head = 1, Tail = [2, 3]
    
    // Struct unification
    Point(X, Y) == Point(10, 20), // X = 10, Y = 20
    
    // Partial unification
    [First, Second | _] == [a, b, c, d]  // First = a, Second = b
}
```

**Unification Rules:**
- Variables unify with any term (binding the variable)
- Identical constants unify with each other
- Compound terms unify if their components unify recursively
- Lists unify element-wise, including head/tail patterns
- Structs unify if they have the same constructor and fields unify

---

## Control Flow

### Conjunction (AND)

The `all` block requires all contained goals to succeed:

```prolog
rel conjunction_example(x, y) {
    all {
        x == "Alice",
        parent(x, y),
        y != "Unknown"
    }
    // All three goals must succeed
}

// Implicit conjunction - goals in sequence
rel implicit_conjunction(x, y) {
    x == "Alice",      // Goal 1
    parent(x, y),      // Goal 2  
    y != "Unknown"     // Goal 3
    // All goals must succeed in order
}
```

### Disjunction (OR)

The `any` block succeeds if any contained goal succeeds:

```prolog
rel disjunction_example(person) {
    any {
        person == "Alice",
        person == "Bob",
        person == "Charlie"
    }
    // Succeeds for any of the three values
}

// Multiple solutions
rel family_member(name) {
    any {
        parent(name, _),     // Anyone who is a parent
        parent(_, name)      // Anyone who is a child
    }
}
```

### Pattern Matching

The `match` statement provides structured disjunction with pattern testing:

```prolog
use std::list::*;

rel list_length(list, length) {
    match list {
        [] => length == 0,
        [_ | tail] => {
            |tail_length| {
                list_length(tail, tail_length),
                length == tail_length + 1
            }
        }
    }
}

// Multiple patterns
rel classify_list(list, classification) {
    match list {
        [] => classification == "empty",
        [_] => classification == "singleton", 
        [_, _] => classification == "pair",
        [_, _, _ | _] => classification == "long"
    }
}
```

**Match Statement Features:**
- Patterns can include literals, variables, and structure destructuring
- Guards can be added to patterns using `if` keyword
- Patterns are tried in order until one succeeds
- Each arm can contain complex goal blocks

#### Pattern Guards

Guards allow you to add additional conditions to pattern matches using the `if` keyword:

```prolog
// Basic guard with constraint
rel classify_number(n, classification) {
    match n {
        x if constraint(domain="clpfd") { x > 0 } => classification == "positive",
        x if constraint(domain="clpfd") { x < 0 } => classification == "negative", 
        0 => classification == "zero"
    }
}

// Guard with unification
rel check_value(value, result) {
    match value {
        n if n == 42 => result == "the answer",
        n if n == 0 => result == "zero",
        _ => result == "other"
    }
}

// Multiple guards on same pattern
rel temperature_range(temp, description) {
    match temp {
        t if constraint(domain="clpfd") { t > 30 } => description == "hot",
        t if constraint(domain="clpfd") { t > 15 } => description == "warm",
        t if constraint(domain="clpfd") { t > 0 } => description == "cool",
        _ => description == "freezing"
    }
}

// Complex guards with conjunction
rel validate_point(point, is_valid) {
    match point {
        [x, y] if all {
            constraint(domain="clpfd") { x > 0 },
            constraint(domain="clpfd") { y > 0 },
            constraint(domain="clpfd") { x + y < 100 }
        } => is_valid == true,
        _ => is_valid == false
    }
}

// Guards with fresh variables
rel check_doubled(n, is_large_when_doubled) {
    match n {
        x if |doubled| {
            constraint(domain="clpfd") { doubled == x * 2 },
            constraint(domain="clpfd") { doubled > 50 }
        } => is_large_when_doubled == true,
        _ => is_large_when_doubled == false
    }
}
```

**Guard Evaluation:**
- Guards are evaluated after pattern matching succeeds
- If a guard fails, the next pattern arm is tried
- Guards can use constraints, unification, and complex goal blocks
- Variables bound in the pattern are available in the guard
- Fresh variables can be introduced in guards using `|var| { ... }`

### Search Strategies

Proto-Vulcan supports both breadth-first (BFS) and depth-first (DFS) search:

```prolog
// Default behavior (BFS) - interleaves solutions
@test(expected = [1, 4, 2, 5, 3, 6])
rel bfs_example(x) {
    any {
        member(x, [1, 2, 3]),
        member(x, [4, 5, 6])
    }
}

// Depth-first search - exhausts first branch before second
@test(expected = [1, 2, 3, 4, 5, 6])
rel dfs_example(x) {
    any(strategy = dfs) {
        member(x, [1, 2, 3]),
        member(x, [4, 5, 6])
    }
}

// Relation-level strategy annotation
@test(expected = [1, 2, 3, 4, 5, 6])
rel dfs_relation(x) @dfs {
    any {
        member(x, [1, 2, 3]),
        member(x, [4, 5, 6])
    }
}
```

**Strategy Inheritance:**
- Relations without explicit strategy inherit from calling context
- `@bfs` and `@dfs` annotations override parent strategy
- Local `strategy = dfs` parameters take highest precedence

---

## Data Types

Proto-Vulcan has a comprehensive type system built around the core `LTerm` type, which serves as the supertype for all other types. Understanding this hierarchy is crucial for effective programming in Proto-Vulcan.

### Type Hierarchy

Proto-Vulcan's type system follows this hierarchy:

```
└─ LTerm (Logic Term - supertype of all types)
   ├─ Val (Literal values)
   │  ├─ Bool      (true, false)
   │  ├─ Number    (isize integers)
   │  ├─ Char      ('a', 'b', etc.)
   │  └─ String    ("hello", "world")
   ├─ Var (Variables)
   │  ├─ Named     (x, y, person_name)
   │  └─ Anonymous (_)
   ├─ Empty (Empty list [])
   ├─ Cons (Non-empty lists [head|tail])
   ├─ Compound (Structured data)
   │  ├─ Structs   (Point(x,y), Person{name,age})
   │  └─ Enums     (Color::Red, Shape::Circle(r))
   ├─ Projection (Projected variables)
   └─ RelationRef (Higher-order predicate references)
```

### Primitive Types (LValue)

#### Numbers
Proto-Vulcan uses `isize` integers for numeric computations:

```prolog
// Number literals and operations
rel number_examples() {
    positive == 42,
    negative == -17,
    zero == 0,
    large == 1000000
}

// Arithmetic works with constraint domains
rel arithmetic_example() {
    |x, y, sum| {
        constraint(domain="clpfd") {
            x in 1..10,
            y in 5..15,
            sum == x + y,
            sum < 20
        }
    }
}
```

#### Strings
Strings are UTF-8 encoded and support standard escape sequences:

```prolog
// String literals and patterns
rel string_examples() {
    simple == "Hello, World!",
    empty == "",
    with_quotes == "She said \"Hello\"",
    multiline == "Line 1\nLine 2\tTabbed",
    unicode == "Hello 🌍 World"
}

// String matching (exact)
rel string_matching(message) {
    match message {
        "hello" => "greeting",
        "goodbye" => "farewell",
        _ => "unknown"
    }
}
```

#### Booleans
Boolean values for logical operations:

```prolog
// Boolean literals and logic
rel boolean_examples() {
    truth == true,
    falsehood == false
}

// Boolean constraints
rel boolean_logic() {
    |p, q, result| {
        any {
            all { p == true, q == true, result == true },
            all { p == false, result == false },
            all { q == false, result == false }
        }
    }
}
```

#### Characters
Individual Unicode characters:

```prolog
// Character examples
rel char_examples() {
    letter == 'a',
    digit == '5',
    symbol == '€',
    space == ' '
}
```

### Variables

#### Named Variables
Variables that can be bound to values through unification:

```prolog
rel variable_examples() {
    // Simple binding
    X == 42,  // X is bound to 42
    
    // Multiple bindings must be consistent
    Y == "hello",
    Y == "hello",  // OK - same value
    
    // Pattern variables
    [Head | Tail] == [1, 2, 3],  // Head = 1, Tail = [2, 3]
}
```

#### Anonymous Variables
The underscore `_` represents "don't care" values:

```prolog
rel anonymous_examples() {
    // Ignore certain values
    [First, _, Third] == [1, 2, 3],  // First = 1, Third = 3
    
    // Each _ is independent
    Point(_, _) == Point(10, 20)  // Both _ can match different values
}
```

#### Fresh Variables
Variables introduced in limited scopes:

```prolog
rel fresh_variable_scoping() {
    // Variables scoped to this block
    |x, y, temp| {
        x == 10,
        y == 20,
        temp == x + y,
        result == temp
    }
    // x, y, temp not accessible outside the block
}
```

### Lists

Lists are fundamental data structures built from `Empty` and `Cons` constructors:

```prolog
// List construction
rel list_construction() {
    empty == [],                    // Empty list
    singleton == [42],              // Single element
    multiple == [1, 2, 3, 4],      // Multiple elements
    mixed == [1, "hello", true],    // Mixed types
    nested == [[1, 2], [3, 4]]     // Nested lists
}

// Head/tail patterns (Cons structure)
rel list_destructuring() {
    // Basic head/tail
    [Head | Tail] == [1, 2, 3],    // Head = 1, Tail = [2, 3]
    
    // Multiple elements
    [First, Second | Rest] == [a, b, c, d],  // First = a, Second = b, Rest = [c, d]
    
    // Empty tail
    [Only] == [42],                // Only = 42, implicit empty tail
}

// Improper lists (non-list tail)
rel improper_lists() {
    dotted == [1, 2 | "tail"],     // Tail is not a list
    partial == [a, b | X],         // X can be any term
}
```

**List Operations:**
```prolog
use std::list::*;

rel list_operations_example() {
    // Membership testing
    member(2, [1, 2, 3]),
    
    // List concatenation
    append([1, 2], [3, 4], [1, 2, 3, 4]),
    
    // List length
    length([a, b, c], 3),
    
    // List reversal
    reverse([1, 2, 3], [3, 2, 1])
}
```


### Compound Types

#### Structs
Structured data types with named fields or positional arguments:

**Tuple Structs:**
```prolog
// Declaration
struct Point(Number, Number);
struct Color(Number, Number, Number);
struct Line(Point, Point);

// Usage
rel tuple_struct_examples() {
    origin == Point(0, 0),
    red == Color(255, 0, 0),
    line == Line(Point(0, 0), Point(10, 10)),
    
    // Pattern matching
    Point(x, y) == Point(10, 20),          // x = 10, y = 20
    Color(r, _, b) == Color(128, 64, 32),  // r = 128, b = 32
    Line(start, Point(end_x, end_y)) == line  // Nested patterns
}
```

**Named Structs:**
```prolog
// Declaration
struct Person { name: String, age: Number }
struct Rectangle { width: Number, height: Number }
struct Company { name: String, employees: [Person] }

// Usage
rel named_struct_examples() {
    alice == Person { name: "Alice", age: 30 },
    rect == Rectangle { width: 100, height: 50 },
    
    // Field order independence
    same_person == Person { age: 30, name: "Alice" },  // Same as alice
    
    // Nested structures
    company == Company {
        name: "TechCorp",
        employees: [
            Person { name: "Alice", age: 30 },
            Person { name: "Bob", age: 25 }
        ]
    }
}
```

**Pattern Matching with Structs:**
```prolog
rel struct_pattern_matching() {
    // Tuple struct patterns
    match Point(10, 20) {
        Point(0, 0) => "origin",
        Point(x, 0) => "on x-axis",
        Point(0, y) => "on y-axis", 
        Point(x, y) => "general point"
    },
    
    // Named struct patterns
    match Person { name: "Alice", age: 30 } {
        Person { name: "Alice", age: a } => a,  // Extract age
        Person { age: 18, name: n } => n,       // Extract name of 18-year-old
        Person { name: name, age: _ } => name   // Extract any name
    }
}
```

#### Enums (Algebraic Data Types)

Proto-Vulcan supports enums for modeling data with multiple variants:

```prolog
// Simple enum
enum Color {
    Red,
    Green, 
    Blue
}

// Enum with data
enum Shape {
    Circle(Number),           // radius
    Rectangle(Number, Number), // width, height
    Triangle(Number, Number, Number) // sides
}

// Using enums
rel enum_examples() {
    primary == Color::Red,
    circle == Shape::Circle(5),
    rect == Shape::Rectangle(10, 20),
    
    // Pattern matching on enums
    match primary {
        Color::Red => "It's red!",
        Color::Green => "It's green!", 
        Color::Blue => "It's blue!"
    }
}
```

### Implementation Blocks (impl)

Proto-Vulcan supports implementation blocks that allow you to define methods on structs and enums. This provides an object-oriented style interface while maintaining the logic programming paradigm.

#### The `self` Parameter

Methods in implementation blocks use a `self` parameter to represent the instance they're called on. This parameter is automatically bound when using dot notation:

```prolog
struct Point {
    x: Number,
    y: Number
}

impl Point {
    // Constructor method (static - no self parameter)
    rel new(x, y, result) {
        result == Point { x: x, y: y }
    }
    
    // Instance method with self parameter
    rel distance_from_origin(self, dist) {
        |x, y| {
            Point { x: x, y: y } == self,
            // Simplified distance calculation
            dist == x
        }
    }
    
    // Check if point is at origin using self
    rel is_origin(self, result) {
        |x, y| {
            Point { x: x, y: y } == self,
            any {
                all {
                    x == 0,
                    y == 0,
                    result == true
                },
                all {
                    any { x != 0, y != 0 },
                    result == false
                }
            }
        }
    }
    
    // Method that validates the self parameter
    rel validate_self(self) {
        self == self  // Always succeeds
    }
    
    // Getter method using self
    rel get_x_alt(self, x) {
        |px, py| {
            Point { x: px, y: py } == self,
            x == px
        }
    }
    
    // Method with self and additional parameters
    rel translate(self, dx, dy, result) {
        |x, y| {
            Point { x: x, y: y } == self,
            // For simplicity, just return original point (no arithmetic constraints)
            result == Point { x: x, y: y }
        }
    }
    
    // Parameter-less method (for testing compilation)
    rel simple_test() {
        1 == 1
    }
}
```

#### Method Call Syntax

Proto-Vulcan supports multiple ways to call methods:

```prolog
rel method_call_examples() {
    // 1. Static method call (no self - like constructor)
    Point::new(3, 4, Point { x: 3, y: 4 }),
    
    // 2. Instance method call with dot notation
    // The Point instance becomes the 'self' parameter
    |point, dist| {
        point == Point { x: 5, y: 12 },
        point.distance_from_origin(dist),  // self = point
        dist == 5
    },
    
    // 3. Method chaining with dot notation on struct literals
    |result| {
        Point { x: 0, y: 0 }.is_origin(result),  // self = Point { x: 0, y: 0 }
        result == true
    },
    
    // 4. Explicit predicate call (alternative syntax)
    |point, x_coord| {
        point == Point { x: 5, y: 10 },
        Point::get_x_alt(point, x_coord),  // point becomes self parameter
        x_coord == 5
    },
    
    // 5. Simple validation
    Point { x: 5, y: 10 }.validate_self(),
    
    // 6. Parameter-less static method
    Point::simple_test()
}
```

#### Complex Implementation Example

```prolog
struct Rectangle {
    top_left: Point,
    width: Number,
    height: Number
}

impl Rectangle {
    // Constructor using Point methods
    rel new(x, y, w, h, result) {
        |point| {
            Point::new(x, y, point),
            result == Rectangle {
                top_left: point,
                width: w,
                height: h
            }
        }
    }
    
    // Calculate area using self (simplified)
    rel area(self, area) {
        |w, h| {
            Rectangle { top_left: _, width: w, height: h } == self,
            // For simplicity, just use width as area (no multiplication)
            area == w
        }
    }
    
    // Check if rectangle contains a point (simplified check)
    rel contains_point(self, point, result) {
        |tl, px, py, tlx, tly| {
            Rectangle { top_left: tl, width: _, height: _ } == self,
            Point { x: px, y: py } == point,
            Point { x: tlx, y: tly } == tl,
            // Simple check: point x must match top_left x (for testing)
            any {
                all {
                    px == tlx,
                    result == true
                },
                all {
                    px != tlx,
                    result == false
                }
            }
        }
    }
}
```

#### Method Usage Patterns from Tests

```prolog
// Based on actual test cases from impl_block_tests.pv

// Pattern 1: Constructor then method calls
@test
rel test_basic_method_call() {
    Point::new(3, 4, Point { x: 3, y: 4 })
}

// Pattern 2: Method on variable
@test
rel test_method_on_variable() {
    |dist| {
        Point { x: 5, y: 8 }.distance_from_origin(dist),
        dist == 5
    }
}

// Pattern 3: Method chaining
@test
rel test_method_chaining() {
    |result| {
        Point { x: 0, y: 0 }.is_origin(result),
        result == true
    }
}

// Pattern 4: Method with multiple arguments
@test
rel test_method_multiple_args() {
    |result| {
        Point { x: 5, y: 6 }.translate(2, 3, result),
        result == Point { x: 5, y: 6 }
    }
}

// Pattern 5: Nested struct method
@test  
rel test_nested_struct_method() {
    |area| {
        Rectangle { 
            top_left: Point { x: 1, y: 1 }, 
            width: 3, 
            height: 4 
        }.area(area),
        area == 3
    }
}

// Pattern 6: Method calls with pattern matching
@test
rel test_method_with_pattern_matching() {
    |input_point, result| {
        input_point == Point { x: 0, y: 0 },
        match input_point {
            Point { x: px, y: py } => {
                Point { x: px, y: py }.is_origin(result),
                result == true
            }
        }
    }
}
```

#### Implementation Blocks for Enums

```prolog
enum Shape {
    Circle(Number),
    Rectangle(Number, Number),
    Triangle(Number, Number, Number)
}

impl Shape {
    // Calculate area for any shape using self
    rel area(self, result) {
        match self {
            Shape::Circle(r) => {
                // Simplified area calculation
                result == r
            },
            Shape::Rectangle(w, h) => {
                result == w
            },
            Shape::Triangle(a, b, c) => {
                result == a
            }
        }
    }
    
    // Get shape type as string using self
    rel shape_type(self, type_name) {
        match self {
            Shape::Circle(_) => type_name == "circle",
            Shape::Rectangle(_, _) => type_name == "rectangle",
            Shape::Triangle(_, _, _) => type_name == "triangle"
        }
    }
    
    // Check if shape is a circle
    rel is_circle(self, result) {
        match self {
            Shape::Circle(_) => result == true,
            _ => result == false
        }
    }
}
```

#### Self Parameter Rules

**Key Rules for `self` Parameters:**

1. **First parameter**: The `self` parameter is always the first parameter in method definitions
2. **Automatic binding**: When using dot notation (`obj.method(args)`), `obj` automatically becomes the `self` parameter  
3. **Pattern matching**: You can pattern match on `self` just like any other parameter
4. **Naming**: While conventionally named `self`, you can use any name (like `point`, `rect`)
5. **Static methods**: Methods without a `self` parameter are static (like constructors)

```prolog
impl Point {
    // Static method - no self parameter
    rel origin(result) {
        result == Point { x: 0, y: 0 }
    }
    
    // Instance method - self parameter required
    rel magnitude_squared(self, mag) {
        |x, y| {
            Point { x: x, y: y } == self,
            // Simplified: just return x coordinate (no arithmetic)
            mag == x
        }
    }
    
    // Self can be pattern matched directly in parameter
    rel is_positive_quadrant(Point { x: px, y: py }, result) {
        any {
            all { 
                px == 0, 
                py == 0, 
                result == false 
            },
            result == true
        }
    }
    
    // Alternative parameter name for self
    rel get_coordinates(point, coords) {
        |x, y| {
            Point { x: x, y: y } == point,  // 'point' acts as self
            coords == [x, y]
        }
    }
}
```

**Key Features of Implementation Blocks:**
- **Method organization**: Group related predicates with their data types
- **Self parameter**: The `self` parameter represents the instance being operated on
- **Dot notation**: `obj.method()` automatically passes `obj` as the `self` parameter  
- **Static methods**: Methods without `self` for constructors and utilities
- **Pattern matching on self**: Can destructure `self` in method parameters
- **Integration**: Methods work seamlessly with unification, pattern matching, and constraints

---

## Constraint Programming

Proto-Vulcan includes powerful constraint programming capabilities through CLP(FD) and CLP(Z) domains.

### CLP(FD) - Finite Domain Constraints

CLP(FD) works with variables that range over finite sets of integers:

```prolog
// Basic domain constraints
rel basic_clpfd_example(x, y) {
    constraint(domain="clpfd") {
        x in 1..10,           // x must be between 1 and 10
        y in 0..5,            // y must be between 0 and 5
        x > y                 // x must be greater than y
    }
}

// Multiple variables with same domain
rel multiple_variables(result) {
    |x, y, z| {
        constraint(domain="clpfd") {
            [x, y, z] in 1..3,    // All variables in range 1..3
            alldiff [x, y, z]     // All must be different
        },
        result == [x, y, z]
    }
}
```

#### Arithmetic Constraints
```prolog
rel arithmetic_constraints(result) {
    |x, y, sum, diff, prod| {
        constraint(domain="clpfd") {
            x in 1..10,
            y in 1..10,
            sum in 5..15,
            
            x + y == sum,         // Addition
            x - y == diff,        // Subtraction  
            x * y == prod,        // Multiplication
            x > y                 // Comparison
        },
        result == [x, y, sum, diff, prod]
    }
}
```

#### All-Different Constraint
```prolog
// N-Queens problem fragment
rel queens_partial(result) {
    |q1, q2, q3| {
        constraint(domain="clpfd") {
            [q1, q2, q3] in 1..3,
            alldiff [q1, q2, q3]    // No two queens in same column
        },
        result == [q1, q2, q3]
    }
}
```

#### Comparison Constraints
```prolog
rel comparison_constraints(x, y) {
    constraint(domain="clpfd") {
        x in 1..100,
        y in 1..100,
        
        x < 50,         // Less than
        y <= 75,        // Less than or equal  
        x > 10,         // Greater than
        y >= 25,        // Greater than or equal
        x != y          // Not equal
    }
}
```

### CLP(Z) - Integer Arithmetic

CLP(Z) provides constraints over the full range of integers:

```prolog
rel clpz_example(x, y) {
    constraint(domain="clpz") {
        x + y > 100,
        x - y < 50,
        x * 2 == y + 10
    }
}
```

### Sudoku Example

Here's a simplified Sudoku constraint setup:

```prolog
rel sudoku_constraints(grid) {
    |row1, row2, row3| {
        // Grid structure
        grid == [row1, row2, row3],
        
        constraint(domain="clpfd") {
            // All cells in range 1..9
            [row1, row2, row3] in 1..9,
            
            // Row constraints - all different in each row
            alldiff row1,
            alldiff row2, 
            alldiff row3,
            
            // Column constraints would require more complex setup
            // Box constraints would require more complex setup
        }
    }
}
```

**Constraint Programming Benefits:**
- Efficient pruning of impossible values
- Automatic constraint propagation
- Suitable for combinatorial problems
- Natural expression of mathematical relationships

---

## Module System

Proto-Vulcan includes a comprehensive module system for organizing code:

### Module Declaration

```prolog
// geometry.pv
mod geometry {
    struct Point(Number, Number);
    struct Rectangle { width: Number, height: Number }
    
    pub rel distance(p1, p2, dist) {
        |Point(x1, y1), Point(x2, y2), dx, dy| {
            p1 == Point(x1, y1),
            p2 == Point(x2, y2),
            dx == x2 - x1,
            dy == y2 - y1,
            dist == sqrt(dx * dx + dy * dy)
        }
    }
    
    rel area(shape, area) {
        match shape {
            Rectangle { width: w, height: h } => area == w * h
        }
    }
}
```

### Imports

```prolog
// main.pv
use std::list::*;              // Import all from std::list
use geometry::{Point, distance}; // Import specific items
use std::env::args;            // Import single item

@main
rel main() {
    |points, d| {
        points == [Point(0, 0), Point(3, 4)],
        distance(Point(0, 0), Point(3, 4), d),
        println!("Distance: {}", d)
    }
}
```

### Visibility Control

```prolog
mod my_module {
    // Public - exported from module
    pub rel public_relation(x, y) {
        x == y
    }
    
    // Private - only visible within module
    rel private_helper(x) {
        x != 0
    }
    
    pub rel uses_private(x) {
        private_helper(x)  // OK - same module
    }
}

// Outside the module
rel external_code() {
    my_module::public_relation(1, 1),    // OK - public
    // my_module::private_helper(1)      // ERROR - private
}
```

### Standard Library

The standard library provides common functionality:

#### std::list
```prolog
use std::list::*;

rel list_example() {
    member(2, [1, 2, 3]),
    append([1, 2], [3, 4], [1, 2, 3, 4]),
    length([a, b, c], 3),
    reverse([1, 2, 3], [3, 2, 1])
}
```

#### std::env  
```prolog
use std::env::*;

rel env_example() {
    |home, args| {
        env_var("HOME", home),
        argv(args),
        has_flag("--verbose")
    }
}
```

#### std::fs
```prolog
use std::fs::*;

rel file_example() {
    |content| {
        read_file("config.txt", content),
        write_file("output.txt", "Hello, World!", _)
    }
}
```

### Module Resolution

```prolog
// Absolute imports
use std::list::member;          // From standard library
use crate::geometry::Point;     // From current crate root
use super::utils::helper;       // From parent module

// Relative imports
use geometry::Point;            // Relative to current scope
use self::nested::item;         // From current module's nested submodule
```

---

## Testing System

Proto-Vulcan includes a comprehensive testing framework built into the language:

### Test Annotations

```prolog
// Basic test - must succeed
@test
rel test_basic_success() {
    assert_eq(1, 1),
    assert_neq(1, 2)
}

// Test that should fail
@test(should_fail)
rel test_expected_failure() {
    assert_eq(1, 2)  // This will fail, but that's expected
}

// Test with expected results
@test(expected = [1, 2, 3])
rel test_with_expected_results(x) {
    member(x, [1, 2, 3])
}

// Test with complex expected results
@test(expected = [
    [1, [2, 3]],
    [2, [1, 3]], 
    [3, [1, 2]]
])
rel test_complex_results(result) {
    |x, others| {
        member(x, [1, 2, 3]),
        rember(x, [1, 2, 3], others),
        result == [x, others]
    }
}
```

### Assertion Predicates

```prolog
// Equality assertions
@test
rel test_assertions() {
    assert_eq(42, 42),           // Values must be equal
    assert_neq("a", "b"),        // Values must not be equal
    
    assert_bound(42),            // Value must be bound (not a variable)
    assert_unbound(_),           // Value must be unbound (a variable)
}

// Domain size assertion (for constraint variables)
@test
rel test_domain_assertion() {
    |x| {
        constraint(domain="clpfd") {
            x in 1..10
        },
        assert_domain_size(x, 10)  // Domain has 10 possible values
    }
}
```

### Running Tests

```bash
# Run all tests
cargo run -- test

# Run tests from specific file
cargo run -- test --file examples/zebra.pv

# Run tests matching pattern
cargo run -- test --filter "list_*"

# Run with timeout
cargo run -- test --timeout 30

# Show detailed output
cargo run -- test --verbose

# Run specific test by name
cargo run -- test --name test_member_basic
```

### Test Organization

```prolog
// tests/list_tests.pv
use std::list::*;

// Group related tests together
@test(expected = [])
rel test_member_empty_list(result) {
    member(result, [])
}

@test(expected = [1, 2, 3])  
rel test_member_basic(x) {
    member(x, [1, 2, 3])
}

@test
rel test_append_empty() {
    append([], [1, 2], [1, 2])
}

@test(expected = [[[], [1, 2, 3]], [[1], [2, 3]], [[1, 2], [3]], [[1, 2, 3], []]])
rel test_append_decomposition(result) {
    |left, right| {
        append(left, right, [1, 2, 3]),
        result == [left, right]
    }
}
```

### Test-Driven Development

```prolog
// Write tests first
@test(expected = [120])
rel test_factorial(result) {
    factorial(5, result)
}

// Then implement
rel factorial(n, result) {
    match n {
        0 => result == 1,
        _ => {
            |n_minus_1, sub_result| {
                constraint(domain="clpz") {
                    n > 0,
                    n_minus_1 == n - 1
                },
                factorial(n_minus_1, sub_result),
                result == n * sub_result
            }
        }
    }
}
```

---

## Standard Library Reference

### std::list - List Operations

The list module provides fundamental list manipulation predicates:

#### Core List Predicates

```prolog
use std::list::*;

// Test membership
rel member_examples() {
    member(2, [1, 2, 3]),        // succeeds
    // member(4, [1, 2, 3])      // fails
}

// Append lists
rel append_examples() {
    append([1, 2], [3, 4], [1, 2, 3, 4]),
    
    // Backward mode - find decompositions
    |left, right| {
        append(left, right, [1, 2, 3])
        // Produces: [[], [1,2,3]], [[1], [2,3]], [[1,2], [3]], [[1,2,3], []]
    }
}

// List construction/deconstruction  
rel list_ops() {
    cons(1, [2, 3], [1, 2, 3]),     // Prepend element
    first([1, 2, 3], 1),             // Get first element
    rest([1, 2, 3], [2, 3])          // Get all but first
}

// Remove element
rel remove_examples() {
    rember(2, [1, 2, 3], [1, 3]),   // Remove first occurrence
    
    // Remove from different positions
    rember(1, [1, 2, 1, 3], [2, 1, 3])  // Removes first 1
}
```

#### List Properties and Transformations

```prolog
// List length
rel length_examples() {
    length([a, b, c, d], 4),
    
    // Generate lists of specific length
    |list| {
        length(list, 3),
        member(1, list)
        // Generates lists of length 3 containing 1
    }
}

// List reversal
rel reverse_examples() {
    reverse([1, 2, 3], [3, 2, 1]),
    
    // Palindrome check
    |list| {
        reverse(list, list)  // List equals its reverse
    }
}

// Check if all elements are distinct
rel distinct_examples() {
    distinct([1, 2, 3]),        // succeeds
    // distinct([1, 2, 2])      // fails
}

// Permutation relationship
rel permute_examples() {
    permute([1, 2, 3], [3, 1, 2]),  // succeeds
    
    // Generate all permutations
    |perm| {
        permute([a, b, c], perm)
        // Generates all 6 permutations
    }
}
```

### std::env - Environment Access

```prolog
use std::env::*;

// Environment variables
rel env_var_examples() {
    |home, path| {
        env_var("HOME", home),              // Get HOME directory
        env_var("PATH", path),              // Get PATH
        env_var_exists("USER")              // Check if USER is set
    }
}

// Command line arguments  
rel command_line_examples() {
    |all_args, just_args, prog_name| {
        argv(all_args),                     // All args including program name
        args(just_args),                    // Just the arguments
        program_name(prog_name),            // Just the program name
        
        has_flag("--verbose"),              // Check for specific flag
        arg_count(3)                        // Expect exactly 3 arguments
    }
}

// System directories
rel system_dirs() {
    |current, home, temp| {
        current_dir(current),               // Current working directory
        home_dir(home),                     // User's home directory  
        temp_dir(temp)                      // System temp directory
    }
}
```

### std::fs - File System Operations

```prolog
use std::fs::*;

// File reading
rel file_read_examples() {
    |content, lines, bytes| {
        read_file("input.txt", content),        // Read as string
        read_file_lines("input.txt", lines),    // Read as line list
        read_file_bytes("data.bin", bytes)      // Read as byte list
    }
}

// File writing
rel file_write_examples() {
    write_file("output.txt", "Hello!", _),
    write_file_lines("list.txt", ["line1", "line2"], _),
    append_file("log.txt", "New entry\n", _)
}

// File operations
rel file_ops_examples() {
    |size| {
        file_exists("input.txt"),           // Check existence
        is_file("input.txt"),               // Check if it's a file
        is_dir("folder"),                   // Check if it's a directory
        file_size("input.txt", size),       // Get file size
        
        copy_file("src.txt", "dst.txt", _), // Copy file
        rename_file("old.txt", "new.txt", _) // Rename/move file
    }
}

// Directory operations
rel dir_ops_examples() {
    |entries| {
        create_dir("new_folder", _),        // Create directory
        read_dir("folder", entries),        // List directory contents
        remove_dir("empty_folder", _)       // Remove empty directory
    }
}
```

### std::path - Path Manipulation

```prolog
use std::path::*;

rel path_examples() {
    |full_path, parent, filename| {
        join_path("/home/user", "document.txt", full_path),
        parent_path(full_path, parent),
        file_name(full_path, filename),
        
        is_absolute_path("/home/user"),     // Check if absolute
        canonicalize("../file.txt", _)      // Resolve relative path
    }
}
```

---

## Builtin Predicates

Proto-Vulcan provides builtin predicates that interface with the underlying system:

### Core Language Builtins

```prolog
// Length calculation (efficient implementation)
rel length_builtin_example() {
    __builtin_length([1, 2, 3, 4], 4),
    
    |list, n| {
        __builtin_length(list, n),
        n > 0  // Generate non-empty lists
    }
}
```

### Testing Builtins

```prolog
// Built into the test framework
@test
rel builtin_assertions() {
    assert_eq(42, 42),              // Equality assertion
    assert_neq("a", "b"),           // Inequality assertion  
    assert_bound("value"),          // Check if term is bound
    assert_unbound(_),              // Check if term is unbound
}

// Constraint domain assertions
@test  
rel constraint_assertions() {
    |x| {
        constraint(domain="clpfd") {
            x in 1..10
        },
        assert_domain_size(x, 10)   // Check constraint domain size
    }
}
```

### File System Builtins

These are wrapped by the std::fs module but can be used directly:

```prolog
// Direct builtin usage (prefer std::fs wrappers)
rel filesystem_builtins() {
    __builtin_read_file("file.txt", _),
    __builtin_write_file("output.txt", "content", _),
    __builtin_file_exists("test.txt"),
    __builtin_create_dir("new_dir", _)
}
```

### Environment Builtins  

```prolog
// Direct builtin usage (prefer std::env wrappers)
rel environment_builtins() {
    __builtin_env_var("HOME", _),
    __builtin_current_dir(_),
    __builtin_argv(_)
}
```

### String and Path Builtins

```prolog
rel string_path_builtins() {
    __builtin_string_concat("hello", " world", "hello world"),
    __builtin_join_path("/home", "user", "/home/user"),
    __builtin_canonicalize("../file.txt", _)
}
```

### When to Use Builtins Directly

**Prefer standard library wrappers** in most cases:
- Better documentation and examples
- Consistent error handling patterns  
- More idiomatic Proto-Vulcan code

**Use builtins directly when:**
- Maximum performance is critical
- Building your own library abstractions
- The standard library wrapper doesn't exist yet

### Creating Custom Builtins

Builtins are implemented in Rust and registered with the interpreter. See `src/interpreter/builtins/` for examples.

---

## Advanced Features

### Command Line Argument Processing

Proto-Vulcan programs can access and process command line arguments:

```prolog
use std::env::*;

@main
rel main() {
    |args, prog_name| {
        program_name(prog_name),
        args(args),
        
        any {
            // Handle help flag
            all {
                has_flag("--help"),
                show_help(prog_name)
            },
            
            // Handle verbose flag
            all {
                has_flag("--verbose"),
                process_with_verbose(args)
            },
            
            // Default processing
            process_args(args)
        }
    }
}

rel show_help(prog_name) {
    println!("Usage: {} [options] [files...]", prog_name),
    println!("Options:"),
    println!("  --help     Show this help"),
    println!("  --verbose  Enable verbose output")
}

rel process_with_verbose(args) {
    println!("Verbose mode enabled"),
    println!("Processing {} arguments", length(args)),
    process_args(args)
}

rel process_args(args) {
    |file| {
        member(file, args),
        println!("Processing file: {}", file)
    }
}
```

### Meta-programming

Working with terms as data structures:

```prolog
// Inspect term structure
rel term_analysis(term, info) {
    any {
        // Check if term is a variable
        all {
            is_var(term),
            info == "variable"
        },
        
        // Check if term is atomic
        all {
            is_atom(term),
            info == "atom"
        },
        
        // Check if term is compound
        all {
            is_compound(term),
            compound_info(term, info)
        }
    }
}

rel compound_info(compound, info) {
    match compound {
        Point(x, y) => info == ["point", x, y],
        [head | tail] => info == ["list", head, tail],
        _ => info == "unknown_compound"
    }
}
```

### Higher-Order Predicates

Predicates that operate on other predicates:

```prolog
use std::higher_order::*;

// Apply predicate to all elements
rel map_example() {
    |doubled| {
        map(double, [1, 2, 3], doubled)  // [2, 4, 6]
    }
}

rel double(x, result) {
    result == x * 2
}

// Filter elements
rel filter_example() {
    |evens| {
        filter(is_even, [1, 2, 3, 4, 5, 6], evens)  // [2, 4, 6]
    }
}

rel is_even(n) {
    |half| {
        constraint(domain="clpz") {
            n == half * 2
        }
    }
}

// Fold/reduce
rel fold_example() {
    |sum| {
        foldl(add, 0, [1, 2, 3, 4], sum)  // 10
    }
}

rel add(x, y, result) {
    result == x + y
}
```

### Performance Considerations

#### Tail Recursion

Write recursive relations in tail-recursive form when possible:

```prolog
// Non-tail recursive (can cause stack overflow)
rel factorial_bad(n, result) {
    match n {
        0 => result == 1,
        _ => {
            |n_minus_1, sub_result| {
                n_minus_1 == n - 1,
                factorial_bad(n_minus_1, sub_result),
                result == n * sub_result  // Not tail position
            }
        }
    }
}

// Tail recursive (better performance)
rel factorial_good(n, result) {
    factorial_acc(n, 1, result)
}

rel factorial_acc(n, acc, result) {
    match n {
        0 => result == acc,
        _ => {
            |n_minus_1, new_acc| {
                n_minus_1 == n - 1,
                new_acc == n * acc,
                factorial_acc(n_minus_1, new_acc, result)  // Tail position
            }
        }
    }
}
```

#### Constraint Programming Efficiency

Place most restrictive constraints first:

```prolog
// Less efficient - generates many possibilities then filters
rel inefficient_constraints(x, y, z) {
    constraint(domain="clpfd") {
        [x, y, z] in 1..100,        // Huge search space
        x + y + z == 10,            // Then restrict
        x < y,
        y < z
    }
}

// More efficient - restrict early
rel efficient_constraints(x, y, z) {
    constraint(domain="clpfd") {
        [x, y, z] in 1..10,         // Smaller initial domain
        x < y,                      // Apply ordering early
        y < z,
        x + y + z == 10            // Then apply sum constraint
    }
}
```

#### Mode Directives (Future Feature)

```prolog
// Future syntax for specifying input/output modes
rel append(+list1, +list2, -result) {
    // list1 and list2 are inputs (+)
    // result is output (-)
    // Enables compiler optimizations
}
```

---

## CLI Reference

The Proto-Vulcan CLI provides comprehensive options for running programs and tests:

### Basic Execution

```bash
# Run a program with @main relation
cargo run -- program.pv

# Run specific query
cargo run -- --query "member(X, [1, 2, 3])" std/list.pv

# Pass arguments to the program
cargo run -- program.pv arg1 arg2 --flag value
```

### Output Formatting

```bash
# Numbered output (default)
cargo run -- --format numbered examples/zebra.pv

# Table format
cargo run -- --format table --query "append(X, Y, [1, 2])" std/list.pv

# JSON output
cargo run -- --format json --query "member(X, [1, 2, 3])" std/list.pv

# Raw output (no formatting)
cargo run -- --format raw program.pv
```

### Result Limiting

```bash
# Limit to 10 results
cargo run -- --limit 10 --query "append(X, Y, [1, 2, 3, 4])" std/list.pv

# Unlimited results (default)
cargo run -- --limit 0 program.pv

# Get just the first result
cargo run -- --limit 1 --query "member(X, [a, b, c])" std/list.pv
```

### Debugging and Tracing

```bash
# Enable basic tracing
cargo run -- --trace --query "member(X, [1, 2, 3])" std/list.pv

# Detailed tracing levels
cargo run -- --trace --trace-level 1 program.pv  # Basic
cargo run -- --trace --trace-level 2 program.pv  # Medium (default)  
cargo run -- --trace --trace-level 3 program.pv  # Detailed

# Trace with specific query
cargo run -- --trace --query "append(X, Y, [1, 2])" std/list.pv
```

### Color Control

```bash
# Automatic color detection (default)
cargo run -- --color auto program.pv

# Force colors on
cargo run -- --color always program.pv

# Force colors off
cargo run -- --color never program.pv
```

### Timeout Control

```bash
# Set execution timeout (in seconds)
cargo run -- --timeout 30 program.pv

# No timeout (default)
cargo run -- --timeout 0 program.pv

# Short timeout for testing
cargo run -- --timeout 5 --query "some_complex_query(X)" program.pv
```

### Error Handling

```bash
# Strict mode - treat warnings as errors
cargo run -- --strict program.pv

# Control specific warnings
cargo run -- --warn-shadowing true program.pv
cargo run -- --warn-unused-imports false program.pv
```

### Testing

```bash
# Run all tests
cargo run -- test

# Test specific file
cargo run -- test --file tests/list_tests.pv

# Test with pattern matching
cargo run -- test --filter "member_*"
cargo run -- test --filter "*_empty_*"

# Test with timeout
cargo run -- test --timeout 60

# Verbose test output
cargo run -- test --verbose

# Run specific test
cargo run -- test --name test_member_basic

# Parallel test execution
cargo run -- test --parallel 4

# Stop on first failure
cargo run -- test --fail-fast
```

### Syntax Checking

```bash
# Check syntax without running
cargo run -- check program.pv

# Show AST structure
cargo run -- check --show-ast program.pv

# Check multiple files
cargo run -- check file1.pv file2.pv file3.pv
```

### Examples

```bash
# Complete examples combining options
cargo run -- --format table --limit 20 --color always --trace \
              --query "permute([1,2,3], P)" std/list.pv

cargo run -- test --file examples/zebra.pv --verbose --timeout 30

cargo run -- --format json --query "member(X, [a,b,c])" --limit 1 std/list.pv
```

---

## Examples and Patterns

### Classic Logic Puzzles

#### The Zebra Puzzle (Einstein's Riddle)

```prolog
// examples/zebra.pv
use std::list::*;

// Helper relation for "right next to" logic
rel righto(x, y, l) {
    match l {
        [first, second | _] => {
            first == y,
            second == x
        },
        [_ | rest] => righto(x, y, rest)
    }
}

// Main zebra puzzle relation
rel zebra_puzzle(houses) {
    // Setup: 5 houses with [nationality, color, drink, smoke, pet] structure
    houses == [house1, house2, house3, house4, house5],
    
    // The Englishman lives in the red house
    member(["english", "red", _, _, _], houses),
    
    // The Spaniard owns the dog
    member(["spanish", _, _, _, "dog"], houses),
    
    // Coffee is drunk in the green house
    member([_, "green", "coffee", _, _], houses),
    
    // The Ukrainian drinks tea
    member(["ukrainian", _, "tea", _, _], houses),
    
    // The green house is immediately to the right of the ivory house
    righto([_, "green", _, _, _], [_, "ivory", _, _, _], houses),
    
    // The Old Gold smoker owns snails
    member([_, _, _, "old_gold", "snails"], houses),
    
    // Kools are smoked in the yellow house
    member([_, "yellow", _, "kools", _], houses),
    
    // Milk is drunk in the middle house
    house3 == [_, _, "milk", _, _],
    
    // The Norwegian lives in the first house
    house1 == ["norwegian", _, _, _, _],
    
    // The man who smokes Chesterfields lives next to the man with the fox
    |chesterfield_house, fox_house| {
        member(chesterfield_house, houses),
        member(fox_house, houses),
        chesterfield_house == [_, _, _, "chesterfields", _],
        fox_house == [_, _, _, _, "fox"],
        righto(chesterfield_house, fox_house, houses)
    },
    
    // Kools are smoked in the house next to the house with the horse
    |kools_house, horse_house| {
        member(kools_house, houses),
        member(horse_house, houses),
        kools_house == [_, _, _, "kools", _],
        horse_house == [_, _, _, _, "horse"],
        righto(kools_house, horse_house, houses)
    },
    
    // The Lucky Strike smoker drinks orange juice
    member([_, _, "orange_juice", "lucky_strike", _], houses),
    
    // The Japanese smokes Parliaments
    member(["japanese", _, _, "parliaments", _], houses),
    
    // The Norwegian lives next to the blue house
    |norwegian_house, blue_house| {
        member(norwegian_house, houses),
        member(blue_house, houses),
        norwegian_house == ["norwegian", _, _, _, _],
        blue_house == [_, "blue", _, _, _],
        righto(norwegian_house, blue_house, houses)
    }
}

@main
rel solve_zebra(houses) {
    zebra_puzzle(houses)
}
```

#### N-Queens Problem

```prolog
// N-Queens constraint satisfaction
rel n_queens(n, solution) {
    |queens| {
        length(queens, n),
        constraint(domain="clpfd") {
            queens in 1..n,
            alldiff queens,              // No two queens in same column
            no_diagonal_attacks(queens)
        },
        solution == queens
    }
}

rel no_diagonal_attacks(queens) {
    no_diagonal_attacks_acc(queens, 1)
}

rel no_diagonal_attacks_acc(queens, row) {
    match queens {
        [] => true,
        [queen | rest] => {
            no_attacks_from_position(queen, row, rest, row + 1),
            no_diagonal_attacks_acc(rest, row + 1)
        }
    }
}

rel no_attacks_from_position(queen_col, queen_row, other_queens, other_row) {
    match other_queens {
        [] => true,
        [other_col | rest] => {
            |row_diff, col_diff| {
                constraint(domain="clpz") {
                    row_diff == other_row - queen_row,
                    col_diff == other_col - queen_col,
                    row_diff != col_diff,      // Not on positive diagonal
                    row_diff != -col_diff      // Not on negative diagonal
                },
                no_attacks_from_position(queen_col, queen_row, rest, other_row + 1)
            }
        }
    }
}
```

### List Processing Patterns

#### Functional-Style List Operations

```prolog
use std::list::*;

// Map operation over lists
rel map(predicate, input_list, output_list) {
    match input_list {
        [] => output_list == [],
        [head | tail] => {
            |mapped_head, mapped_tail| {
                call(predicate, head, mapped_head),
                map(predicate, tail, mapped_tail),
                output_list == [mapped_head | mapped_tail]
            }
        }
    }
}

// Filter operation
rel filter(predicate, input_list, output_list) {
    match input_list {
        [] => output_list == [],
        [head | tail] => {
            |filtered_tail| {
                filter(predicate, tail, filtered_tail),
                any {
                    // Include element if predicate succeeds
                    all {
                        call(predicate, head),
                        output_list == [head | filtered_tail]
                    },
                    // Exclude element if predicate fails
                    all {
                        not(call(predicate, head)),
                        output_list == filtered_tail
                    }
                }
            }
        }
    }
}

// Fold left (reduce)
rel foldl(predicate, initial, list, result) {
    match list {
        [] => result == initial,
        [head | tail] => {
            |intermediate| {
                call(predicate, initial, head, intermediate),
                foldl(predicate, intermediate, tail, result)
            }
        }
    }
}

// Example usage
@test(expected = [[2, 4, 6, 8]])
rel test_map_double(result) {
    map(double, [1, 2, 3, 4], result)
}

rel double(x, result) {
    result == x * 2
}
```

#### Tree Processing

```prolog
// Binary tree structure
enum Tree {
    Empty,
    Node(Number, Tree, Tree)  // value, left, right
}

// Tree membership
rel tree_member(value, tree) {
    match tree {
        Tree::Empty => false,
        Tree::Node(v, left, right) => {
            any {
                value == v,
                tree_member(value, left),
                tree_member(value, right)
            }
        }
    }
}

// Tree insertion (binary search tree)
rel tree_insert(value, old_tree, new_tree) {
    match old_tree {
        Tree::Empty => new_tree == Tree::Node(value, Tree::Empty, Tree::Empty),
        Tree::Node(v, left, right) => {
            any {
                // Insert into left subtree
                all {
                    value < v,
                    |new_left| {
                        tree_insert(value, left, new_left),
                        new_tree == Tree::Node(v, new_left, right)
                    }
                },
                // Insert into right subtree
                all {
                    value > v,
                    |new_right| {
                        tree_insert(value, right, new_right),
                        new_tree == Tree::Node(v, left, new_right)
                    }
                },
                // Value already exists
                all {
                    value == v,
                    new_tree == old_tree
                }
            }
        }
    }
}
```

### Constraint Satisfaction Problems

#### Sudoku Solver

```prolog
// 4x4 Sudoku for simplicity
rel sudoku_4x4(grid) {
    // Grid structure: [[r1c1,r1c2,r1c3,r1c4], [r2c1,...], ...]
    |row1, row2, row3, row4| {
        grid == [row1, row2, row3, row4],
        row1 == [r1c1, r1c2, r1c3, r1c4],
        row2 == [r2c1, r2c2, r2c3, r2c4],
        row3 == [r3c1, r3c2, r3c3, r3c4],
        row4 == [r4c1, r4c2, r4c3, r4c4],
        
        constraint(domain="clpfd") {
            // All values 1-4
            [r1c1,r1c2,r1c3,r1c4, r2c1,r2c2,r2c3,r2c4,
             r3c1,r3c2,r3c3,r3c4, r4c1,r4c2,r4c3,r4c4] in 1..4,
            
            // Row constraints
            alldiff [r1c1, r1c2, r1c3, r1c4],
            alldiff [r2c1, r2c2, r2c3, r2c4],
            alldiff [r3c1, r3c2, r3c3, r3c4],
            alldiff [r4c1, r4c2, r4c3, r4c4],
            
            // Column constraints
            alldiff [r1c1, r2c1, r3c1, r4c1],
            alldiff [r1c2, r2c2, r3c2, r4c2],
            alldiff [r1c3, r2c3, r3c3, r4c3],
            alldiff [r1c4, r2c4, r3c4, r4c4],
            
            // Box constraints (2x2 boxes)
            alldiff [r1c1, r1c2, r2c1, r2c2],  // Top-left box
            alldiff [r1c3, r1c4, r2c3, r2c4],  // Top-right box
            alldiff [r3c1, r3c2, r4c1, r4c2],  // Bottom-left box
            alldiff [r3c3, r3c4, r4c3, r4c4]   // Bottom-right box
        }
    }
}
```

#### Graph Coloring

```prolog
// Graph coloring with constraint programming
struct Edge(Number, Number);  // edge between two vertices

rel graph_coloring(edges, num_vertices, num_colors, coloring) {
    |colors| {
        length(colors, num_vertices),
        constraint(domain="clpfd") {
            colors in 1..num_colors,
            no_adjacent_same_color(edges, colors)
        },
        coloring == colors
    }
}

rel no_adjacent_same_color(edges, colors) {
    match edges {
        [] => true,
        [Edge(v1, v2) | rest] => {
            |color1, color2| {
                nth(v1, colors, color1),  // Get color of vertex v1
                nth(v2, colors, color2),  // Get color of vertex v2
                constraint(domain="clpfd") {
                    color1 != color2      // Adjacent vertices have different colors
                },
                no_adjacent_same_color(rest, colors)
            }
        }
    }
}
```

### Real-World Applications

#### Configuration File Parser

```prolog
use std::fs::*;
use std::list::*;

// Parse simple key=value configuration
rel parse_config_file(filename, config) {
    |lines| {
        read_file_lines(filename, lines),
        parse_config_lines(lines, config)
    }
}

rel parse_config_lines(lines, config) {
    match lines {
        [] => config == [],
        [line | rest] => {
            |parsed_line, rest_config| {
                parse_config_line(line, parsed_line),
                parse_config_lines(rest, rest_config),
                config == [parsed_line | rest_config]
            }
        }
    }
}

rel parse_config_line(line, result) {
    any {
        // Skip empty lines and comments
        all {
            any {
                line == "",
                string_starts_with(line, "#")
            },
            result == skip
        },
        // Parse key=value pairs
        |key, value| {
            split_on_equals(line, key, value),
            result == config_entry(key, value)
        }
    }
}
```

#### Simple Calculator

```prolog
// Expression evaluation
enum Expr {
    Num(Number),
    Add(Expr, Expr),
    Sub(Expr, Expr),
    Mul(Expr, Expr),
    Div(Expr, Expr)
}

rel eval_expr(expr, result) {
    match expr {
        Expr::Num(n) => result == n,
        Expr::Add(left, right) => {
            |left_val, right_val| {
                eval_expr(left, left_val),
                eval_expr(right, right_val),
                result == left_val + right_val
            }
        },
        Expr::Sub(left, right) => {
            |left_val, right_val| {
                eval_expr(left, left_val),
                eval_expr(right, right_val),
                result == left_val - right_val
            }
        },
        Expr::Mul(left, right) => {
            |left_val, right_val| {
                eval_expr(left, left_val),
                eval_expr(right, right_val),
                result == left_val * right_val
            }
        },
        Expr::Div(left, right) => {
            |left_val, right_val| {
                eval_expr(left, left_val),
                eval_expr(right, right_val),
                right_val != 0,  // Division by zero check
                result == left_val / right_val
            }
        }
    }
}

// Example: (2 + 3) * 4 = 20
@test(expected = [20])
rel test_calculator(result) {
    |expr| {
        expr == Expr::Mul(
            Expr::Add(Expr::Num(2), Expr::Num(3)),
            Expr::Num(4)
        ),
        eval_expr(expr, result)
    }
}
```

---

## Appendices

### A. Grammar Reference

Proto-Vulcan's complete syntax specification:

```ebnf
Program ::= Item*

Item ::= Relation
       | Struct
       | Enum
       | Module
       | Import

Relation ::= Attribute* Visibility? "rel" Identifier "(" ParamList? ")" Goal

Struct ::= "struct" Identifier ( TupleFields | NamedFields )

TupleFields ::= "(" TypeList? ")"

NamedFields ::= "{" FieldList? "}"

Enum ::= "enum" Identifier "{" VariantList? "}"

Module ::= "mod" Identifier "{" Item* "}"

Import ::= "use" ImportPath ";"

Goal ::= UnificationGoal
       | DisequalityGoal
       | RelationCall
       | AllGoal
       | AnyGoal
       | MatchGoal
       | FreshGoal
       | ConstraintGoal

UnificationGoal ::= Term "==" Term

DisequalityGoal ::= Term "!=" Term

RelationCall ::= QualifiedPath "(" TermList? ")"

AllGoal ::= "all" ("(" StrategySpec ")")? "{" GoalList "}"

AnyGoal ::= "any" ("(" StrategySpec ")")? "{" GoalList "}"

MatchGoal ::= "match" Term "{" MatchArm+ "}"

FreshGoal ::= "|" IdentifierList "|" Goal

ConstraintGoal ::= "constraint" "(" DomainSpec ")" "{" ConstraintList "}"

Term ::= Variable
       | Literal
       | StructTerm
       | ListTerm

Attribute ::= "@" Identifier ( "(" AttributeArgs? ")" )?

Visibility ::= "pub"
```

### B. Error Messages and Solutions

#### Common Compilation Errors

**Unknown relation 'foo'**
```
Solution: Check spelling, ensure relation is defined, verify imports
```

**Arity mismatch: expected 2 arguments, found 3**
```
Solution: Check relation definition, verify number of arguments in call
```

**Cannot resolve import 'std::unknown'**
```
Solution: Check module exists, verify import path spelling
```

**Shadowing warning: variable 'x' shadows previous declaration**
```
Solution: Use different variable name or disable with --warn-shadowing false
```

#### Runtime Errors

**Stack overflow in recursive relation**
```
Solution: Check for infinite recursion, add base cases, use tail recursion
```

**Constraint domain error: variable not in domain**
```
Solution: Ensure variables are properly constrained before use
```

**File not found: 'missing.pv'**
```
Solution: Check file path, ensure file exists, verify working directory
```

### C. Performance Guide

#### Best Practices for Efficient Code

1. **Use constraints early and specifically**
   ```prolog
   // Good: specific constraints first
   constraint(domain="clpfd") {
       x in 1..10,
       x > 5,
       expensive_constraint(x)
   }
   
   // Bad: expensive constraints first
   constraint(domain="clpfd") {
       expensive_constraint(x),
       x in 1..1000,
       x > 995
   }
   ```

2. **Prefer tail recursion**
   ```prolog
   // Good: tail recursive
   rel sum_acc(list, acc, result) {
       match list {
           [] => result == acc,
           [head | tail] => sum_acc(tail, acc + head, result)
       }
   }
   ```

3. **Use appropriate search strategy**
   ```prolog
   // Use DFS for deep, narrow searches
   rel find_path(start, end, path) @dfs {
       // Path finding logic
   }
   
   // Use BFS for broad, shallow searches  
   rel find_all_neighbors(node, neighbors) @bfs {
       // Neighbor finding logic
   }
   ```

4. **Minimize fresh variable scope**
   ```prolog
   // Good: minimal scope
   rel example() {
       goal1(),
       |x| { goal2(x), goal3(x) },
       goal4()
   }
   ```

#### Memory and Performance Monitoring

- Use `--trace` to understand execution patterns
- Monitor constraint propagation effectiveness
- Profile with Rust profiling tools when needed

### D. Contributing Guide

#### Adding New Builtin Predicates

1. **Implement in Rust**: Add function to appropriate module in `src/interpreter/builtins/`
2. **Register builtin**: Add entry to builtin specs in `builtins/mod.rs`
3. **Add library wrapper**: Create user-friendly wrapper in `std/` modules
4. **Write tests**: Add comprehensive test cases
5. **Document**: Update this guide and add examples

#### Extending the Language

1. **Parser changes**: Modify grammar in `src/interpreter/parser/grammar.pest`
2. **AST updates**: Add new AST nodes in `parser/ast.rs`
3. **Compilation**: Update compiler to handle new constructs
4. **Runtime support**: Add runtime execution logic
5. **Testing**: Comprehensive test coverage for new features

For detailed contribution guidelines, see the project's CONTRIBUTING.md file.

---

*This guide covers the essential aspects of programming in Proto-Vulcan. For the most up-to-date information and advanced topics, refer to the project documentation and source code.*