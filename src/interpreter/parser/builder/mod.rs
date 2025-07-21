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
    // Currently no state needed, but the mutable reference allows for future extensibility
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> AstBuilder<'a> {
    /// Create a new AstBuilder instance
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Helper function to extract span information from a Pest pair
    pub fn pair_to_span(&self, pair: &Pair<Rule>) -> Span {
        let span = pair.as_span();
        Span::new(span.start(), span.end())
    }
}

impl<'a> Default for AstBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}