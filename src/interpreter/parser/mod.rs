use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

pub mod ast;
pub mod builder;
pub mod meta_parser;
use ast::*;
use builder::{AstBuilder, ParseResult};
use crate::interpreter::symbol_table::SymbolTable;

#[derive(Parser)]
#[grammar = "interpreter/parser/grammar.pest"]
pub struct VulcanParser;

/// Public API for parsing Proto-Vulcan source code
pub fn parse_str(input: &str) -> ParseResult<Program> {
    parse_str_with_file(input, &std::path::PathBuf::from("<input>"))
}

/// Public API for parsing Proto-Vulcan source code with file path
pub fn parse_str_with_file(input: &str, file_path: &std::path::PathBuf) -> ParseResult<Program> {
    let mut symbol_table = SymbolTable::new();
    parse_str_with_symbols(input, file_path, &mut symbol_table)
}

/// Public API for parsing Proto-Vulcan source code with existing symbol table
pub fn parse_str_with_symbols(
    input: &str,
    file_path: &std::path::PathBuf,
    symbol_table: &mut SymbolTable,
) -> ParseResult<Program> {
    let pairs = VulcanParser::parse(Rule::program, input)?;
    let mut builder = AstBuilder::new(symbol_table, file_path);
    let program = builder.build_program(pairs.into_iter().next().unwrap())?;
    Ok(program)
}

/// Public API for building a Goal from a Pair (used by query parser)
pub fn build_goal(pair: Pair<Rule>) -> ParseResult<Goal> {
    build_goal_with_file(pair, &std::path::PathBuf::from("<query>"))
}

/// Public API for building a Goal from a Pair with file path
pub fn build_goal_with_file(pair: Pair<Rule>, file_path: &std::path::PathBuf) -> ParseResult<Goal> {
    let mut symbol_table = SymbolTable::new();
    build_goal_with_symbols(pair, file_path, &mut symbol_table)
}

/// Public API for building a Goal from a Pair with existing symbol table
pub fn build_goal_with_symbols(
    pair: Pair<Rule>,
    file_path: &std::path::PathBuf,
    symbol_table: &mut SymbolTable,
) -> ParseResult<Goal> {
    let mut builder = AstBuilder::new(symbol_table, file_path);
    builder.build_goal(pair)
}

#[cfg(test)]
mod tests;