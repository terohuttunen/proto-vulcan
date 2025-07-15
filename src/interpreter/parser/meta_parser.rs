use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use thiserror::Error;

use crate::interpreter::metaprogramming::{MetaBinaryOp, MetaExpression, MetaValue};
use crate::interpreter::parser::ast::Span;

#[derive(Parser)]
#[grammar = "interpreter/parser/meta_expr.pest"]
pub struct MetaExpressionParser;

#[derive(Error, Debug)]
pub enum MetaParseError {
    #[error("Pest error: {0}")]
    Pest(#[from] pest::error::Error<Rule>),
    #[error("Unexpected rule: {0:?}")]
    UnexpectedRule(Rule),
    #[error("Missing rule: {0:?}")]
    MissingRule(Rule),
}

type MetaParseResult<T> = Result<T, MetaParseError>;

/// Helper function to map meta expression positions to source positions
pub fn map_meta_position_to_source(meta_pos: usize, source_span: &Span) -> usize {
    source_span.start + meta_pos
}

/// Helper function to create a span for meta expression content within source context
pub fn create_meta_span(meta_start: usize, meta_end: usize, source_span: &Span) -> Span {
    let source_start = map_meta_position_to_source(meta_start, source_span);
    let source_end = map_meta_position_to_source(meta_end, source_span);
    Span::new(source_start, source_end)
}

/// Parse a meta expression from a string with source span context
pub fn parse_meta_expression(input: &str, source_span: &Span) -> MetaParseResult<MetaExpression> {
    let pairs = MetaExpressionParser::parse(Rule::meta_expression, input)?;
    let expr_pair = pairs.into_iter().next().unwrap();
    build_meta_expression(expr_pair.into_inner().next().unwrap(), source_span)
}

/// Backwards-compatible wrapper that uses dummy spans

/// Build a meta expression from a pest pair with source span context
pub fn build_meta_expression(
    pair: Pair<Rule>,
    source_span: &Span,
) -> MetaParseResult<MetaExpression> {
    match pair.as_rule() {
        Rule::meta_or_expression => {
            let span = create_meta_span(pair.as_span().start(), pair.as_span().end(), source_span);
            let mut inner = pair.into_inner();
            let mut expr = build_meta_expression(inner.next().unwrap(), source_span)?;

            for next_pair in inner {
                let right = build_meta_expression(next_pair, source_span)?;
                expr = MetaExpression::BinaryOp(
                    MetaBinaryOp::Or,
                    Box::new(expr),
                    Box::new(right),
                    span.clone(),
                );
            }
            Ok(expr)
        }

        Rule::meta_and_expression => {
            let span = create_meta_span(pair.as_span().start(), pair.as_span().end(), source_span);
            let mut inner = pair.into_inner();
            let mut expr = build_meta_expression(inner.next().unwrap(), source_span)?;

            for next_pair in inner {
                let right = build_meta_expression(next_pair, source_span)?;
                expr = MetaExpression::BinaryOp(
                    MetaBinaryOp::And,
                    Box::new(expr),
                    Box::new(right),
                    span.clone(),
                );
            }
            Ok(expr)
        }

        Rule::meta_equality_expression => {
            let rule = pair.as_rule();
            let original_str = pair.as_str();
            let span = create_meta_span(pair.as_span().start(), pair.as_span().end(), source_span);
            let inner_pairs: Vec<_> = pair.into_inner().collect();

            if inner_pairs.len() == 1 {
                return build_meta_expression(inner_pairs[0].clone(), source_span);
            }

            let mut expr = build_meta_expression(inner_pairs[0].clone(), source_span)?;

            let mut i = 1;
            while i < inner_pairs.len() {
                let right = build_meta_expression(inner_pairs[i].clone(), source_span)?;

                // Determine the operator by looking at the original string
                let op = if original_str.contains("!=") {
                    MetaBinaryOp::NotEqual
                } else if original_str.contains("==") {
                    MetaBinaryOp::Equal
                } else {
                    return Err(MetaParseError::UnexpectedRule(rule));
                };

                expr = MetaExpression::BinaryOp(op, Box::new(expr), Box::new(right), span.clone());
                i += 1;
            }
            Ok(expr)
        }

        Rule::meta_comparison_expression => {
            let rule = pair.as_rule();
            let original_str = pair.as_str();
            let span = create_meta_span(pair.as_span().start(), pair.as_span().end(), source_span);
            let inner_pairs: Vec<_> = pair.into_inner().collect();

            if inner_pairs.len() == 1 {
                return build_meta_expression(inner_pairs[0].clone(), source_span);
            }

            let mut expr = build_meta_expression(inner_pairs[0].clone(), source_span)?;

            let mut i = 1;
            while i < inner_pairs.len() {
                let right = build_meta_expression(inner_pairs[i].clone(), source_span)?;

                // Determine the operator by looking at the original string
                let op = if original_str.contains("<=") {
                    MetaBinaryOp::LessEqual
                } else if original_str.contains(">=") {
                    MetaBinaryOp::GreaterEqual
                } else if original_str.contains("<") {
                    MetaBinaryOp::LessThan
                } else if original_str.contains(">") {
                    MetaBinaryOp::GreaterThan
                } else {
                    return Err(MetaParseError::UnexpectedRule(rule));
                };

                expr = MetaExpression::BinaryOp(op, Box::new(expr), Box::new(right), span.clone());
                i += 1;
            }
            Ok(expr)
        }

        Rule::meta_additive_expression => {
            let rule = pair.as_rule();
            let original_str = pair.as_str();
            let span = create_meta_span(pair.as_span().start(), pair.as_span().end(), source_span);
            let inner_pairs: Vec<_> = pair.into_inner().collect();

            if inner_pairs.len() == 1 {
                return build_meta_expression(inner_pairs[0].clone(), source_span);
            }

            let mut expr = build_meta_expression(inner_pairs[0].clone(), source_span)?;

            let mut i = 1;
            while i < inner_pairs.len() {
                let right = build_meta_expression(inner_pairs[i].clone(), source_span)?;

                // Determine the operator by looking at the original string
                let op = if original_str.contains("+") {
                    MetaBinaryOp::Add
                } else if original_str.contains("-") {
                    MetaBinaryOp::Subtract
                } else {
                    return Err(MetaParseError::UnexpectedRule(rule));
                };

                expr = MetaExpression::BinaryOp(op, Box::new(expr), Box::new(right), span.clone());
                i += 1;
            }
            Ok(expr)
        }

        Rule::meta_multiplicative_expression => {
            let rule = pair.as_rule();
            let original_str = pair.as_str();
            let span = create_meta_span(pair.as_span().start(), pair.as_span().end(), source_span);
            let inner_pairs: Vec<_> = pair.into_inner().collect();

            if inner_pairs.len() == 1 {
                return build_meta_expression(inner_pairs[0].clone(), source_span);
            }

            let mut expr = build_meta_expression(inner_pairs[0].clone(), source_span)?;

            let mut i = 1;
            while i < inner_pairs.len() {
                let right = build_meta_expression(inner_pairs[i].clone(), source_span)?;

                // Determine the operator by looking at the original string
                let op = if original_str.contains("*") {
                    MetaBinaryOp::Multiply
                } else if original_str.contains("/") {
                    MetaBinaryOp::Divide
                } else {
                    return Err(MetaParseError::UnexpectedRule(rule));
                };

                expr = MetaExpression::BinaryOp(op, Box::new(expr), Box::new(right), span.clone());
                i += 1;
            }
            Ok(expr)
        }

        // meta_range_expression removed - ranges are no longer meta expressions
        Rule::meta_primary_expression => {
            let inner = pair.into_inner().next().unwrap();
            build_meta_expression(inner, source_span)
        }

        Rule::meta_literal => {
            let span = create_meta_span(pair.as_span().start(), pair.as_span().end(), source_span);
            let inner = pair.into_inner().next().unwrap();
            match inner.as_rule() {
                Rule::meta_integer_literal => {
                    let value = inner
                        .as_str()
                        .parse::<i64>()
                        .map_err(|_| MetaParseError::UnexpectedRule(Rule::meta_integer_literal))?;
                    Ok(MetaExpression::Literal(MetaValue::Integer(value), span))
                }
                Rule::meta_string_literal => {
                    let value = inner.as_str();
                    // Remove quotes
                    let unquoted = &value[1..value.len() - 1];
                    Ok(MetaExpression::Literal(
                        MetaValue::String(unquoted.to_string()),
                        span,
                    ))
                }
                Rule::meta_boolean_literal => {
                    let value = inner
                        .as_str()
                        .parse::<bool>()
                        .map_err(|_| MetaParseError::UnexpectedRule(Rule::meta_boolean_literal))?;
                    Ok(MetaExpression::Literal(MetaValue::Boolean(value), span))
                }
                _ => Err(MetaParseError::UnexpectedRule(inner.as_rule())),
            }
        }

        Rule::meta_variable => {
            let span = create_meta_span(pair.as_span().start(), pair.as_span().end(), source_span);
            Ok(MetaExpression::Variable(pair.as_str().to_string(), span))
        }

        _ => Err(MetaParseError::UnexpectedRule(pair.as_rule())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_variable() {
        let expr = parse_meta_expression("x", &Span::new(0, 1)).unwrap();
        assert_eq!(
            expr,
            MetaExpression::Variable("x".to_string(), Span::new(0, 1))
        );
    }

    #[test]
    fn test_parse_integer_literal() {
        let expr = parse_meta_expression("42", &Span::new(0, 2)).unwrap();
        assert_eq!(
            expr,
            MetaExpression::Literal(MetaValue::Integer(42), Span::new(0, 2))
        );
    }

    #[test]
    fn test_parse_string_literal() {
        let expr = parse_meta_expression("\"hello\"", &Span::new(0, 8)).unwrap();
        assert_eq!(
            expr,
            MetaExpression::Literal(MetaValue::String("hello".to_string()), Span::new(0, 8))
        );
    }

    #[test]
    fn test_parse_boolean_literal() {
        let expr = parse_meta_expression("true", &Span::new(0, 4)).unwrap();
        assert_eq!(
            expr,
            MetaExpression::Literal(MetaValue::Boolean(true), Span::new(0, 4))
        );
    }

    #[test]
    fn test_parse_addition() {
        let expr = parse_meta_expression("x + 1", &Span::new(0, 5)).unwrap();
        match expr {
            MetaExpression::BinaryOp(MetaBinaryOp::Add, left, right, _) => {
                assert_eq!(
                    *left,
                    MetaExpression::Variable("x".to_string(), Span::new(0, 1))
                );
                assert_eq!(
                    *right,
                    MetaExpression::Literal(MetaValue::Integer(1), Span::new(4, 5))
                );
            }
            _ => panic!("Expected addition expression"),
        }
    }

    // Range test removed - ranges are no longer meta expressions

    #[test]
    fn test_parse_comparison() {
        let expr = parse_meta_expression("x < 5", &Span::new(0, 5)).unwrap();
        match expr {
            MetaExpression::BinaryOp(MetaBinaryOp::LessThan, left, right, _) => {
                assert_eq!(
                    *left,
                    MetaExpression::Variable("x".to_string(), Span::new(0, 1))
                );
                assert_eq!(
                    *right,
                    MetaExpression::Literal(MetaValue::Integer(5), Span::new(4, 5))
                );
            }
            _ => panic!("Expected comparison expression"),
        }
    }

    #[test]
    fn test_parse_parentheses() {
        let expr = parse_meta_expression("(x + 1) * 2", &Span::new(0, 10)).unwrap();
        match expr {
            MetaExpression::BinaryOp(MetaBinaryOp::Multiply, left, right, _) => {
                match *left {
                    MetaExpression::BinaryOp(MetaBinaryOp::Add, _, _, _) => {}
                    _ => panic!("Expected addition in left operand"),
                }
                assert_eq!(
                    *right,
                    MetaExpression::Literal(MetaValue::Integer(2), Span::new(8, 9))
                );
            }
            _ => panic!("Expected multiplication expression"),
        }
    }
}
