use pest::iterators::Pair;
use thiserror::Error;

use super::ast::*;

// Re-export builder modules
pub mod attributes;
pub mod goals;
pub mod imports;
pub mod items;
pub mod meta;
pub mod relations;
pub mod terms;

// Import the VulcanParser Rule enum from parent
use super::Rule;

/// Error type for parsing operations
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Pest error: {0}")]
    Pest(#[from] pest::error::Error<Rule>),
    #[error("Unexpected rule: {0:?}")]
    UnexpectedRule(Rule),
    #[error("Missing rule: {0:?}")]
    MissingRule(Rule),
}

pub type ParseResult<T> = Result<T, ParseError>;

/// AST builder that contains methods for converting Pest parse tree nodes into AST nodes
pub struct AstBuilder<'a> {
    /// Symbol table for interning identifiers with source location information
    symbol_table: &'a mut crate::interpreter::symbol_table::SymbolTable,
    /// Current file path being parsed (for source location tracking)
    current_file: &'a std::path::PathBuf,
}

impl<'a> AstBuilder<'a> {
    /// Create a new AstBuilder instance with symbol table and file path
    pub fn new(
        symbol_table: &'a mut crate::interpreter::symbol_table::SymbolTable,
        current_file: &'a std::path::PathBuf,
    ) -> Self {
        Self {
            symbol_table,
            current_file,
        }
    }

    /// Helper function to extract span information from a Pest pair
    pub fn pair_to_span(&self, pair: &Pair<Rule>) -> Span {
        let span = pair.as_span();
        Span::new(span.start(), span.end())
    }

    /// Create an interned symbol from text and span
    pub fn create_symbol(
        &mut self,
        text: &str,
        span: Span,
    ) -> crate::interpreter::symbol_table::InternedSymbol {
        self.symbol_table
            .create_symbol(text, span, self.current_file.clone())
    }

    /// Extract text from a Pest pair
    pub fn extract_text_from_pair(pair: &Pair<Rule>) -> String {
        pair.as_str().to_string()
    }

    /// Create an interned symbol from a Pest pair (extracts text and span automatically)
    pub fn create_symbol_from_pair(
        &mut self,
        pair: &Pair<Rule>,
    ) -> crate::interpreter::symbol_table::InternedSymbol {
        let text = self.extract_clean_text_from_pair(pair);
        let span = self.pair_to_span(pair);
        self.create_symbol(&text, span)
    }

    /// Extract text from a Pest pair - no cleaning, just raw text
    fn extract_clean_text_from_pair(&self, pair: &Pair<Rule>) -> String {
        pair.as_str().to_string()
    }
}
