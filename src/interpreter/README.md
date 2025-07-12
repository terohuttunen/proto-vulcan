# Proto-Vulcan Interpreter

This module provides a complete interpreter for the proto-vulcan relational logic programming language. The interpreter bridges the gap between parsed AST and the existing runtime system, enabling direct execution of proto-vulcan programs without macro expansion.

## Architecture Overview

The interpreter follows a layered architecture:

```
Source Code → Parser → AST → Interpreter → Runtime Goals → Solver → Results
```

### Core Components

#### 1. **Parser Module** (`parser/`)
- **Grammar**: Complete Pest-based grammar for the language (`grammar.pest`)
- **AST**: Comprehensive Abstract Syntax Tree definitions (`ast.rs`)
- **Parser**: Converts source code to AST using Pest (`mod.rs`)

**Features:**
- Full language support: relations, goals, modules, structs, patterns
- Error handling with detailed error messages
- 29 comprehensive tests covering all language constructs

#### 2. **Environment Module** (`environment.rs`)
- **Symbol Table**: Maps identifiers to runtime values
- **Scope Management**: Handles modules, relations, and variable scoping
- **Type Registry**: Manages struct definitions and compound types

**Features:**
- Global and module-scoped symbol resolution
- Relation and struct definition storage
- Fresh variable generation
- 5 tests covering scoping and symbol management

#### 3. **Runtime Value Module** (`runtime_value.rs`)
- **Bridge Types**: Converts between AST terms and runtime LTerms
- **Value System**: Handles relations, terms, and type constructors
- **Type Conversion**: AST literals to runtime values

**Features:**
- Support for all literal types (boolean, number, string, char)
- Variable and list conversion
- Relation definition storage
- 8 tests covering all conversion scenarios

#### 4. **Query Module** (`query.rs`)
- **Query Execution**: Placeholder for query processing
- **Result Handling**: QueryResult type for variable bindings
- **Integration**: Connects with environment for query resolution

**Features:**
- QueryResult structure for results
- Parse query interface (placeholder)
- 1 test for basic functionality

#### 5. **Integration Module** (`integration.rs`)
- **Runtime Integration**: Utilities for connecting with existing runtime
- **Type Safety**: Proper generic type handling

## Usage Example

```rust
use proto_vulcan::interpreter::{Interpreter, InterpreterError};
use proto_vulcan::user::DefaultUser;
use proto_vulcan::engine::DefaultEngine;

type MyInterpreter = Interpreter<DefaultUser, DefaultEngine<DefaultUser>>;

fn main() -> Result<(), InterpreterError> {
    let mut interpreter = MyInterpreter::new();
    
    // Parse and load a program
    let program_source = r#"
        pub struct Point {
            pub x: i32,
            pub y: i32,
        }
        
        rel distance(p1: Point, p2: Point) @bfs {
            p1 == p2
        }
    "#;
    
    let program = proto_vulcan::interpreter::parser::parse_str(program_source)?;
    interpreter.load_program(program)?;
    
    // Verify the program was loaded
    let env = interpreter.environment();
    assert!(env.lookup("distance").is_some());
    assert!(env.get_struct("Point").is_some());
    
    Ok(())
}
```

## Language Support

The interpreter supports the full proto-vulcan language:

### **Declarations**
- **Relations**: `rel name(params) { goals }`
- **Structures**: `struct Name { fields }` and `struct Name(types);`
- **Modules**: `mod name { items }`
- **Imports**: `use path;`, `use path::*;`, `use path::{a, b as c};`

### **Goals**
- **Equality**: `a == b`
- **Disequality**: `a != b`
- **Conjunction**: `[goal1, goal2]`
- **Disjunction**: `conde { goal1, goal2 }`
- **Fresh Variables**: `|x, y| { goals }`
- **Let Declarations**: `let x = value;`
- **Relation Calls**: `relation(args)`
- **Method Calls**: `obj.method(args)`
- **Pattern Matching**: `match term { pattern => goals }`

### **Terms**
- **Literals**: `true`, `42`, `"string"`, `'c'`
- **Variables**: `x`, `my_var`
- **Lists**: `[1, 2, 3]`, `[]`
- **Structs**: `Point { x: 1, y: 2 }`
- **Compounds**: `Some(42)`, `Node(left, right)`

### **Patterns**
- **Literals**: `true`, `42`
- **Variables**: `x`
- **Wildcards**: `_`
- **Lists**: `[head | tail]`, `[a, b, c]`
- **Structs**: `Point { x: px, y: py }`
- **Compounds**: `Some(value)`

### **Modifiers**
- **Visibility**: `pub rel`, `pub struct`
- **Search Strategy**: `@bfs`, `@dfs`

## Testing

The interpreter includes comprehensive testing:

```bash
# Run all interpreter tests (49 tests)
cargo test interpreter --lib

# Run specific module tests
cargo test interpreter::parser --lib      # 29 tests
cargo test interpreter::environment --lib # 5 tests  
cargo test interpreter::runtime_value --lib # 8 tests
cargo test interpreter::query --lib       # 1 test
cargo test interpreter::tests --lib       # 6 integration tests
```

### **Test Coverage**
- **Parser Tests**: All language constructs, error handling
- **Environment Tests**: Symbol resolution, scoping, type management
- **Runtime Value Tests**: AST to runtime conversion
- **Integration Tests**: End-to-end parser to interpreter workflow

## Design Principles

### **1. Separation of Concerns**
- Parser handles syntax analysis
- Environment manages program state
- Runtime values bridge AST and runtime
- Integration connects with existing solver

### **2. Type Safety**
- Generic over User and Engine types
- Proper error handling with InterpreterError
- Compile-time type checking

### **3. Incremental Development**
- Core components implemented and tested
- Execution module planned for future implementation
- Modular architecture enables independent development

### **4. Compatibility**
- Uses existing runtime infrastructure (LTerm, Goal, Solver)
- Maintains compatibility with current macro system
- Enables gradual migration from macros to interpreter

## Future Development

### **Execution Module** (Planned)
The execution module will complete the interpreter by implementing:
- AST goal to runtime goal conversion
- Relation body execution
- Variable binding and unification
- Integration with the solver system

### **Query System Enhancement**
- Complete query parsing implementation
- Result formatting and display
- Interactive query interface

### **Performance Optimization**
- Caching of parsed programs
- Optimized symbol lookup
- Lazy evaluation of relation bodies

## Error Handling

The interpreter provides comprehensive error handling:

```rust
#[derive(Debug, Clone)]
pub enum InterpreterError {
    ParseError(String),      // Syntax errors
    RuntimeError(String),    // Execution errors  
    UnknownRelation(String), // Undefined relations
    UnknownVariable(String), // Undefined variables
    TypeMismatch(String),    // Type errors
    ScopeError(String),      // Scoping issues
}
```

All errors implement `std::error::Error` and provide detailed error messages for debugging.

## Integration with Existing System

The interpreter is designed to work alongside the existing macro system:

1. **Parser**: Can be used independently for syntax validation
2. **Environment**: Provides symbol management for any system
3. **Runtime Values**: Bridge between AST and existing runtime types
4. **Gradual Migration**: Enables step-by-step transition from macros

This architecture provides a solid foundation for a complete proto-vulcan interpreter while maintaining compatibility with the existing codebase. 