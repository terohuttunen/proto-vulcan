use pest::iterators::Pair;
use super::{AstBuilder, ParseError, ParseResult, Rule};
use crate::interpreter::parser::ast::*;

impl<'a> AstBuilder<'a> {
    pub fn build_attribute(&mut self, pair: Pair<Rule>) -> ParseResult<Attribute> {
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();
        let mut args = vec![];
        if let Some(args_pair) = inner.next() {
            if args_pair.as_rule() == Rule::attribute_args {
                for arg_pair in args_pair.into_inner() {
                    if arg_pair.as_rule() == Rule::attribute_arg {
                        let mut inner_arg = arg_pair.into_inner();
                        let arg_rule_pair = inner_arg.next().unwrap();
                        match arg_rule_pair.as_rule() {
                            Rule::named_attribute_arg => {
                                let mut named_inner = arg_rule_pair.into_inner();
                                let arg_name = named_inner.next().unwrap().as_str().to_string();
                                let arg_value = self.build_term(named_inner.next().unwrap())?;
                                args.push(AttributeArg::Named(arg_name, arg_value));
                            }
                            Rule::flag_attribute_arg => {
                                let flag_name = arg_rule_pair.as_str().to_string();
                                args.push(AttributeArg::Flag(flag_name));
                            }
                            _ => return Err(ParseError::UnexpectedRule(arg_rule_pair.as_rule())),
                        }
                    }
                }
            }
        }
        Ok(Attribute { name, args })
    }

    pub fn build_parameter(&mut self, pair: Pair<Rule>) -> ParseResult<Parameter> {
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();
        let type_annotation = inner
            .next()
            .map(|p| self.parse_meta_type_annotation(p))
            .transpose()?;
        Ok(Parameter {
            name,
            type_annotation,
        })
    }

    pub fn build_search_strategy(&mut self, pair: Pair<Rule>) -> ParseResult<SearchStrategy> {
        let strategy_value = pair
            .into_inner()
            .next()
            .ok_or_else(|| ParseError::MissingRule(Rule::strategy_value))?;
        match strategy_value.as_str() {
            "bfs" => Ok(SearchStrategy::Bfs),
            "dfs" => Ok(SearchStrategy::Dfs),
            _ => Err(ParseError::UnexpectedRule(Rule::strategy_value)),
        }
    }

    pub fn build_visibility(&mut self, pair: Pair<Rule>) -> ParseResult<Visibility> {
        // Check if the visibility pair has any content (empty string means private)
        if pair.as_str().is_empty() {
            return Ok(Visibility::Private);
        }

        let mut inner = pair.into_inner();

        // If we have a pub_keyword, look for optional scope
        if let Some(pub_pair) = inner.next() {
            if pub_pair.as_rule() == Rule::pub_keyword {
                // Check if there's a visibility scope
                if let Some(scope_pair) = inner.next() {
                    if scope_pair.as_rule() == Rule::visibility_scope {
                        if let Some(scope_inner) = scope_pair.into_inner().next() {
                            match scope_inner.as_rule() {
                                Rule::crate_keyword => Ok(Visibility::Crate),
                                Rule::super_keyword => Ok(Visibility::Super),
                                Rule::self_keyword => Ok(Visibility::SelfModule),
                                Rule::qualified_path => {
                                    let qualified_path = self.build_qualified_path(scope_inner)?;
                                    Ok(Visibility::Restricted(qualified_path))
                                }
                                _ => Err(ParseError::UnexpectedRule(scope_inner.as_rule())),
                            }
                        } else {
                            // Empty scope
                            Ok(Visibility::Public)
                        }
                    } else {
                        Err(ParseError::UnexpectedRule(scope_pair.as_rule()))
                    }
                } else {
                    // Just pub without scope
                    Ok(Visibility::Public)
                }
            } else {
                Err(ParseError::UnexpectedRule(pub_pair.as_rule()))
            }
        } else {
            // No pub keyword means private
            Ok(Visibility::Private)
        }
    }
}