/// Symbol table implementation with string interning for Proto-Vulcan
///
/// This module provides efficient symbol management through a two-level interning system:
/// 1. String interning: Deduplicates string content using Rc<str>
/// 2. Symbol creation: Combines interned strings with source location information
///
/// AST nodes store Rc<Symbol> for direct access without table lookups.
use super::parser::ast::Location;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

/// A symbol represents an identifier with its source location and file information
#[derive(Debug, Clone, Eq)]
pub struct Symbol {
    /// The identifier text (interned for memory efficiency)
    pub text: Rc<str>,
    /// Source location where this symbol appears
    pub location: Location,
    /// File path where this symbol is defined (interned for memory efficiency)
    pub file_path: Rc<PathBuf>,
}

impl Symbol {
    /// Get the identifier text as a string slice
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the source span for this symbol
    pub fn span(&self) -> Location {
        self.location.clone()
    }

    /// Get the file path where this symbol appears
    pub fn file_path(&self) -> &PathBuf {
        &self.file_path
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

/// An interned symbol reference that provides a clean API for AST nodes
#[derive(Debug, Clone, Eq)]
pub struct InternedSymbol(Rc<Symbol>);

impl PartialEq for InternedSymbol {
    fn eq(&self, other: &Self) -> bool {
        // Compare only the text, not spans or file paths
        self.0.text == other.0.text
    }
}

impl InternedSymbol {
    /// Create a new symbol reference
    pub fn new(symbol: Rc<Symbol>) -> Self {
        Self(symbol)
    }

    /// Create a InternedSymbol from just text (no source location info)
    /// Useful for tests and when source location doesn't matter
    pub fn from_text(text: &str) -> Self {
        use std::rc::Rc;
        let symbol = Rc::new(Symbol {
            text: text.into(),
            location: crate::interpreter::parser::ast::Location::new(0, 0, 0, 0), // Empty span
            file_path: Rc::new(std::path::PathBuf::new()),                        // Empty path
        });
        Self(symbol)
    }

    /// Get the symbol text
    pub fn text(&self) -> &str {
        &self.0.text
    }

    /// Get the source span
    pub fn span(&self) -> Location {
        self.0.location.clone()
    }

    /// Get the source span as a reference
    pub fn span_ref(&self) -> &Location {
        &self.0.location
    }

    /// Get the file path
    pub fn file_path(&self) -> &PathBuf {
        &self.0.file_path
    }

    /// Get the underlying symbol
    pub fn symbol(&self) -> &Symbol {
        &self.0
    }

    /// Get the underlying Rc<Symbol>
    pub fn inner(&self) -> &Rc<Symbol> {
        &self.0
    }
}

impl std::fmt::Display for InternedSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.text)
    }
}

impl From<Rc<Symbol>> for InternedSymbol {
    fn from(symbol: Rc<Symbol>) -> Self {
        Self(symbol)
    }
}

impl PartialEq<str> for InternedSymbol {
    fn eq(&self, other: &str) -> bool {
        self.0.text.as_ref() == other
    }
}

impl PartialEq<&str> for InternedSymbol {
    fn eq(&self, other: &&str) -> bool {
        self.0.text.as_ref() == *other
    }
}

impl PartialEq<String> for InternedSymbol {
    fn eq(&self, other: &String) -> bool {
        self.0.text.as_ref() == other.as_str()
    }
}

impl PartialEq<Symbol> for InternedSymbol {
    fn eq(&self, other: &Symbol) -> bool {
        self.0.text == other.text
    }
}

impl PartialEq<InternedSymbol> for Symbol {
    fn eq(&self, other: &InternedSymbol) -> bool {
        self.text == other.0.text
    }
}

impl PartialEq<str> for Symbol {
    fn eq(&self, other: &str) -> bool {
        self.text.as_ref() == other
    }
}

impl PartialEq<String> for Symbol {
    fn eq(&self, other: &String) -> bool {
        self.text.as_ref() == other.as_str()
    }
}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl From<InternedSymbol> for String {
    fn from(symbol: InternedSymbol) -> String {
        symbol.0.text.to_string()
    }
}

impl From<&InternedSymbol> for String {
    fn from(symbol: &InternedSymbol) -> String {
        symbol.0.text.to_string()
    }
}

impl From<&str> for InternedSymbol {
    fn from(text: &str) -> Self {
        InternedSymbol::from_text(text)
    }
}

impl From<String> for InternedSymbol {
    fn from(text: String) -> Self {
        InternedSymbol::from_text(&text)
    }
}

impl AsRef<str> for InternedSymbol {
    fn as_ref(&self) -> &str {
        &self.0.text
    }
}

impl std::borrow::Borrow<str> for InternedSymbol {
    fn borrow(&self) -> &str {
        &self.0.text
    }
}

impl Ord for InternedSymbol {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.text.cmp(&other.0.text)
    }
}

impl PartialOrd for InternedSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Symbol {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.text.cmp(&other.text)
    }
}

impl PartialOrd for Symbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

impl std::hash::Hash for Symbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Only hash the text content, not the span or file path
        self.text.hash(state);
    }
}

impl std::hash::Hash for InternedSymbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Only hash the text content, not the span or file path
        self.0.text.hash(state);
    }
}

impl InternedSymbol {
    /// Temporary method to create a InternedSymbol from a string and span (TODO: remove when parser uses symbol table)
    pub fn from_string_and_span(
        text: String,
        span: crate::interpreter::parser::ast::Location,
    ) -> Self {
        use std::rc::Rc;
        let symbol = Rc::new(Symbol {
            text: text.as_str().into(),
            location: span,
            file_path: Rc::new(std::path::PathBuf::new()),
        });
        InternedSymbol::new(symbol)
    }

    /// Check if the symbol text contains a substring (useful for "::" checks)
    pub fn contains(&self, pattern: &str) -> bool {
        self.0.text.contains(pattern)
    }
}

/// Additional useful trait implementations for better ergonomics
impl std::ops::Deref for InternedSymbol {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0.text
    }
}

/// String interning table for deduplicating string content
#[derive(Debug)]
pub struct StringTable {
    strings: HashSet<Rc<str>>,
}

impl StringTable {
    /// Create a new empty string table
    pub fn new() -> Self {
        Self {
            strings: HashSet::new(),
        }
    }

    /// Intern a string, returning a shared reference
    pub fn intern(&mut self, text: &str) -> Rc<str> {
        if let Some(existing) = self.strings.get(text) {
            existing.clone()
        } else {
            let rc_str: Rc<str> = text.into();
            self.strings.insert(rc_str.clone());
            rc_str
        }
    }

    /// Get the number of unique strings interned
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if the string table is empty
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

impl Default for StringTable {
    fn default() -> Self {
        Self::new()
    }
}

/// File path interning table for deduplicating file paths
#[derive(Debug)]
pub struct FileTable {
    files: HashSet<Rc<PathBuf>>,
}

impl FileTable {
    /// Create a new empty file table
    pub fn new() -> Self {
        Self {
            files: HashSet::new(),
        }
    }

    /// Intern a file path, returning a shared reference
    pub fn intern(&mut self, path: PathBuf) -> Rc<PathBuf> {
        if let Some(existing) = self.files.get(&path) {
            existing.clone()
        } else {
            let rc_path = Rc::new(path.clone());
            self.files.insert(rc_path.clone());
            rc_path
        }
    }

    /// Get the number of unique file paths interned
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if the file table is empty
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

impl Default for FileTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Central symbol table that manages string and file interning
#[derive(Debug)]
pub struct SymbolTable {
    string_table: StringTable,
    file_table: FileTable,
}

impl SymbolTable {
    /// Create a new empty symbol table
    pub fn new() -> Self {
        Self {
            string_table: StringTable::new(),
            file_table: FileTable::new(),
        }
    }

    /// Create a new symbol with interned string and file path
    pub fn create_symbol(
        &mut self,
        text: &str,
        span: Location,
        file_path: PathBuf,
    ) -> InternedSymbol {
        let interned_text = self.string_table.intern(text);
        let interned_file = self.file_table.intern(file_path);

        let symbol = Rc::new(Symbol {
            text: interned_text,
            location: span,
            file_path: interned_file,
        });

        InternedSymbol::new(symbol)
    }

    /// Get statistics about the symbol table
    pub fn stats(&self) -> SymbolTableStats {
        SymbolTableStats {
            unique_strings: self.string_table.len(),
            unique_files: self.file_table.len(),
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about symbol table usage
#[derive(Debug, Clone)]
pub struct SymbolTableStats {
    pub unique_strings: usize,
    pub unique_files: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_interning() {
        let mut table = StringTable::new();

        let str1 = table.intern("hello");
        let str2 = table.intern("hello");
        let str3 = table.intern("world");

        // Same string should return the same Rc
        assert!(Rc::ptr_eq(&str1, &str2));
        assert!(!Rc::ptr_eq(&str1, &str3));

        // Should only have 2 unique strings
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn test_file_interning() {
        let mut table = FileTable::new();

        let file1 = table.intern(PathBuf::from("test.pv"));
        let file2 = table.intern(PathBuf::from("test.pv"));
        let file3 = table.intern(PathBuf::from("other.pv"));

        // Same file should return the same Rc
        assert!(Rc::ptr_eq(&file1, &file2));
        assert!(!Rc::ptr_eq(&file1, &file3));

        // Should only have 2 unique files
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn test_symbol_creation() {
        let mut table = SymbolTable::new();

        let symbol1 =
            table.create_symbol("main", Location::new(0, 0, 0, 4), PathBuf::from("test.pv"));
        let symbol2 = table.create_symbol(
            "main",
            Location::new(0, 0, 10, 14),
            PathBuf::from("test.pv"),
        );
        let symbol3 =
            table.create_symbol("other", Location::new(0, 0, 0, 5), PathBuf::from("test.pv"));

        // Symbols with same text should be equal (text-only comparison)
        assert_eq!(symbol1, symbol2);
        assert_ne!(symbol1, symbol3);

        // But they should share interned strings and files
        assert!(Rc::ptr_eq(&symbol1.symbol().text, &symbol2.symbol().text));
        // Note: file_path() returns &PathBuf, not &Rc<PathBuf>, so we can't compare Rc pointers directly
        // The interning is happening at the symbol level

        // Different text should not share strings
        assert!(!Rc::ptr_eq(&symbol1.symbol().text, &symbol3.symbol().text));
    }

    #[test]
    fn test_symbol_stats() {
        let mut table = SymbolTable::new();

        table.create_symbol("main", Location::new(0, 0, 0, 4), PathBuf::from("test.pv"));
        table.create_symbol(
            "main",
            Location::new(0, 0, 10, 14),
            PathBuf::from("test.pv"),
        );
        table.create_symbol(
            "other",
            Location::new(0, 0, 0, 5),
            PathBuf::from("other.pv"),
        );

        let stats = table.stats();
        assert_eq!(stats.unique_strings, 2); // "main" and "other"
        assert_eq!(stats.unique_files, 2); // "test.pv" and "other.pv"
    }

    #[test]
    fn test_symbol_comparison() {
        let mut table = SymbolTable::new();

        // Create two symbols with same text but different locations/files
        let symbol1 =
            table.create_symbol("test", Location::new(0, 0, 0, 4), PathBuf::from("file1.pv"));
        let symbol2 = table.create_symbol(
            "test",
            Location::new(0, 0, 10, 14),
            PathBuf::from("file2.pv"),
        );
        let symbol3 = table.create_symbol(
            "other",
            Location::new(0, 0, 0, 5),
            PathBuf::from("file1.pv"),
        );

        // Symbols with same text should be equal regardless of location/file
        assert_eq!(symbol1, symbol2);
        assert_eq!(symbol2, symbol1);

        // Symbols with different text should not be equal
        assert_ne!(symbol1, symbol3);
        assert_ne!(symbol2, symbol3);

        // Test comparison with strings
        assert_eq!(symbol1, "test");
        assert_eq!(symbol1, "test".to_string());
        assert_ne!(symbol1, "other");

        // Test cross-type comparisons
        assert_eq!(symbol1.symbol(), symbol2.symbol());
        assert_eq!(*symbol1.symbol(), *symbol2.symbol());
    }
}
