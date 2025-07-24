# Proto-Vulcan Interpreter

A comprehensive interpreter for the proto-vulcan relational logic programming language. This interpreter provides a complete pipeline from source code to execution, supporting direct program execution without macro expansion.

## Architecture

The interpreter follows a layered architecture:

```
Source Code → Parser → AST → Interpreter → Runtime Goals → Solver → Results
```

### Components

1. **Parser** (`parser/`): Converts source code into Abstract Syntax Tree (AST)
2. **Environment** (`environment.rs`): Manages symbol tables, scopes, and program state
3. **Runtime Values** (`runtime_value.rs`): Bridges AST terms with runtime LTerms
4. **Execution** (`execution.rs`): Converts AST goals to runtime goals for execution
5. **Query** (`query.rs`): Handles query parsing and execution
6. **Integration** (`integration.rs`): Utilities for connecting with existing runtime

## Key Features

### Execution Module

The execution module (`execution.rs`) is the core component that converts AST goals into runtime goals that can be executed by the proto-vulcan solver. It provides:

#### ExecutionContext
- **Variable Management**: Maintains local variable bindings with proper scoping
- **Fresh Variable Generation**: Creates unique variable names to avoid conflicts
- **Environment Integration**: Connects with the symbol table for relation lookups

#### Goal Conversion
- **Equality/Disequality**: Converts `==` and `!=` goals to runtime equivalents
- **Conjunction/Disjunction**: Handles `[]` and `conde` goal combinations
- **Fresh Variables**: Manages `|x|` fresh variable scoping with proper cleanup
- **Relation Calls**: Resolves relation names and executes relation bodies
- **Method Calls**: Treats method calls as relation calls with receiver as first argument

#### Pattern Matching
- **Literal Patterns**: Matches against boolean, number, string, and char literals
- **Variable Patterns**: Binds variables to matched terms
- **Wildcard Patterns**: Always succeeds with `_` pattern
- **List Patterns**: Supports list destructuring with tail patterns
- **Struct Patterns**: Handles named and compound struct pattern matching

#### Term Conversion
- **Literals**: Converts AST literals to runtime LTerms
- **Variables**: Manages variable bindings and creates fresh variables as needed
- **Lists**: Converts list constructions to runtime list terms
- **Named Structs**: Handles struct construction with field initialization
- **Compound Terms**: Supports compound constructor calls

#### Variable Scoping
- **Lexical Scoping**: Maintains proper variable scope boundaries
- **Fresh Variable Isolation**: Fresh variables are properly scoped and cleaned up
- **Binding Restoration**: Previous variable bindings are restored after scope exit

## Language Support

### Complete proto-vulcan Language Support

#### Declarations
- **Relations** (`rel name(params) { body }`): Function-like logical relations
- **Structures** (`struct Name { fields }`): Data type definitions
- **Modules** (`mod name { items }`): Namespace organization
- **Imports** (`use path`): Module importing (parsing only)

#### Goals
- **Equality** (`x == y`): Unification goals
- **Disequality** (`x != y`): Disequality constraints
- **Conjunction** (`[goal1, goal2]`): Logical AND
- **Disjunction** (`conde { goal1; goal2 }`): Logical OR
- **Fresh Variables** (`|x| goal`): Introduce fresh variables
- **Let Declarations** (`let x = value`): Variable binding
- **Relation Calls** (`relation(args)`): Call defined relations
- **Method Calls** (`receiver.method(args)`): Method-style calls
- **Pattern Matching** (`match term { pattern => goal }`): Pattern-based dispatch

#### Terms
- **Literals**: `true`, `false`, `42`, `"string"`, `'c'`
- **Variables**: `x`, `my_var`
- **Lists**: `[1, 2, 3]`, `[head | tail]`
- **Named Structs**: `Point { x: 10, y: 20 }`
- **Compound Terms**: `Some(value)`, `Node(left, right)`

#### Patterns
- **Literals**: `42`, `"hello"`, `true`
- **Variables**: `x`, `result`
- **Wildcards**: `_`
- **List Patterns**: `[a, b, c]`, `[head | tail]`
- **Struct Patterns**: `Point { x, y }`, `Some(value)`

#### Modifiers
- **Visibility**: `pub` for public items
- **Search Strategies**: `@bfs`, `@dfs` for breadth-first/depth-first search

## Usage

### Basic Usage

```rust
use proto_vulcan::interpreter::Interpreter;
use proto_vulcan::engine::DefaultEngine;
use proto_vulcan::user::DefaultUser;

type MyInterpreter = Interpreter;

let mut interpreter = MyInterpreter::new();

// Load and parse a program
let program_source = r#"
    rel parent(x, y) {
        x == "alice", y == "bob";
        x == "bob", y == "charlie"
    }
"#;

let program = parser::parse_program(program_source)?;
interpreter.load_program(program)?;

// Execute queries
let results = interpreter.query("parent(X, Y)")?;
```

### Advanced Features

```rust
// Complex program with structs and pattern matching
let program_source = r#"
    struct Point {
        x: i32,
        y: i32
    }
    
    rel distance(p1, p2, result) {
        match p1 {
            Point { x: x1, y: y1 } => {
                match p2 {
                    Point { x: x2, y: y2 } => {
                        // Calculate distance (simplified)
                        result == 0
                    }
                }
            }
        }
    }
"#;

let program = parser::parse_program(program_source)?;
interpreter.load_program(program)?;

let results = interpreter.query("distance(Point{x:0,y:0}, Point{x:3,y:4}, D)")?;
```

## Testing

The interpreter includes comprehensive tests covering all major functionality:

### Test Coverage
- **Execution Module**: 12 tests covering goal conversion, pattern matching, and variable scoping
- **Parser Module**: 29 tests covering all language constructs
- **Environment Module**: 5 tests covering symbol table and scoping
- **Runtime Values Module**: 8 tests covering AST to runtime conversion
- **Integration Tests**: 6 tests covering end-to-end workflows

### Running Tests

```bash
# Run all interpreter tests
cargo test interpreter --lib

# Run specific module tests
cargo test interpreter::execution --lib
cargo test interpreter::parser --lib
cargo test interpreter::environment --lib
```

## Implementation Details

### Type System Integration
- **Generic Design**: Fully generic over User and Engine types
- **Type Safety**: Comprehensive error handling and type checking
- **Goal Casting**: Proper conversion between different goal types using GoalCast trait

### Performance Considerations
- **Lazy Evaluation**: Goals are evaluated lazily by the solver
- **Memory Management**: Efficient variable lifetime management
- **Cloning Strategy**: Strategic cloning to avoid borrowing conflicts

### Error Handling
- **Comprehensive Errors**: Detailed error types for different failure modes
- **Parse Errors**: Clear error messages for syntax issues
- **Runtime Errors**: Helpful error messages for execution problems
- **Type Errors**: Clear indication of type mismatches

## Future Enhancements

### Planned Features
1. **List Relations**: Proper list element extraction and manipulation
2. **Struct Type Checking**: Runtime verification of struct types
3. **Module System**: Full import/export functionality
4. **Optimization**: Performance improvements for large programs
5. **Debugging**: Enhanced debugging and tracing capabilities

### Extension Points
- **Custom Relations**: Easy addition of built-in relations
- **Type Extensions**: Support for additional data types
- **Search Strategies**: Pluggable search strategy implementations
- **Constraint Domains**: Integration with constraint solving domains

## Contributing

The interpreter is designed to be extensible and maintainable:

1. **Modular Design**: Clear separation of concerns between components
2. **Comprehensive Tests**: All functionality is thoroughly tested
3. **Documentation**: Detailed documentation for all public APIs
4. **Type Safety**: Leverages Rust's type system for correctness

When adding new features:
1. Add comprehensive tests covering the new functionality
2. Update documentation to reflect changes
3. Ensure all existing tests continue to pass
4. Follow the established patterns for error handling and type safety 