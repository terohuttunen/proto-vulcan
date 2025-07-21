use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

pub mod ast;
pub mod builder;
pub mod meta_parser;
use ast::*;
use builder::{AstBuilder, ParseResult};

#[derive(Parser)]
#[grammar = "interpreter/parser/grammar.pest"]
pub struct VulcanParser;

/// Public API for parsing Proto-Vulcan source code
pub fn parse_str(input: &str) -> ParseResult<Program> {
    let pairs = VulcanParser::parse(Rule::program, input)?;
    let mut builder = AstBuilder::new();
    let program = builder.build_program(pairs.into_iter().next().unwrap())?;
    Ok(program)
}

/// Public API for building a Goal from a Pair (used by query parser)
pub fn build_goal(pair: Pair<Rule>) -> ParseResult<Goal> {
    let mut builder = AstBuilder::new();
    builder.build_goal(pair)
}

#[cfg(test)]
mod tests;