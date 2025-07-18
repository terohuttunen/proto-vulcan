# Proto-Vulcan Language Support

A Visual Studio Code extension that provides syntax highlighting and language support for Proto-Vulcan, a modern logic programming language.

## Features

- **Syntax Highlighting**: Full syntax highlighting for `.pv` files including:
  - Keywords (`rel`, `macro`, `if`, `else`, etc.)
  - Operators and logical connectives
  - String and numeric literals
  - Comments
  - Test annotations (`@test`)
  - Constraint programming constructs

- **Language Configuration**: Proper bracket matching, auto-indentation, and commenting support

## About Proto-Vulcan

Proto-Vulcan is a logic programming language that combines:
- **Relational Programming**: Define relationships between data
- **Constraint Programming**: Finite domain and integer constraints
- **Macros**: Template-based code generation
- **Testing Framework**: Built-in test annotations
- **Higher-Order Programming**: Support for meta-programming constructs

## File Extensions

This extension automatically activates for files with the `.pv` extension.

## Example Syntax

```proto-vulcan
// Define a relation
rel append(a, b, c) {
    if a == [] {
        b == c
    } else {
        |first, rest, result| {
            a == [first, rest],
            c == [first, result],
            append(rest, b, result)
        }
    }
}

// Test the relation
@test(expected = [[1, 2, 3, 4]])
rel test_append(q) {
    append([1, 2], [3, 4], q)
}

// Define a macro
macro repeat_element(count: int, element, result) {
    if count == 0 {
        result == []
    } else {
        |tail| {
            repeat_element(count - 1, element, tail),
            result == [element | tail]
        }
    }
}

// Grade classification with else if chains
macro classify_grade(score: int, result) {
    if score >= 90 {
        result == "A"
    } else if score >= 80 {
        result == "B"
    } else if score >= 70 {
        result == "C"
    } else if score >= 60 {
        result == "D"
    } else {
        result == "F"
    }
}
```

## Language Features Supported

### Keywords
- `rel` - Define relations
- `macro` - Define macros
- `if` - Conditional logic
- `else` - Alternative conditions
- `else if` - Chained conditional logic
- `import` - Import modules
- `module` - Define modules

### Operators
- `==` - Unification
- `!=` - Disequality
- `<`, `>`, `<=`, `>=` - Comparison
- `+`, `-`, `*`, `/` - Arithmetic
- `&&`, `||` - Logical connectives

### Special Constructs
- `|var|` - Fresh variable introduction
- `@test` - Test annotations
- `#|...|#` - Multi-line comments
- `//` - Single-line comments

### Constraint Programming
- Finite domain constraints (`infd`, `domfd`, etc.)
- Integer constraints (`plusz`, `timesz`, etc.)
- Distinct value constraints

## Installation

This extension can be installed by copying the extension files to your VS Code extensions directory. Use the provided installation scripts:

### Linux/macOS
```bash
./install.sh          # For VS Code
./install-cursor.sh    # For Cursor IDE
```

### Windows
```cmd
install.bat            # For VS Code
install-cursor.bat     # For Cursor IDE
```

## Development

To contribute to this extension:

1. Clone the repository
2. Make your changes to the syntax definitions in `syntaxes/proto-vulcan.tmLanguage.json`
3. Test with sample `.pv` files
4. Submit a pull request

## License

This extension is licensed under the MIT License.

## Links

- [Proto-Vulcan Repository](https://github.com/your-username/proto-vulcan)
- [Logic Programming](https://en.wikipedia.org/wiki/Logic_programming)
- [Constraint Programming](https://en.wikipedia.org/wiki/Constraint_programming)

---

**Enjoy coding in Proto-Vulcan!** 🖖 