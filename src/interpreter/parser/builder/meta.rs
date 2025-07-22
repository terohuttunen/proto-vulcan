use pest::iterators::Pair;
use super::{AstBuilder, ParseError, ParseResult, Rule};
use crate::interpreter::parser::{ast::*, meta_parser};
use crate::interpreter::metaprogramming::{MetaStatement, TypeAnnotation, LetStatement, MetaForRange};

impl<'a> AstBuilder<'a> {
    pub fn build_meta_statement(&mut self, pair: Pair<Rule>) -> ParseResult<MetaStatement> {
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::meta_let_statement => Ok(MetaStatement::Let(self.build_meta_let_statement(inner)?)),
            Rule::meta_if_statement => {
                let (condition, then_body, else_ifs, else_body) = self.build_meta_if_statement(inner)?;
                Ok(MetaStatement::If {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                })
            }
            Rule::meta_for_statement => {
                let (variable, variable_type, range, body) = self.build_meta_for_statement(inner)?;
                Ok(MetaStatement::For {
                    variable,
                    variable_type,
                    range,
                    body,
                })
            }
            _ => Err(ParseError::UnexpectedRule(inner.as_rule())),
        }
    }

    pub fn build_meta_let_statement(&mut self, pair: Pair<Rule>) -> ParseResult<LetStatement> {
        let span = self.pair_to_span(&pair);
        let mut inner = pair.into_inner();
        let variable_pair = inner.next().unwrap();
        let variable = self.create_symbol_from_pair(&variable_pair);
        let variable_type = self.parse_type_annotation(inner.next().unwrap().as_str())?;
        let content = inner.next().unwrap().as_str();
        let expression = meta_parser::parse_meta_expression(content, &span)
            .map_err(|_| ParseError::UnexpectedRule(Rule::meta_let_statement))?;

        Ok(LetStatement {
            variable,
            variable_type,
            expression,
        })
    }

    pub fn build_meta_if_statement(&mut self, pair: Pair<Rule>) -> ParseResult<(
        crate::interpreter::metaprogramming::MetaExpression,
        GoalBody,
        Vec<(
            crate::interpreter::metaprogramming::MetaExpression,
            GoalBody,
        )>,
        Option<GoalBody>,
    )> {
        let span = self.pair_to_span(&pair);
        let mut inner = pair.into_inner();

        // Parse initial if condition and body
        let content = inner.next().unwrap().as_str();
        let condition = meta_parser::parse_meta_expression(content, &span)
            .map_err(|_| ParseError::UnexpectedRule(Rule::meta_if_statement))?;
        let then_body = self.build_goal_body(inner.next().unwrap())?;

        // Parse remaining elements (else if clauses and final else)
        let mut else_ifs = Vec::new();
        let mut else_body = None;

        while let Some(element) = inner.next() {
            match element.as_rule() {
                Rule::meta_expr_content => {
                    // This should be an else if condition
                    let else_if_condition = meta_parser::parse_meta_expression(element.as_str(), &span)
                        .map_err(|_| ParseError::UnexpectedRule(Rule::meta_if_statement))?;
                    let else_if_body = self.build_goal_body(inner.next().unwrap())?;
                    else_ifs.push((else_if_condition, else_if_body));
                }
                Rule::goal_body => {
                    // This should be the final else body
                    else_body = Some(self.build_goal_body(element)?);
                    break;
                }
                _ => {
                    // Skip keywords like "else", "if"
                    continue;
                }
            }
        }

        Ok((condition, then_body, else_ifs, else_body))
    }

    pub fn build_meta_for_statement(&mut self, pair: Pair<Rule>) -> ParseResult<(
        crate::interpreter::symbol_table::InternedSymbol,
        TypeAnnotation,
        MetaForRange,
        GoalBody,
    )> {
        let mut inner = pair.into_inner();
        let variable_pair = inner.next().unwrap();
        let variable = self.create_symbol_from_pair(&variable_pair);
        let variable_type = self.parse_type_annotation(inner.next().unwrap().as_str())?;
        let range_pair = inner.next().unwrap(); // meta_for_range
        let range = self.build_meta_for_range(range_pair)?;
        let body = self.build_goal_body(inner.next().unwrap())?;

        Ok((variable, variable_type, range, body))
    }

    pub fn build_meta_for_range(&mut self, pair: Pair<Rule>) -> ParseResult<MetaForRange> {
        let span = self.pair_to_span(&pair);
        let mut inner = pair.into_inner();
        let start_content = inner.next().unwrap().as_str();
        let end_content = inner.next().unwrap().as_str();

        let start = meta_parser::parse_meta_expression(start_content, &span)
            .map_err(|_| ParseError::UnexpectedRule(Rule::meta_for_range))?;
        let end = meta_parser::parse_meta_expression(end_content, &span)
            .map_err(|_| ParseError::UnexpectedRule(Rule::meta_for_range))?;

        Ok(MetaForRange { start, end })
    }

    pub fn parse_meta_type_annotation(&mut self, pair: Pair<Rule>) -> ParseResult<TypeAnnotation> {
        // Save the string representation before consuming the pair
        let pair_str = pair.as_str();

        // Check if there are any inner pairs (for complex types like relation_type)
        let mut inner_pairs = pair.into_inner();
        if let Some(inner) = inner_pairs.next() {
            // We have an inner rule (like relation_type or type_name)
            match inner.as_rule() {
                Rule::relation_type => {
                    let arity_pair = inner.into_inner().next().unwrap();
                    let arity_str = arity_pair.as_str();
                    let arity = arity_str
                        .parse::<usize>()
                        .map_err(|_| ParseError::UnexpectedRule(Rule::number_literal))?;
                    Ok(TypeAnnotation::Relation(arity))
                }
                Rule::type_name => {
                    // Handle type_name (qualified paths and simple identifiers)
                    let type_name = self.build_type_name(inner)?;
                    Ok(TypeAnnotation::Custom(type_name))
                }
                _ => self.parse_type_annotation(inner.as_str()),
            }
        } else {
            // No inner pairs - this is a terminal type (int, string, bool)
            self.parse_type_annotation(pair_str)
        }
    }

    pub fn parse_type_annotation(&mut self, type_str: &str) -> ParseResult<TypeAnnotation> {
        match type_str {
            "int" => Ok(TypeAnnotation::Int),
            "string" => Ok(TypeAnnotation::String),
            "bool" => Ok(TypeAnnotation::Bool),
            _ => Err(ParseError::UnexpectedRule(Rule::ident)), // For now, reject unknown types
        }
    }
}