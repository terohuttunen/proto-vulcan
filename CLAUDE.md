# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Proto-Vulcan is a relational logic programming language embedded in Rust, inspired by miniKanren. It supports both macro-based embedding in Rust code and direct execution via the new interpreter that processes `.pv` files.

## Development Commands

### Build and Test
```bash
# Build the project
cargo build

# Run all tests
cargo test

# Build with all features
cargo build --features "core,extras,clpfd,clpz,debugger"

# Run examples (most require clpfd feature)
cargo run --example sudoku --features clpfd
cargo run --example n-queens --features clpfd
```

### CLI Usage
```bash
# Run a .pv file with @main relation
cargo run -- examples/zebra.pv

# Run specific queries
cargo run -- --query "member(X, [1, 2, 3])" std/list.pv

# Control output format
cargo run -- --format table --limit 10 examples/zebra.pv
cargo run -- --format json --query "member(X, [1, 2, 3])" std/list.pv

# Enable tracing
cargo run -- --trace --query "member(X, [1, 2, 3])" std/list.pv

# Run tests
cargo run -- test
cargo run -- test --file examples/zebra.pv
cargo run -- test --filter "pattern*"

# Syntax checking
cargo run -- check examples/zebra.pv --show-ast
```

### Test Infrastructure
```bash
# Run interpreter tests specifically
cargo test interpreter --lib

# Run individual module tests
cargo test interpreter::execution --lib
cargo test interpreter::parser --lib
cargo test interpreter::environment --lib

# Run with specific features
cargo test --features "clpfd,clpz"
```

## Architecture

### Dual Language System
Proto-Vulcan operates as two related but distinct languages:

1. **Macro-based (Legacy)**: Embedded in Rust using `proto_vulcan_query!`, `proto_vulcan!`, etc.
2. **Interpreter-based (Current)**: Direct execution of `.pv` files with comprehensive language support

### Core Components

#### Engine and Runtime (`src/`)
- **Engine**: Stream-based execution engine (`engine.rs`, `stream.rs`)
- **Solver**: Core solving logic (`solver.rs`)
- **State Management**: Substitution maps, constraints, unification (`state/`)
- **Relations**: Built-in predicates like `eq`, `member`, `append` (`relation/`)
- **Operators**: Logic operators like `conde`, `anyo`, `fresh` (`operator/`)

#### Interpreter (`src/interpreter/`)
- **Parser**: Converts `.pv` source to AST (`parser/`)
- **Environment**: Symbol tables and scoping (`environment.rs`)
- **Execution**: AST to runtime goal conversion (`execution.rs`)
- **Query System**: Query parsing and execution (`query.rs`)
- **Import System**: Module resolution and loading (`import/`)
- **Constraint Domains**: CLP(FD) and CLP(Z) support (`constraint_domains/`)

#### Key Type Relationships
- `LTerm`: Logic terms (variables, values, compounds)
- `Goal`: Runtime goals for the solver
- `State`: Solver state with substitutions and constraints
- `Stream`: Lazy evaluation streams
- `ArgumentValue`: Unified parameter type supporting both `Meta(MetaValue)` and `Relational(LTerm)`

### Language Features

#### .pv Language Support
- **Relations**: `rel name(params) { body }`
- **Structs**: `struct Name { fields }` with named and compound variants
- **Modules**: `mod name { items }` with visibility controls
- **Goals**: Equality (`==`), disequality (`!=`), conjunction (`all`), disjunction (`any`)
- **Pattern Matching**: `match term { pattern => goal }`
- **Fresh Variables**: `|x| goal` introduces scoped variables
- **Constraint Blocks**: `constraint(domain="clpfd") { constraints }`
- **Search Strategies**: `@bfs`, `@dfs` annotations

#### Test System
- Tests in `.pv` files marked with `@test` attribute
- Built-in assertions: `assert_eq`, `assert_neq`, `assert_succeeds`, `assert_fails`
- Test runner with filtering, timing, and parallel execution options

### Builtin Predicate System (`src/interpreter/builtins/`)

Proto-Vulcan features a unified builtin predicate system that supports both relational terms and meta-values (compile-time values like integers, strings, booleans).

#### Architecture
```
src/interpreter/builtins/
├── mod.rs       # Central registration system
├── core.rs      # Core language builtins
├── testing.rs   # Test assertion builtins  
└── macros.rs    # Helper utilities for builtin implementation
```

#### Builtin Categories
- **Core Language**: `__builtin_length` - list length calculation
- **Test Assertions**: `assert_eq`, `assert_neq`, `assert_bound`, `assert_unbound`, `assert_domain_size`

#### Parameter System
Builtins use `Vec<ArgumentValue>` parameters supporting:
- `ArgumentValue::Relational(LTerm)` - Logic terms for unification
- `ArgumentValue::Meta(MetaValue)` - Compile-time values (integers, strings, booleans)

This unified interface allows builtins to receive the same parameter types as regular predicates, enabling future support for higher-order predicates and meta-programming.

#### Adding New Builtins
1. Implement function with signature `fn(Vec<ArgumentValue>) -> Goal`
2. Add `BuiltinSpec` entry to array in `builtins/mod.rs`
3. Use helper functions from `macros.rs` for common patterns
4. Follow naming conventions: `__builtin_` for core, `assert_` for testing

### Standard Library (`std/`)
- `list.pv`: List manipulation predicates
- `higher_order.pv`: Higher-order operations
- `mod.pv`: Module entry point

## Migration Notes

The project supports both legacy macro syntax and new `.pv` syntax. See `MIGRATION_GUIDE.md` for detailed migration instructions from macro-based to interpreter-based syntax.

## Constraint Programming

Proto-Vulcan includes constraint programming domains:
- **CLP(FD)**: Finite domain constraints for integers
- **CLP(Z)**: Integer arithmetic constraints
- Constraint blocks with domain-specific syntax parsing

## Development Patterns

### Adding New Relations

#### Builtin Relations
1. **Create the builtin function**: Implement with signature `fn(Vec<ArgumentValue>) -> Goal`
   ```rust
   pub fn my_builtin(args: Vec<ArgumentValue>) -> Goal {
       let (term1, term2) = match extract_two_relational(args) {
           Ok(result) => result,
           Err(goal) => return goal, // Handle arity/type errors
       };
       // Implementation logic here
   }
   ```

2. **Register the builtin**: Add `BuiltinSpec` to array in `builtins/mod.rs`
   ```rust
   BuiltinSpec { name: "__builtin_my_pred", arity: 2, func: my_builtin },
   ```

3. **Organize by category**: Place in appropriate module (`core.rs`, `testing.rs`, etc.)

#### Library Relations
- Add to appropriate `.pv` file in `std/` directory
- Use standard `.pv` syntax with proper module declarations
- Follow existing naming and documentation conventions

### Working with AST
- Parse with `parser::parse_str()` for programs or `query::parse_query()` for queries
- Convert AST to runtime using `ExecutionContext` in `execution.rs`
- AST types defined in `parser/ast.rs`

### Testing Strategy
- Unit tests for individual modules using `cargo test`
- Integration tests via the CLI test runner using `.pv` test files
- End-to-end tests in `src/interpreter/mod.rs`
- **Important**: Run interpreter tests after making changes with `cargo run -- test`

## VS Code Integration

The repository includes a VS Code extension in `vscode-plugin/` with syntax highlighting and language support for `.pv` files.