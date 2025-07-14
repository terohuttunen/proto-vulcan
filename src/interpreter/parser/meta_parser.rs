use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use thiserror::Error;

use crate::interpreter::metaprogramming::{MetaBinaryOp, MetaExpression, MetaValue};

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

/// Parse a meta expression from a string
pub fn parse_meta_expression(input: &str) -> MetaParseResult<MetaExpression> {
    let pairs = MetaExpressionParser::parse(Rule::meta_expression, input)?;
    let expr_pair = pairs.into_iter().next().unwrap();
    build_meta_expression(expr_pair.into_inner().next().unwrap())
}

/// Build a meta expression from a pest pair
pub fn build_meta_expression(pair: Pair<Rule>) -> MetaParseResult<MetaExpression> {
    match pair.as_rule() {
        Rule::meta_or_expression => {
            let mut inner = pair.into_inner();
            let mut expr = build_meta_expression(inner.next().unwrap())?;

            for next_pair in inner {
                let right = build_meta_expression(next_pair)?;
                expr = MetaExpression::BinaryOp(MetaBinaryOp::Or, Box::new(expr), Box::new(right));
            }
            Ok(expr)
        }

        Rule::meta_and_expression => {
            let mut inner = pair.into_inner();
            let mut expr = build_meta_expression(inner.next().unwrap())?;

            for next_pair in inner {
                let right = build_meta_expression(next_pair)?;
                expr = MetaExpression::BinaryOp(MetaBinaryOp::And, Box::new(expr), Box::new(right));
            }
            Ok(expr)
        }

        Rule::meta_equality_expression => {
            let rule = pair.as_rule();
            let original_str = pair.as_str();
            let inner_pairs: Vec<_> = pair.into_inner().collect();

            if inner_pairs.is_empty() {
                return Err(MetaParseError::UnexpectedRule(rule));
            }

            let mut expr = build_meta_expression(inner_pairs[0].clone())?;

            let mut i = 1;
            while i < inner_pairs.len() {
                let right = build_meta_expression(inner_pairs[i].clone())?;

                // Determine the operator by looking at the original string
                let left_str = inner_pairs[i - 1].as_str();
                let right_str = inner_pairs[i].as_str();

                let left_end = original_str.find(left_str).unwrap() + left_str.len();
                let right_start = original_str.rfind(right_str).unwrap();
                let operator_section = &original_str[left_end..right_start];

                let op = if operator_section.contains("!=") {
                    MetaBinaryOp::NotEqual
                } else if operator_section.contains("==") {
                    MetaBinaryOp::Equal
                } else {
                    return Err(MetaParseError::UnexpectedRule(rule));
                };

                expr = MetaExpression::BinaryOp(op, Box::new(expr), Box::new(right));
                i += 1;
            }
            Ok(expr)
        }

        Rule::meta_comparison_expression => {
            let rule = pair.as_rule();
            let original_str = pair.as_str();
            let inner_pairs: Vec<_> = pair.into_inner().collect();

            if inner_pairs.is_empty() {
                return Err(MetaParseError::UnexpectedRule(rule));
            }

            let mut expr = build_meta_expression(inner_pairs[0].clone())?;

            let mut i = 1;
            while i < inner_pairs.len() {
                let right = build_meta_expression(inner_pairs[i].clone())?;

                // Determine the operator by looking at the original string
                let left_str = inner_pairs[i - 1].as_str();
                let right_str = inner_pairs[i].as_str();

                let left_end = original_str.find(left_str).unwrap() + left_str.len();
                let right_start = original_str.rfind(right_str).unwrap();
                let operator_section = &original_str[left_end..right_start];

                let op = if operator_section.contains("<=") {
                    MetaBinaryOp::LessEqual
                } else if operator_section.contains(">=") {
                    MetaBinaryOp::GreaterEqual
                } else if operator_section.contains('<') {
                    MetaBinaryOp::LessThan
                } else if operator_section.contains('>') {
                    MetaBinaryOp::GreaterThan
                } else {
                    return Err(MetaParseError::UnexpectedRule(rule));
                };

                expr = MetaExpression::BinaryOp(op, Box::new(expr), Box::new(right));
                i += 1;
            }
            Ok(expr)
        }

        Rule::meta_additive_expression => {
            let rule = pair.as_rule();
            let original_str = pair.as_str();
            let inner_pairs: Vec<_> = pair.into_inner().collect();

            if inner_pairs.is_empty() {
                return Err(MetaParseError::UnexpectedRule(rule));
            }

            let mut expr = build_meta_expression(inner_pairs[0].clone())?;

            let mut i = 1;
            while i < inner_pairs.len() {
                let right = build_meta_expression(inner_pairs[i].clone())?;

                // Determine the operator by looking at the original string
                let left_str = inner_pairs[i - 1].as_str();
                let right_str = inner_pairs[i].as_str();

                let left_end = original_str.find(left_str).unwrap() + left_str.len();
                let right_start = original_str.rfind(right_str).unwrap();
                let operator_section = &original_str[left_end..right_start];

                let op = if operator_section.contains('+') {
                    MetaBinaryOp::Add
                } else if operator_section.contains('-') {
                    MetaBinaryOp::Subtract
                } else {
                    return Err(MetaParseError::UnexpectedRule(rule));
                };

                expr = MetaExpression::BinaryOp(op, Box::new(expr), Box::new(right));
                i += 1;
            }
            Ok(expr)
        }

        Rule::meta_multiplicative_expression => {
            let rule = pair.as_rule();
            let original_str = pair.as_str();
            let inner_pairs: Vec<_> = pair.into_inner().collect();

            if inner_pairs.is_empty() {
                return Err(MetaParseError::UnexpectedRule(rule));
            }

            let mut expr = build_meta_expression(inner_pairs[0].clone())?;

            let mut i = 1;
            while i < inner_pairs.len() {
                let right = build_meta_expression(inner_pairs[i].clone())?;

                // Determine the operator by looking at the original string
                let left_str = inner_pairs[i - 1].as_str();
                let right_str = inner_pairs[i].as_str();

                let left_end = original_str.find(left_str).unwrap() + left_str.len();
                let right_start = original_str.rfind(right_str).unwrap();
                let operator_section = &original_str[left_end..right_start];

                let op = if operator_section.contains('*') {
                    MetaBinaryOp::Multiply
                } else if operator_section.contains('/') {
                    MetaBinaryOp::Divide
                } else {
                    return Err(MetaParseError::UnexpectedRule(rule));
                };

                expr = MetaExpression::BinaryOp(op, Box::new(expr), Box::new(right));
                i += 1;
            }
            Ok(expr)
        }

        // meta_range_expression removed - ranges are no longer meta expressions
        Rule::meta_primary_expression => {
            let inner = pair.into_inner().next().unwrap();
            build_meta_expression(inner)
        }

        Rule::meta_literal => {
            let inner = pair.into_inner().next().unwrap();
            match inner.as_rule() {
                Rule::meta_integer_literal => {
                    let value = inner
                        .as_str()
                        .parse::<i64>()
                        .map_err(|_| MetaParseError::UnexpectedRule(Rule::meta_integer_literal))?;
                    Ok(MetaExpression::Literal(MetaValue::Integer(value)))
                }
                Rule::meta_string_literal => {
                    let value = inner.as_str();
                    // Remove quotes
                    let unquoted = &value[1..value.len() - 1];
                    Ok(MetaExpression::Literal(MetaValue::String(
                        unquoted.to_string(),
                    )))
                }
                Rule::meta_boolean_literal => {
                    let value = inner
                        .as_str()
                        .parse::<bool>()
                        .map_err(|_| MetaParseError::UnexpectedRule(Rule::meta_boolean_literal))?;
                    Ok(MetaExpression::Literal(MetaValue::Boolean(value)))
                }
                _ => Err(MetaParseError::UnexpectedRule(inner.as_rule())),
            }
        }

        Rule::meta_variable => Ok(MetaExpression::Variable(pair.as_str().to_string())),

        _ => Err(MetaParseError::UnexpectedRule(pair.as_rule())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_variable() {
        let expr = parse_meta_expression("x").unwrap();
        assert_eq!(expr, MetaExpression::Variable("x".to_string()));
    }

    #[test]
    fn test_parse_integer_literal() {
        let expr = parse_meta_expression("42").unwrap();
        assert_eq!(expr, MetaExpression::Literal(MetaValue::Integer(42)));
    }

    #[test]
    fn test_parse_string_literal() {
        let expr = parse_meta_expression("\"hello\"").unwrap();
        assert_eq!(
            expr,
            MetaExpression::Literal(MetaValue::String("hello".to_string()))
        );
    }

    #[test]
    fn test_parse_boolean_literal() {
        let expr = parse_meta_expression("true").unwrap();
        assert_eq!(expr, MetaExpression::Literal(MetaValue::Boolean(true)));
    }

    #[test]
    fn test_parse_addition() {
        let expr = parse_meta_expression("x + 1").unwrap();
        match expr {
            MetaExpression::BinaryOp(MetaBinaryOp::Add, left, right) => {
                assert_eq!(*left, MetaExpression::Variable("x".to_string()));
                assert_eq!(*right, MetaExpression::Literal(MetaValue::Integer(1)));
            }
            _ => panic!("Expected addition expression"),
        }
    }

    // Range test removed - ranges are no longer meta expressions

    #[test]
    fn test_parse_comparison() {
        let expr = parse_meta_expression("x < 5").unwrap();
        match expr {
            MetaExpression::BinaryOp(MetaBinaryOp::LessThan, left, right) => {
                assert_eq!(*left, MetaExpression::Variable("x".to_string()));
                assert_eq!(*right, MetaExpression::Literal(MetaValue::Integer(5)));
            }
            _ => panic!("Expected comparison expression"),
        }
    }

    #[test]
    fn test_parse_parentheses() {
        let expr = parse_meta_expression("(x + 1) * 2").unwrap();
        match expr {
            MetaExpression::BinaryOp(MetaBinaryOp::Multiply, left, right) => {
                match *left {
                    MetaExpression::BinaryOp(MetaBinaryOp::Add, _, _) => {}
                    _ => panic!("Expected addition in left operand"),
                }
                assert_eq!(*right, MetaExpression::Literal(MetaValue::Integer(2)));
            }
            _ => panic!("Expected multiplication expression"),
        }
    }
}
