# Proto-Vulcan VSCode Extension

A complete VSCode syntax highlighting extension for the Proto-Vulcan logic programming language.

## 📁 Extension Structure

```
proto-vulcan/
├── src/                            # Language implementation
├── tests/                          # Test files (.pv)
├── std/                            # Standard library
└── vscode-plugin/                  # VSCode/Cursor IDE extension
    ├── package.json                # Extension manifest
    ├── language-configuration.json # Language settings (brackets, comments, etc.)
    ├── syntaxes/
    │   └── proto-vulcan.tmLanguage.json # TextMate grammar
    ├── README.md                   # Extension documentation
    ├── EXTENSION_README.md         # This detailed guide
    ├── install.sh                  # VSCode installation script for Linux/macOS
    ├── install.bat                 # VSCode installation script for Windows
    ├── install-cursor.sh           # Cursor installation script for Linux/macOS
    ├── install-cursor.bat          # Cursor installation script for Windows
    └── .vscodeignore               # Extension packaging exclusions

```

## 🚀 Quick Installation

### For VSCode

**Linux/macOS:**
```bash
cd vscode-plugin
./install.sh
```

**Windows:**
```cmd
cd vscode-plugin
install.bat
```

### For Cursor IDE

**Linux/macOS:**
```bash
cd vscode-plugin
./install-cursor.sh
```

**Windows:**
```cmd
cd vscode-plugin
install-cursor.bat
```

### Manual Installation

**For VSCode:**
1. Copy the extension files to your VSCode extensions directory:
   - **Windows**: `%USERPROFILE%\.vscode\extensions\proto-vulcan\`
   - **macOS**: `~/.vscode/extensions/proto-vulcan/`
   - **Linux**: `~/.vscode/extensions/proto-vulcan/`

**For Cursor IDE:**
1. Copy the extension files to your Cursor extensions directory:
   - **Windows**: `%USERPROFILE%\.cursor\extensions\proto-vulcan\`
   - **macOS**: `~/.cursor/extensions/proto-vulcan/`
   - **Linux**: `~/.cursor/extensions/proto-vulcan/`

2. Restart VSCode or Cursor

## 🎨 Features Implemented

### Syntax Highlighting
- ✅ **Keywords**: `rel`, `macro`, `use`, `mod`, `struct`, `impl`, `pub`, `constraint`, `let`, `if`, `else`, `else if`, `for`, `in`, `match`, `any`, `all`
- ✅ **Attributes**: `@test`, `@dfs`, `@bfs` with parameters
- ✅ **Operators**: `==`, `!=`, `<`, `>`, `<=`, `>=`, `=`, `=>`, `|`, `..`, `::`
- ✅ **Comments**: Line (`//`) and block (`/* */`) comments
- ✅ **Strings**: Double and single quoted strings with escape sequences
- ✅ **Numbers**: Integer literals (positive and negative)
- ✅ **Boolean literals**: `true`, `false`
- ✅ **Built-in functions**: `assert_eq`, `assert_bound`, `assert_unbound`, `member`, `append`

### Language Features
- ✅ **Relation definitions**: Function-like syntax highlighting
- ✅ **Constraint blocks**: Special syntax for CLP(FD) and CLP(Z) constraints
- ✅ **Meta programming**: `{...}` interpolation expressions
- ✅ **Pattern matching**: Match expressions with arrow syntax
- ✅ **Import statements**: `use` statements with module paths
- ✅ **Type annotations**: Primitive types and custom types
- ✅ **List syntax**: `[...]` with tail operator `|`
- ✅ **Fresh variables**: `|var|` syntax

### Editor Support
- ✅ **Auto-closing pairs**: Brackets, parentheses, quotes
- ✅ **Bracket matching**: Proper bracket pair highlighting
- ✅ **Comment toggling**: Ctrl+/ for line comments
- ✅ **Code folding**: Collapse/expand code blocks
- ✅ **Indentation**: Smart indentation rules

## 🧪 Testing the Extension

1. Open the `example.pv` file included with the extension
2. You should see proper syntax highlighting for:
   - Keywords in **blue** (rel, use, constraint, etc.)
   - Attributes in **yellow** (@test, @dfs, etc.)
   - Strings in **green**
   - Comments in **gray/green**
   - Numbers in **light blue**
   - Operators properly highlighted

## 📝 Example Code

The extension correctly highlights this Proto-Vulcan code:

```proto-vulcan
use std::list::*;

@test(expected = [1, 2, 3])
rel test_member(q) {
    member(q, [1, 2, 3])
}

@test(expected = [7])
rel test_interpolation(result) {
    let a: int = 3;
    let b: int = 4;
    result == {a + b}
}

rel constraint_example(x) {
    constraint(domain="clpfd") {
        x in 1..5,
        x != 3
    }
}
```

## 🛠️ Development

To modify or extend the extension:

1. **Edit syntax patterns**: Modify `syntaxes/proto-vulcan.tmLanguage.json`
2. **Add language features**: Update `language-configuration.json`
3. **Test changes**: Press F5 in VSCode to launch Extension Development Host
4. **Package extension**: Use `vsce package` (requires vsce CLI tool)

## 📋 Grammar Patterns Implemented

The TextMate grammar includes comprehensive patterns for:

- **Comments**: Line and block comments with TODO/FIXME highlighting
- **Attributes**: `@test`, `@dfs`, `@bfs` with parameter parsing
- **Keywords**: All Proto-Vulcan keywords and control structures
- **Literals**: Strings, numbers, booleans
- **Operators**: Comparison, assignment, and logical operators
- **Definitions**: Relations, structs, modules, implementations
- **Constraints**: Special constraint block syntax
- **Interpolation**: `{...}` meta expressions
- **Patterns**: Match patterns and list destructuring

## 🎯 Color Scope Mapping

The extension uses standard TextMate scopes that work with all VSCode themes:

- `keyword.control` → Control flow keywords
- `keyword.other` → Declaration keywords  
- `entity.name.function` → Relation names
- `entity.name.type` → Type names
- `string.quoted` → String literals
- `constant.numeric` → Number literals
- `comment.line` → Comments
- `punctuation.*` → Various punctuation
- `variable.parameter` → Function parameters

## 🔧 Troubleshooting

**Extension not working?**
1. Check VSCode extensions are enabled
2. Verify files have `.pv` extension
3. Restart VSCode after installation
4. Check VSCode Developer Tools (Help → Toggle Developer Tools) for errors

**Syntax highlighting incorrect?**
1. Open a `.pv` file
2. Check language mode in bottom-right (should say "Proto-Vulcan")
3. If not, click it and select "Proto-Vulcan" from the list

## 📄 License

MIT License - Feel free to modify and distribute! 