use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use thiserror::Error;

pub mod ast;
pub mod meta_parser;
use crate::interpreter::metaprogramming::TypeAnnotation;
use ast::*;

/// Helper function to extract span information from a Pest pair
fn pair_to_span(pair: &Pair<Rule>) -> Span {
    let span = pair.as_span();
    Span::new(span.start(), span.end())
}

#[derive(Parser)]
#[grammar = "interpreter/parser/grammar.pest"]
pub struct VulcanParser;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Pest error: {0}")]
    Pest(#[from] pest::error::Error<Rule>),
    #[error("Unexpected rule: {0:?}")]
    UnexpectedRule(Rule),
    #[error("Missing rule: {0:?}")]
    MissingRule(Rule),
}

type ParseResult<T> = Result<T, ParseError>;

pub fn parse_str(input: &str) -> ParseResult<Program> {
    let pairs = VulcanParser::parse(Rule::program, input)?;
    let program = build_program(pairs.into_iter().next().unwrap())?;
    Ok(program)
}

fn build_program(pair: Pair<Rule>) -> ParseResult<Program> {
    let span = pair_to_span(&pair);
    let mut items = vec![];
    for item_pair in pair.into_inner() {
        if let Rule::EOI = item_pair.as_rule() {
            continue;
        }
        items.push(build_item(item_pair)?);
    }
    Ok(Program { items, span })
}

fn build_item(pair: Pair<Rule>) -> ParseResult<Item> {
    match pair.as_rule() {
        Rule::use_statement => Ok(Item::Use(build_use_statement(pair)?)),
        Rule::mod_definition => Ok(Item::Module(build_mod_definition(pair)?)),
        Rule::struct_definition => Ok(Item::Struct(build_struct_definition(pair)?)),
        Rule::impl_block => Ok(Item::Impl(build_impl_block(pair)?)),
        Rule::relation_definition => Ok(Item::Relation(build_relation_definition(pair)?)),
        _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
    }
}

fn build_use_statement(pair: Pair<Rule>) -> ParseResult<UseStatement> {
    let span = pair_to_span(&pair);
    let path_pair = pair.into_inner().next().unwrap();
    let path = build_use_path(path_pair)?;
    Ok(UseStatement { path, span })
}

fn build_use_path(pair: Pair<Rule>) -> ParseResult<UsePath> {
    let inner = pair.into_inner();
    let mut segments = vec![];
    let mut last_part = None;

    for part in inner {
        match part.as_rule() {
            Rule::path_segment => segments.push(part.as_str().to_string()),
            Rule::glob => {
                last_part = Some(Ok(UsePath::Glob(segments.clone())));
                break;
            }
            Rule::list_import => {
                let mut imports = vec![];
                for import_item in part.into_inner() {
                    let mut item_inner = import_item.into_inner();
                    let name = item_inner.next().unwrap().as_str().to_string();
                    let alias = item_inner.next().map(|p| p.as_str().to_string());
                    imports.push((name, alias));
                }
                last_part = Some(Ok(UsePath::List(segments.clone(), imports)));
                break;
            }
            _ => (),
        }
    }
    last_part.unwrap_or_else(|| Ok(UsePath::Simple(segments)))
}

fn build_mod_definition(pair: Pair<Rule>) -> ParseResult<ModuleDefinition> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut search_strategy = None;
    let mut items = vec![];

    for part in inner {
        match part.as_rule() {
            Rule::search_strategy => search_strategy = Some(build_search_strategy(part)?),
            Rule::use_statement
            | Rule::mod_definition
            | Rule::struct_definition
            | Rule::impl_block
            | Rule::relation_definition => items.push(build_item(part)?),
            _ => (),
        }
    }
    Ok(ModuleDefinition {
        name,
        search_strategy,
        items,
        span,
    })
}

fn build_struct_definition(pair: Pair<Rule>) -> ParseResult<StructDefinition> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let mut is_pub = false;

    // Check if first token is pub_keyword
    let first_pair = inner.next().unwrap();
    let name_pair = if first_pair.as_rule() == Rule::pub_keyword {
        is_pub = true;
        inner.next().unwrap() // Skip "struct", get type name
    } else {
        first_pair // This should be the type name (after "struct" was consumed)
    };

    let name = name_pair.as_str().to_string();
    let def_pair = inner.next().unwrap();
    let kind = match def_pair.as_rule() {
        Rule::tuple_struct_def => {
            let mut types = vec![];
            for type_pair in def_pair.into_inner() {
                types.push(type_pair.as_str().to_string());
            }
            StructKind::Tuple(types)
        }
        Rule::named_struct_def => {
            let mut fields = vec![];
            for field_pair in def_pair.into_inner() {
                fields.push(build_named_field(field_pair)?);
            }
            StructKind::Named(fields)
        }
        _ => return Err(ParseError::UnexpectedRule(def_pair.as_rule())),
    };

    Ok(StructDefinition {
        is_pub,
        name,
        kind,
        span,
    })
}

fn build_named_field(pair: Pair<Rule>) -> ParseResult<NamedField> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let mut is_pub = false;

    // Check if first token is pub_keyword
    let first_pair = inner.next().unwrap();
    let name_pair = if first_pair.as_rule() == Rule::pub_keyword {
        is_pub = true;
        inner.next().unwrap()
    } else {
        first_pair
    };

    let name = name_pair.as_str().to_string();
    let type_name = inner.next().unwrap().as_str().to_string();
    Ok(NamedField {
        is_pub,
        name,
        type_name,
        span,
    })
}

fn build_impl_block(pair: Pair<Rule>) -> ParseResult<ImplBlock> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let type_name = inner.next().unwrap().as_str().to_string();
    let mut relations = vec![];
    for rel_pair in inner {
        if rel_pair.as_rule() == Rule::relation_definition {
            relations.push(build_relation_definition(rel_pair)?);
        }
    }
    Ok(ImplBlock {
        type_name,
        relations,
        span,
    })
}

fn build_attribute(pair: Pair<Rule>) -> ParseResult<Attribute> {
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
                            let arg_value = build_term(named_inner.next().unwrap())?;
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

fn build_relation_definition(pair: Pair<Rule>) -> ParseResult<RelationDefinition> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let mut attributes = vec![];
    let mut is_pub = false;
    let mut search_strategy = None;

    // The first pairs can be attributes or `pub`, in any order.
    while let Some(p) = inner.peek() {
        match p.as_rule() {
            Rule::attribute => {
                attributes.push(build_attribute(inner.next().unwrap())?);
            }
            Rule::pub_keyword => {
                is_pub = true;
                inner.next(); // Consume pub
            }
            _ => break, // Done with optional leading elements
        }
    }

    // Now we must have `rel`, `ident`, `(params)`, optionally `search_strategy`, and `{body}`
    let name = inner.next().unwrap().as_str().to_string();

    let mut parameters = vec![];
    if let Some(p) = inner.peek() {
        if p.as_rule() == Rule::parameter {
            parameters.push(build_parameter(inner.next().unwrap())?);
            while let Some(p) = inner.peek() {
                if p.as_rule() == Rule::parameter {
                    parameters.push(build_parameter(inner.next().unwrap())?);
                } else {
                    break;
                }
            }
        }
    }

    if let Some(p) = inner.peek() {
        if p.as_rule() == Rule::search_strategy {
            search_strategy = Some(build_search_strategy(inner.next().unwrap())?);
        }
    }

    // Remove search strategy from general attributes if it was captured there
    if search_strategy.is_some() {
        attributes.retain(|a| a.name != "bfs" && a.name != "dfs");
    }

    let body = if let Some(p) = inner.peek() {
        if p.as_rule() == Rule::goal_body {
            build_goal_body(inner.next().unwrap())?
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    Ok(RelationDefinition {
        span,
        is_pub,
        attributes,
        name,
        parameters,
        search_strategy,
        body,
    })
}

fn build_parameter(pair: Pair<Rule>) -> ParseResult<Parameter> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let type_annotation = inner
        .next()
        .map(|p| parse_type_annotation(p.as_str()))
        .transpose()?;
    Ok(Parameter {
        name,
        type_annotation,
    })
}

fn parse_type_annotation(type_str: &str) -> ParseResult<TypeAnnotation> {
    match type_str {
        "int" => Ok(TypeAnnotation::Int),
        "string" => Ok(TypeAnnotation::String),
        "bool" => Ok(TypeAnnotation::Bool),
        _ => Err(ParseError::UnexpectedRule(Rule::ident)), // For now, reject unknown types
    }
}

fn build_search_strategy(pair: Pair<Rule>) -> ParseResult<SearchStrategy> {
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

fn build_goal_body(pair: Pair<Rule>) -> ParseResult<GoalBody> {
    let mut body = vec![];
    for goal_pair in pair.into_inner() {
        body.push(build_goal(goal_pair)?);
    }
    Ok(body)
}

pub fn build_goal(pair: Pair<Rule>) -> ParseResult<Goal> {
    if pair.as_rule() == Rule::goal {
        // If we get a generic goal, we need to extract the specific goal type
        let inner = pair.into_inner().next().unwrap();
        return build_goal(inner);
    }

    match pair.as_rule() {
        Rule::meta_statement => Ok(Goal::MetaStatement(
            build_meta_statement(pair.clone())?,
            pair_to_span(&pair),
        )),
        Rule::let_declaration => Ok(Goal::Let(
            build_let_declaration(pair.clone())?,
            pair_to_span(&pair),
        )),
        Rule::fresh_variables => Ok(Goal::Fresh(
            build_fresh_variables(pair.clone())?,
            pair_to_span(&pair),
        )),
        Rule::any_block => build_any_block(pair),
        Rule::all_block => build_all_block(pair),
        Rule::constraint_block => build_constraint_block(pair),
        Rule::pattern_matching => Ok(Goal::PatternMatch(
            build_pattern_matching(pair.clone())?,
            pair_to_span(&pair),
        )),
        Rule::call_expr => Ok(Goal::RelationCall(
            build_relation_call(pair.clone())?,
            pair_to_span(&pair),
        )),
        Rule::method_call => Ok(Goal::MethodCall(
            build_method_call(pair.clone())?,
            pair_to_span(&pair),
        )),
        Rule::equality_goal => {
            let span = pair_to_span(&pair);
            let mut inner = pair.into_inner();
            let lhs = build_term(inner.next().unwrap())?;
            let _equality_op = inner.next().unwrap(); // Skip the atomic equality_op
            let rhs = build_term(inner.next().unwrap())?;
            Ok(Goal::Equality(lhs, rhs, span))
        }
        Rule::disequality_goal => {
            let span = pair_to_span(&pair);
            let mut inner = pair.into_inner();
            let lhs = build_term(inner.next().unwrap())?;
            let _inequality_op = inner.next().unwrap(); // Skip the atomic inequality_op
            let rhs = build_term(inner.next().unwrap())?;
            Ok(Goal::Disequality(lhs, rhs, span))
        }
        Rule::parenthesized_goal => {
            let span = pair_to_span(&pair);
            let body = build_goal_body(pair.into_inner().next().unwrap())?;
            Ok(Goal::Parenthesized(body, span))
        }
        Rule::literal => {
            let span = pair_to_span(&pair);
            let literal = build_literal(pair)?;
            match literal {
                Literal::Boolean(b) => Ok(Goal::BooleanLiteral(b, span)),
                _ => Err(ParseError::UnexpectedRule(Rule::literal)),
            }
        }
        _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
    }
}

fn build_any_block(pair: Pair<Rule>) -> ParseResult<Goal> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let mut params = None;
    let mut body = vec![];

    while let Some(part) = inner.next() {
        match part.as_rule() {
            Rule::search_params => {
                params = Some(build_search_params(part)?);
            }
            Rule::goal_body => {
                body = build_goal_body(part)?;
            }
            _ => return Err(ParseError::UnexpectedRule(part.as_rule())),
        }
    }

    Ok(Goal::Disjunction(Disjunction { body, params }, span))
}

fn build_all_block(pair: Pair<Rule>) -> ParseResult<Goal> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let mut params = None;
    let mut body = vec![];

    while let Some(part) = inner.next() {
        match part.as_rule() {
            Rule::search_params => {
                params = Some(build_search_params(part)?);
            }
            Rule::goal_body => {
                body = build_goal_body(part)?;
            }
            _ => return Err(ParseError::UnexpectedRule(part.as_rule())),
        }
    }

    Ok(Goal::Conjunction(Conjunction { body, params }, span))
}

fn build_constraint_block(pair: Pair<Rule>) -> ParseResult<Goal> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let mut domain = "clpfd".to_string(); // Default domain
    let mut raw_content = "".to_string();
    let mut body_span = Span::dummy(); // Default span if body not found

    // The first pairs can be constraint_params
    if let Some(p) = inner.peek() {
        if p.as_rule() == Rule::constraint_params {
            domain = parse_constraint_params(inner.next().unwrap())?;
        }
    }

    // The next part must be the constraint_body
    if let Some(body_pair) = inner.next() {
        if body_pair.as_rule() == Rule::constraint_body {
            raw_content = body_pair.as_str().to_string();
            body_span = pair_to_span(&body_pair);
        }
    }

    Ok(Goal::ConstraintBlock(
        ConstraintBlock {
            domain,
            body: ConstraintBody {
                raw_content,
                span: body_span,
            },
        },
        span,
    ))
}

fn parse_constraint_params(pair: Pair<Rule>) -> ParseResult<String> {
    let mut domain = "clpfd".to_string();

    for param in pair.into_inner() {
        match param.as_rule() {
            Rule::domain_param => {
                let mut param_inner = param.into_inner();
                if let Some(domain_value) = param_inner.next() {
                    // Extract string value without quotes
                    let domain_str = domain_value.as_str();
                    domain = domain_str.trim_matches('"').to_string();
                }
            }
            Rule::custom_constraint_param => {
                // Handle custom parameters if needed in the future
            }
            _ => return Err(ParseError::UnexpectedRule(param.as_rule())),
        }
    }

    Ok(domain)
}

fn build_search_params(pair: Pair<Rule>) -> ParseResult<SearchParams> {
    let mut params = SearchParams::new();

    for param_pair in pair.into_inner() {
        match param_pair.as_rule() {
            Rule::strategy_param => {
                let strategy_value = param_pair
                    .into_inner()
                    .next()
                    .ok_or_else(|| ParseError::MissingRule(Rule::strategy_value))?;
                match strategy_value.as_rule() {
                    Rule::strategy_value => {
                        let strategy = match strategy_value.as_str() {
                            "bfs" => SearchStrategy::Bfs,
                            "dfs" => SearchStrategy::Dfs,
                            _ => return Err(ParseError::UnexpectedRule(Rule::strategy_value)),
                        };
                        params = params.with_strategy(strategy);
                    }
                    Rule::ident => return Err(ParseError::UnexpectedRule(Rule::strategy_value)),
                    _ => return Err(ParseError::UnexpectedRule(strategy_value.as_rule())),
                }
            }
            Rule::limit_param => {
                let limit_str = param_pair
                    .into_inner()
                    .next()
                    .ok_or_else(|| ParseError::MissingRule(Rule::limit_param))?;
                match limit_str.as_rule() {
                    Rule::number_literal => {
                        let limit = limit_str
                            .as_str()
                            .parse::<u64>()
                            .map_err(|_| ParseError::UnexpectedRule(Rule::limit_param))?;
                        params = params.with_limit(limit);
                    }
                    Rule::string_literal => {
                        return Err(ParseError::UnexpectedRule(Rule::limit_param))
                    }
                    _ => return Err(ParseError::UnexpectedRule(limit_str.as_rule())),
                }
            }
            Rule::depth_param => {
                let depth_str = param_pair
                    .into_inner()
                    .next()
                    .ok_or_else(|| ParseError::MissingRule(Rule::depth_param))?;
                match depth_str.as_rule() {
                    Rule::number_literal => {
                        let depth = depth_str
                            .as_str()
                            .parse::<u64>()
                            .map_err(|_| ParseError::UnexpectedRule(Rule::depth_param))?;
                        params = params.with_depth(depth);
                    }
                    Rule::string_literal => {
                        return Err(ParseError::UnexpectedRule(Rule::depth_param))
                    }
                    _ => return Err(ParseError::UnexpectedRule(depth_str.as_rule())),
                }
            }
            Rule::custom_param => {
                let mut inner = param_pair.into_inner();
                let name = inner
                    .next()
                    .ok_or_else(|| ParseError::MissingRule(Rule::custom_param))?
                    .as_str()
                    .to_string();
                let value_pair = inner
                    .next()
                    .ok_or_else(|| ParseError::MissingRule(Rule::custom_param))?;
                let value = if value_pair.as_rule() == Rule::param_value {
                    let inner_value = value_pair
                        .into_inner()
                        .next()
                        .ok_or_else(|| ParseError::MissingRule(Rule::param_value))?;
                    build_param_value(inner_value)?
                } else {
                    build_param_value(value_pair)?
                };
                params = params.with_custom_param(name, value);
            }
            _ => return Err(ParseError::UnexpectedRule(param_pair.as_rule())),
        }
    }

    Ok(params)
}

fn build_param_value(pair: Pair<Rule>) -> ParseResult<SearchParamValue> {
    match pair.as_rule() {
        Rule::number_literal => {
            Ok(SearchParamValue::Number(pair.as_str().parse().map_err(
                |_| ParseError::UnexpectedRule(Rule::number_literal),
            )?))
        }
        Rule::string_literal => {
            let s = pair.as_str();
            Ok(SearchParamValue::String(s[1..s.len() - 1].to_string()))
        }
        Rule::boolean_literal => {
            let b = pair.as_str();
            match b {
                "true" => Ok(SearchParamValue::Boolean(true)),
                "false" => Ok(SearchParamValue::Boolean(false)),
                _ => Err(ParseError::UnexpectedRule(Rule::boolean_literal)),
            }
        }
        Rule::ident => Ok(SearchParamValue::Identifier(pair.as_str().to_string())),
        _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
    }
}

fn build_let_declaration(pair: Pair<Rule>) -> ParseResult<LetDeclaration> {
    let mut inner = pair.into_inner();
    let var_name = inner.next().unwrap().as_str().to_string();
    let value = if let Some(term_pair) = inner.next() {
        Some(build_term(term_pair)?)
    } else {
        None
    };
    Ok(LetDeclaration { var_name, value })
}

fn build_fresh_variables(pair: Pair<Rule>) -> ParseResult<FreshVariables> {
    let inner = pair.into_inner();
    let mut vars = vec![];
    let mut body = None;

    for part in inner {
        match part.as_rule() {
            Rule::ident => vars.push(part.as_str().to_string()),
            Rule::goal_body => body = Some(build_goal_body(part)?),
            _ => (),
        }
    }
    Ok(FreshVariables {
        vars,
        body: body.unwrap_or_default(),
    })
}

fn build_disjunction(pair: Pair<Rule>) -> ParseResult<Goal> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let mut params = None;
    let mut body = vec![];

    while let Some(part) = inner.next() {
        match part.as_rule() {
            Rule::search_params => {
                params = Some(build_search_params(part)?);
            }
            Rule::goal_body => {
                body = build_goal_body(part)?;
            }
            _ => return Err(ParseError::UnexpectedRule(part.as_rule())),
        }
    }

    Ok(Goal::Disjunction(Disjunction { body, params }, span))
}

fn build_conjunction(pair: Pair<Rule>) -> ParseResult<Goal> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let mut params = None;
    let mut body = vec![];

    while let Some(part) = inner.next() {
        match part.as_rule() {
            Rule::search_params => {
                params = Some(build_search_params(part)?);
            }
            Rule::goal_body => {
                body = build_goal_body(part)?;
            }
            _ => return Err(ParseError::UnexpectedRule(part.as_rule())),
        }
    }

    Ok(Goal::Conjunction(Conjunction { body, params }, span))
}

fn build_pattern_matching(pair: Pair<Rule>) -> ParseResult<PatternMatching> {
    let mut inner = pair.into_inner();
    let term = build_term(inner.next().unwrap())?;
    let mut arms = vec![];
    for arm_pair in inner {
        arms.push(build_pattern_arm(arm_pair)?);
    }
    Ok(PatternMatching { term, arms })
}

fn build_pattern_arm(pair: Pair<Rule>) -> ParseResult<PatternArm> {
    let mut inner = pair.into_inner();
    let pattern = build_pattern(inner.next().unwrap())?;
    // Skip the pattern_arrow token
    let _arrow = inner.next().unwrap(); // This should be the pattern_arrow rule
    let body_part = inner.next().unwrap();
    let body = match body_part.as_rule() {
        Rule::goal => vec![build_goal(body_part)?],
        Rule::goal_body => build_goal_body(body_part)?,
        _ => return Err(ParseError::UnexpectedRule(body_part.as_rule())),
    };
    Ok(PatternArm { pattern, body })
}

fn build_relation_call(pair: Pair<Rule>) -> ParseResult<RelationCall> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut args = vec![];
    for term_pair in inner {
        args.push(build_term(term_pair)?);
    }
    Ok(RelationCall { name, args })
}

fn build_method_call(pair: Pair<Rule>) -> ParseResult<MethodCall> {
    let mut inner = pair.into_inner();
    let receiver = Box::new(build_term(inner.next().unwrap())?);
    let method = inner.next().unwrap().as_str().to_string();
    let mut args = vec![];
    for term_pair in inner {
        args.push(build_term(term_pair)?);
    }
    Ok(MethodCall {
        receiver,
        method,
        args,
    })
}

fn build_term(pair: Pair<Rule>) -> ParseResult<Term> {
    if pair.as_rule() == Rule::term {
        // If we get a generic term, we need to extract the specific term type
        let inner = pair
            .into_inner()
            .next()
            .ok_or_else(|| ParseError::MissingRule(Rule::term))?;
        return build_term(inner);
    }

    let span = pair_to_span(&pair);
    match pair.as_rule() {
        Rule::interpolation_expression => {
            let content = pair.into_inner().next().unwrap().as_str();
            let expr = meta_parser::parse_meta_expression(content, &span)
                .map_err(|_| ParseError::UnexpectedRule(Rule::interpolation_expression))?;
            Ok(Term::Interpolation(expr, span))
        }
        Rule::literal => {
            let lit_pair = pair
                .into_inner()
                .next()
                .ok_or_else(|| ParseError::MissingRule(Rule::literal))?;
            Ok(Term::Literal(build_literal(lit_pair)?, span))
        }
        Rule::variable => Ok(Term::Variable(pair.as_str().to_string(), span)),
        Rule::wildcard => Ok(Term::Wildcard(span)),
        Rule::list_construction => Ok(Term::List(build_list_construction(pair.clone())?, span)),
        Rule::named_struct_construction => Ok(Term::NamedStruct(
            build_named_struct_construction(pair.clone())?,
            span,
        )),
        Rule::call_expr => Ok(Term::Compound(
            build_compound_construction(pair.clone())?,
            span,
        )),
        Rule::path_term => Ok(Term::Compound(
            CompoundConstruction {
                name: pair.as_str().to_string(),
                args: vec![],
            },
            span,
        )),
        Rule::parenthesized_term => Ok(Term::Parenthesized(
            Box::new(build_term(pair.into_inner().next().unwrap())?),
            span,
        )),
        _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
    }
}

fn build_named_struct_construction(pair: Pair<Rule>) -> ParseResult<NamedStructConstruction> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut fields = vec![];
    for field_pair in inner {
        fields.push(build_field_initializer(field_pair)?);
    }
    Ok(NamedStructConstruction { name, fields })
}

fn build_field_initializer(pair: Pair<Rule>) -> ParseResult<FieldInitializer> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let value = build_term(inner.next().unwrap())?;
    Ok(FieldInitializer { name, value })
}

fn build_compound_construction(pair: Pair<Rule>) -> ParseResult<CompoundConstruction> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut args = vec![];
    for term_pair in inner {
        args.push(build_term(term_pair)?);
    }
    Ok(CompoundConstruction { name, args })
}

fn build_literal(pair: Pair<Rule>) -> ParseResult<Literal> {
    if pair.as_rule() == Rule::literal {
        // If we get a generic literal, we need to extract the specific literal type
        let inner = pair.into_inner().next().unwrap();
        return build_literal(inner);
    }

    match pair.as_rule() {
        Rule::boolean_literal => Ok(Literal::Boolean(pair.as_str().parse().unwrap())),
        Rule::number_literal => Ok(Literal::Number(pair.as_str().to_string())),
        Rule::string_literal => {
            let s = pair.as_str();
            Ok(Literal::String(s[1..s.len() - 1].to_string()))
        }
        Rule::char_literal => {
            let s = pair.as_str();
            Ok(Literal::Char(s[1..s.len() - 1].chars().next().unwrap()))
        }
        _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
    }
}

fn build_pattern(pair: Pair<Rule>) -> ParseResult<Pattern> {
    if pair.as_rule() == Rule::pattern {
        // If we get a generic pattern, we need to extract the specific pattern type
        let inner = pair.into_inner().next().unwrap();
        return build_pattern(inner);
    }

    match pair.as_rule() {
        Rule::literal => Ok(Pattern::Literal(build_literal(pair)?)),
        Rule::variable => Ok(Pattern::Variable(pair.as_str().to_string())),
        Rule::wildcard => Ok(Pattern::Wildcard),
        Rule::list_pattern => Ok(Pattern::List(build_list_pattern(pair)?)),
        Rule::named_struct_pattern => Ok(Pattern::NamedStruct(build_named_struct_pattern(pair)?)),
        Rule::compound_pattern_with_parens => {
            Ok(Pattern::Compound(build_compound_pattern_with_parens(pair)?))
        }
        Rule::compound_pattern_no_parens => {
            Ok(Pattern::Compound(build_compound_pattern_no_parens(pair)?))
        }
        _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
    }
}

fn build_list_pattern(pair: Pair<Rule>) -> ParseResult<ListPattern> {
    let mut elements = vec![];
    let mut tail = None;

    if let Some(pattern_list_pair) = pair.into_inner().next() {
        let mut inner = pattern_list_pair.into_inner();
        while let Some(p) = inner.next() {
            match p.as_rule() {
                Rule::pattern => {
                    elements.push(build_pattern(p)?);
                }
                Rule::list_tail => {
                    let tail_pattern_pair = p.into_inner().next().unwrap();
                    tail = Some(Box::new(build_pattern(tail_pattern_pair)?));
                    break; // No more elements after tail
                }
                _ => return Err(ParseError::UnexpectedRule(p.as_rule())),
            }
        }
    }

    Ok(ListPattern { elements, tail })
}

fn build_named_struct_pattern(pair: Pair<Rule>) -> ParseResult<NamedStructPattern> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut fields = vec![];
    for field_pair in inner {
        fields.push(build_field_pattern(field_pair)?);
    }
    Ok(NamedStructPattern { name, fields })
}

fn build_field_pattern(pair: Pair<Rule>) -> ParseResult<FieldPattern> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let pattern = build_pattern(inner.next().unwrap())?;
    Ok(FieldPattern { name, pattern })
}

fn build_compound_pattern_with_parens(pair: Pair<Rule>) -> ParseResult<CompoundPattern> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut args = vec![];
    for pattern_pair in inner {
        args.push(build_pattern(pattern_pair)?);
    }
    Ok(CompoundPattern { name, args })
}

fn build_compound_pattern_no_parens(pair: Pair<Rule>) -> ParseResult<CompoundPattern> {
    Ok(CompoundPattern {
        name: pair.as_str().to_string(),
        args: vec![],
    })
}

fn build_list_construction(pair: Pair<Rule>) -> ParseResult<ListConstruction> {
    let mut elements = vec![];
    let mut tail = None;

    if let Some(term_list_pair) = pair.into_inner().next() {
        let mut inner = term_list_pair.into_inner();
        while let Some(part) = inner.next() {
            match part.as_rule() {
                Rule::term => {
                    elements.push(build_term(part)?);
                }
                Rule::term_tail => {
                    if let Some(tail_term) = part.into_inner().next() {
                        tail = Some(Box::new(build_term(tail_term)?));
                    }
                }
                _ => return Err(ParseError::UnexpectedRule(part.as_rule())),
            }
        }
    }

    Ok(ListConstruction { elements, tail })
}

// =============================================================================
// Meta Programming Parser Functions
// =============================================================================

fn build_meta_statement(
    pair: Pair<Rule>,
) -> ParseResult<crate::interpreter::metaprogramming::MetaStatement> {
    use crate::interpreter::metaprogramming::MetaStatement;

    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::meta_let_statement => Ok(MetaStatement::Let(build_meta_let_statement(inner)?)),
        Rule::meta_if_statement => {
            let (condition, then_body, else_body) = build_meta_if_statement(inner)?;
            Ok(MetaStatement::If {
                condition,
                then_body,
                else_body,
            })
        }
        Rule::meta_for_statement => {
            let (variable, variable_type, range, body) = build_meta_for_statement(inner)?;
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

fn build_meta_let_statement(
    pair: Pair<Rule>,
) -> ParseResult<crate::interpreter::metaprogramming::LetStatement> {
    use crate::interpreter::metaprogramming::LetStatement;

    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let variable = inner.next().unwrap().as_str().to_string();
    let variable_type = parse_type_annotation(inner.next().unwrap().as_str())?;
    let content = inner.next().unwrap().as_str();
    let expression = meta_parser::parse_meta_expression(content, &span)
        .map_err(|_| ParseError::UnexpectedRule(Rule::meta_let_statement))?;

    Ok(LetStatement {
        variable,
        variable_type,
        expression,
    })
}

fn build_meta_if_statement(
    pair: Pair<Rule>,
) -> ParseResult<(
    crate::interpreter::metaprogramming::MetaExpression,
    GoalBody,
    Option<GoalBody>,
)> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let content = inner.next().unwrap().as_str();
    let condition = meta_parser::parse_meta_expression(content, &span)
        .map_err(|_| ParseError::UnexpectedRule(Rule::meta_if_statement))?;
    let then_body = build_goal_body(inner.next().unwrap())?;
    let else_body = inner.next().map(|p| build_goal_body(p)).transpose()?;

    Ok((condition, then_body, else_body))
}

fn build_meta_for_statement(
    pair: Pair<Rule>,
) -> ParseResult<(
    String,
    TypeAnnotation,
    crate::interpreter::metaprogramming::MetaForRange,
    GoalBody,
)> {
    let mut inner = pair.into_inner();
    let variable = inner.next().unwrap().as_str().to_string();
    let variable_type = parse_type_annotation(inner.next().unwrap().as_str())?;
    let range_pair = inner.next().unwrap(); // meta_for_range
    let range = build_meta_for_range(range_pair)?;
    let body = build_goal_body(inner.next().unwrap())?;

    Ok((variable, variable_type, range, body))
}

fn build_meta_for_range(
    pair: Pair<Rule>,
) -> ParseResult<crate::interpreter::metaprogramming::MetaForRange> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let start_content = inner.next().unwrap().as_str();
    let end_content = inner.next().unwrap().as_str();

    let start = meta_parser::parse_meta_expression(start_content, &span)
        .map_err(|_| ParseError::UnexpectedRule(Rule::meta_for_range))?;
    let end = meta_parser::parse_meta_expression(end_content, &span)
        .map_err(|_| ParseError::UnexpectedRule(Rule::meta_for_range))?;

    Ok(crate::interpreter::metaprogramming::MetaForRange { start, end })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_program() {
        let input = "";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_simple_relation() {
        let input = "rel my_rel() {}";
        let ast = parse_str(input).unwrap();
        let expected_ast = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "my_rel".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected_ast);
    }

    #[test]
    fn test_parse_pub_relation() {
        let input = "pub rel my_rel(a: int, b: string) @dfs { a == b }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: true,
                attributes: vec![],
                name: "my_rel".to_string(),
                parameters: vec![
                    Parameter {
                        name: "a".to_string(),
                        type_annotation: Some(TypeAnnotation::Int),
                    },
                    Parameter {
                        name: "b".to_string(),
                        type_annotation: Some(TypeAnnotation::String),
                    },
                ],
                search_strategy: Some(SearchStrategy::Dfs),
                body: vec![Goal::Equality(
                    Term::Variable("a".to_string(), Span::dummy()),
                    Term::Variable("b".to_string(), Span::dummy()),
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_use_statement_simple() {
        let input = "use a::b::c;";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Use(UseStatement {
                path: UsePath::Simple(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
                span: Default::default(),
            })],
            span: Default::default(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_use_statement_glob() {
        let input = "use a::b::*;";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Use(UseStatement {
                path: UsePath::Glob(vec!["a".to_string(), "b".to_string()]),
                span: Default::default(),
            })],
            span: Default::default(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_use_statement_list() {
        let input = "use a::{b, c as d};";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Use(UseStatement {
                path: UsePath::List(
                    vec!["a".to_string()],
                    vec![
                        ("b".to_string(), None),
                        ("c".to_string(), Some("d".to_string())),
                    ],
                ),
                span: Default::default(),
            })],
            span: Default::default(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_tuple_struct() {
        let input = "struct MyTuple(A, B);";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Struct(StructDefinition {
                is_pub: false,
                name: "MyTuple".to_string(),
                kind: StructKind::Tuple(vec!["A".to_string(), "B".to_string()]),
                span: Span::dummy(),
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_named_struct() {
        let input = "pub struct MyStruct { pub field: T, other: U }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Struct(StructDefinition {
                is_pub: true,
                name: "MyStruct".to_string(),
                kind: StructKind::Named(vec![
                    NamedField {
                        is_pub: true,
                        name: "field".to_string(),
                        type_name: "T".to_string(),
                        span: Span::dummy(),
                    },
                    NamedField {
                        is_pub: false,
                        name: "other".to_string(),
                        type_name: "U".to_string(),
                        span: Span::dummy(),
                    },
                ]),
                span: Span::dummy(),
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_impl_block() {
        let input = r#"
        impl Point {
            rel new(x, y, p) {
                p == Point { x: x, y: y }
            }
        }
        "#;
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Impl(ImplBlock {
                type_name: "Point".to_string(),
                span: Span::dummy(),
                relations: vec![RelationDefinition {
                    span: Span::dummy(),
                    is_pub: false,
                    attributes: vec![],
                    name: "new".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "x".to_string(),
                            type_annotation: None,
                        },
                        Parameter {
                            name: "y".to_string(),
                            type_annotation: None,
                        },
                        Parameter {
                            name: "p".to_string(),
                            type_annotation: None,
                        },
                    ],
                    search_strategy: None,
                    body: vec![Goal::Equality(
                        Term::Variable("p".to_string(), Span::dummy()),
                        Term::NamedStruct(
                            NamedStructConstruction {
                                name: "Point".to_string(),
                                fields: vec![
                                    FieldInitializer {
                                        name: "x".to_string(),
                                        value: Term::Variable("x".to_string(), Span::dummy()),
                                    },
                                    FieldInitializer {
                                        name: "y".to_string(),
                                        value: Term::Variable("y".to_string(), Span::dummy()),
                                    },
                                ],
                            },
                            Span::dummy(),
                        ),
                        Span::dummy(),
                    )],
                }],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_literals() {
        let input = r#"rel test() { 
            a == true,
            b == false,
            c == 42,
            d == -17,
            e == "hello",
            f == 'x'
        }"#;
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![
                    Goal::Equality(
                        Term::Variable("a".to_string(), Span::dummy()),
                        Term::Literal(Literal::Boolean(true), Span::dummy()),
                        Span::dummy(),
                    ),
                    Goal::Equality(
                        Term::Variable("b".to_string(), Span::dummy()),
                        Term::Literal(Literal::Boolean(false), Span::dummy()),
                        Span::dummy(),
                    ),
                    Goal::Equality(
                        Term::Variable("c".to_string(), Span::dummy()),
                        Term::Literal(Literal::Number("42".to_string()), Span::dummy()),
                        Span::dummy(),
                    ),
                    Goal::Equality(
                        Term::Variable("d".to_string(), Span::dummy()),
                        Term::Literal(Literal::Number("-17".to_string()), Span::dummy()),
                        Span::dummy(),
                    ),
                    Goal::Equality(
                        Term::Variable("e".to_string(), Span::dummy()),
                        Term::Literal(Literal::String("hello".to_string()), Span::dummy()),
                        Span::dummy(),
                    ),
                    Goal::Equality(
                        Term::Variable("f".to_string(), Span::dummy()),
                        Term::Literal(Literal::Char('x'), Span::dummy()),
                        Span::dummy(),
                    ),
                ],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_list_construction() {
        let input = "rel test() { a == [1, 2, 3] }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Equality(
                    Term::Variable("a".to_string(), Span::dummy()),
                    Term::List(
                        ListConstruction {
                            elements: vec![
                                Term::Literal(Literal::Number("1".to_string()), Span::dummy()),
                                Term::Literal(Literal::Number("2".to_string()), Span::dummy()),
                                Term::Literal(Literal::Number("3".to_string()), Span::dummy()),
                            ],
                            tail: None,
                        },
                        Span::dummy(),
                    ),
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_compound_construction() {
        let input = "rel test() { a == Some(42) }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Equality(
                    Term::Variable("a".to_string(), Span::dummy()),
                    Term::Compound(
                        CompoundConstruction {
                            name: "Some".to_string(),
                            args: vec![Term::Literal(
                                Literal::Number("42".to_string()),
                                Span::dummy(),
                            )],
                        },
                        Span::dummy(),
                    ),
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_disjunction() {
        let input = r#"rel test() { 
            any {
                a == 1,
                a == 2
            }
        }"#;
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Disjunction(
                    Disjunction {
                        body: vec![
                            Goal::Equality(
                                Term::Variable("a".to_string(), Span::dummy()),
                                Term::Literal(Literal::Number("1".to_string()), Span::dummy()),
                                Span::dummy(),
                            ),
                            Goal::Equality(
                                Term::Variable("a".to_string(), Span::dummy()),
                                Term::Literal(Literal::Number("2".to_string()), Span::dummy()),
                                Span::dummy(),
                            ),
                        ],
                        params: None,
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_conjunction() {
        let input = "rel test() { all { a == 1, b == 2 } }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Conjunction(
                    Conjunction {
                        body: vec![
                            Goal::Equality(
                                Term::Variable("a".to_string(), Span::dummy()),
                                Term::Literal(Literal::Number("1".to_string()), Span::dummy()),
                                Span::dummy(),
                            ),
                            Goal::Equality(
                                Term::Variable("b".to_string(), Span::dummy()),
                                Term::Literal(Literal::Number("2".to_string()), Span::dummy()),
                                Span::dummy(),
                            ),
                        ],
                        params: None,
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_fresh_variables() {
        let input = "rel test() { |x, y| { x == y } }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Fresh(
                    FreshVariables {
                        vars: vec!["x".to_string(), "y".to_string()],
                        body: vec![Goal::Equality(
                            Term::Variable("x".to_string(), Span::dummy()),
                            Term::Variable("y".to_string(), Span::dummy()),
                            Span::dummy(),
                        )],
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_pattern_matching() {
        let input = r#"rel test(l) {
            match l {
                [] => { succeed() },
                [a] => { a == 1 },
                _ => { fail() }
            }
        }"#;
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![Parameter {
                    name: "l".to_string(),
                    type_annotation: None,
                }],
                search_strategy: None,
                body: vec![Goal::PatternMatch(
                    PatternMatching {
                        term: Term::Variable("l".to_string(), Span::dummy()),
                        arms: vec![
                            PatternArm {
                                pattern: Pattern::List(ListPattern {
                                    elements: vec![],
                                    tail: None,
                                }),
                                body: vec![Goal::RelationCall(
                                    RelationCall {
                                        name: "succeed".to_string(),
                                        args: vec![],
                                    },
                                    Span::dummy(),
                                )],
                            },
                            PatternArm {
                                pattern: Pattern::List(ListPattern {
                                    elements: vec![Pattern::Variable("a".to_string())],
                                    tail: None,
                                }),
                                body: vec![Goal::Equality(
                                    Term::Variable("a".to_string(), Span::dummy()),
                                    Term::Literal(Literal::Number("1".to_string()), Span::dummy()),
                                    Span::dummy(),
                                )],
                            },
                            PatternArm {
                                pattern: Pattern::Wildcard,
                                body: vec![Goal::RelationCall(
                                    RelationCall {
                                        name: "fail".to_string(),
                                        args: vec![],
                                    },
                                    Span::dummy(),
                                )],
                            },
                        ],
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_pattern_matching_single_goal() {
        let input = r#"rel test(l) {
            match l {
                [] => succeed(),
                [a] => a == 1,
                _ => fail()
            }
        }"#;
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![Parameter {
                    name: "l".to_string(),
                    type_annotation: None,
                }],
                search_strategy: None,
                body: vec![Goal::PatternMatch(
                    PatternMatching {
                        term: Term::Variable("l".to_string(), Span::dummy()),
                        arms: vec![
                            PatternArm {
                                pattern: Pattern::List(ListPattern {
                                    elements: vec![],
                                    tail: None,
                                }),
                                body: vec![Goal::RelationCall(
                                    RelationCall {
                                        name: "succeed".to_string(),
                                        args: vec![],
                                    },
                                    Span::dummy(),
                                )],
                            },
                            PatternArm {
                                pattern: Pattern::List(ListPattern {
                                    elements: vec![Pattern::Variable("a".to_string())],
                                    tail: None,
                                }),
                                body: vec![Goal::Equality(
                                    Term::Variable("a".to_string(), Span::dummy()),
                                    Term::Literal(Literal::Number("1".to_string()), Span::dummy()),
                                    Span::dummy(),
                                )],
                            },
                            PatternArm {
                                pattern: Pattern::Wildcard,
                                body: vec![Goal::RelationCall(
                                    RelationCall {
                                        name: "fail".to_string(),
                                        args: vec![],
                                    },
                                    Span::dummy(),
                                )],
                            },
                        ],
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_list_pattern_with_tail() {
        let input = r#"rel test(l) {
            match l {
                [a, b | t] => { a == b }
            }
        }"#;
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![Parameter {
                    name: "l".to_string(),
                    type_annotation: None,
                }],
                search_strategy: None,
                body: vec![Goal::PatternMatch(
                    PatternMatching {
                        term: Term::Variable("l".to_string(), Span::dummy()),
                        arms: vec![PatternArm {
                            pattern: Pattern::List(ListPattern {
                                elements: vec![
                                    Pattern::Variable("a".to_string()),
                                    Pattern::Variable("b".to_string()),
                                ],
                                tail: Some(Box::new(Pattern::Variable("t".to_string()))),
                            }),
                            body: vec![Goal::Equality(
                                Term::Variable("a".to_string(), Span::dummy()),
                                Term::Variable("b".to_string(), Span::dummy()),
                                Span::dummy(),
                            )],
                        }],
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_let_declaration() {
        let input = r#"rel test() {
            let x = 42;
            let y;
            x == y
        }"#;
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![
                    Goal::Let(
                        LetDeclaration {
                            var_name: "x".to_string(),
                            value: Some(Term::Literal(
                                Literal::Number("42".to_string()),
                                Span::dummy(),
                            )),
                        },
                        Span::dummy(),
                    ),
                    Goal::Let(
                        LetDeclaration {
                            var_name: "y".to_string(),
                            value: None,
                        },
                        Span::dummy(),
                    ),
                    Goal::Equality(
                        Term::Variable("x".to_string(), Span::dummy()),
                        Term::Variable("y".to_string(), Span::dummy()),
                        Span::dummy(),
                    ),
                ],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_method_call() {
        let input = "rel test() { x.method(a, b) }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::MethodCall(
                    MethodCall {
                        receiver: Box::new(Term::Variable("x".to_string(), Span::dummy())),
                        method: "method".to_string(),
                        args: vec![
                            Term::Variable("a".to_string(), Span::dummy()),
                            Term::Variable("b".to_string(), Span::dummy()),
                        ],
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_relation_call() {
        let input = "rel test() { my_relation(a, b, c) }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::RelationCall(
                    RelationCall {
                        name: "my_relation".to_string(),
                        args: vec![
                            Term::Variable("a".to_string(), Span::dummy()),
                            Term::Variable("b".to_string(), Span::dummy()),
                            Term::Variable("c".to_string(), Span::dummy()),
                        ],
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_relation_call_no_args() {
        let input = "rel test() { succeed() }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::RelationCall(
                    RelationCall {
                        name: "succeed".to_string(),
                        args: vec![],
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_disequality() {
        let input = "rel test() { a != b }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Disequality(
                    Term::Variable("a".to_string(), Span::dummy()),
                    Term::Variable("b".to_string(), Span::dummy()),
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_module() {
        let input = r#"
        mod my_module @bfs {
            use std::collections::HashMap;
            
            struct Point { x: i32, y: i32 }
            
            rel test() { succeed() }
        }
        "#;
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Module(ModuleDefinition {
                name: "my_module".to_string(),
                search_strategy: Some(SearchStrategy::Bfs),
                items: vec![
                    Item::Use(UseStatement {
                        path: UsePath::Simple(vec![
                            "std".to_string(),
                            "collections".to_string(),
                            "HashMap".to_string(),
                        ]),
                        span: Span::dummy(),
                    }),
                    Item::Struct(StructDefinition {
                        is_pub: false,
                        name: "Point".to_string(),
                        kind: StructKind::Named(vec![
                            NamedField {
                                is_pub: false,
                                name: "x".to_string(),
                                type_name: "i32".to_string(),
                                span: Span::dummy(),
                            },
                            NamedField {
                                is_pub: false,
                                name: "y".to_string(),
                                type_name: "i32".to_string(),
                                span: Span::dummy(),
                            },
                        ]),
                        span: Span::dummy(),
                    }),
                    Item::Relation(RelationDefinition {
                        span: Span::dummy(),
                        is_pub: false,
                        attributes: vec![],
                        name: "test".to_string(),
                        parameters: vec![],
                        search_strategy: None,
                        body: vec![Goal::RelationCall(
                            RelationCall {
                                name: "succeed".to_string(),
                                args: vec![],
                            },
                            Span::dummy(),
                        )],
                    }),
                ],
                span: Span::dummy(),
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_parenthesized_goal() {
        let input = "rel test() { (a == b, c == d) }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Parenthesized(
                    vec![
                        Goal::Equality(
                            Term::Variable("a".to_string(), Span::dummy()),
                            Term::Variable("b".to_string(), Span::dummy()),
                            Span::dummy(),
                        ),
                        Goal::Equality(
                            Term::Variable("c".to_string(), Span::dummy()),
                            Term::Variable("d".to_string(), Span::dummy()),
                            Span::dummy(),
                        ),
                    ],
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_parenthesized_term() {
        let input = "rel test() { a == (b) }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Equality(
                    Term::Variable("a".to_string(), Span::dummy()),
                    Term::Parenthesized(
                        Box::new(Term::Variable("b".to_string(), Span::dummy())),
                        Span::dummy(),
                    ),
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_named_struct_pattern() {
        let input = r#"rel test(p) {
            match p {
                Point { x: a, y: b } => { a == b }
            }
        }"#;
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![Parameter {
                    name: "p".to_string(),
                    type_annotation: None,
                }],
                search_strategy: None,
                body: vec![Goal::PatternMatch(
                    PatternMatching {
                        term: Term::Variable("p".to_string(), Span::dummy()),
                        arms: vec![PatternArm {
                            pattern: Pattern::NamedStruct(NamedStructPattern {
                                name: "Point".to_string(),
                                fields: vec![
                                    FieldPattern {
                                        name: "x".to_string(),
                                        pattern: Pattern::Variable("a".to_string()),
                                    },
                                    FieldPattern {
                                        name: "y".to_string(),
                                        pattern: Pattern::Variable("b".to_string()),
                                    },
                                ],
                            }),
                            body: vec![Goal::Equality(
                                Term::Variable("a".to_string(), Span::dummy()),
                                Term::Variable("b".to_string(), Span::dummy()),
                                Span::dummy(),
                            )],
                        }],
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_compound_pattern() {
        let input = "rel a() { match x { Cons(h, t) => { h == 1 } } }";
        let ast = parse_str(input).unwrap();

        let item = ast.items.get(0).unwrap();
        let relation = match item {
            Item::Relation(r) => r,
            _ => panic!("Expected relation"),
        };
        let goal = relation.body.get(0).unwrap();
        let pattern_matching = match goal {
            Goal::PatternMatch(pm, _) => pm,
            _ => panic!("Expected pattern matching goal"),
        };

        let expected_pattern = Pattern::Compound(CompoundPattern {
            name: "Cons".to_string(),
            args: vec![
                Pattern::Variable("h".to_string()),
                Pattern::Variable("t".to_string()),
            ],
        });

        assert_eq!(pattern_matching.arms[0].pattern, expected_pattern);
    }

    #[test]
    fn test_parse_compound_pattern_qualified() {
        let input = "rel a() { match x { Option::Some(a) => { a == 1 } } }";
        let ast = parse_str(input).unwrap();

        let item = ast.items.get(0).unwrap();
        let relation = match item {
            Item::Relation(r) => r,
            _ => panic!("Expected relation"),
        };
        let goal = relation.body.get(0).unwrap();
        let pattern_matching = match goal {
            Goal::PatternMatch(pm, _) => pm,
            _ => panic!("Expected pattern matching goal"),
        };

        let expected_pattern = Pattern::Compound(CompoundPattern {
            name: "Option::Some".to_string(),
            args: vec![Pattern::Variable("a".to_string())],
        });

        assert_eq!(pattern_matching.arms[0].pattern, expected_pattern);
    }

    #[test]
    fn test_parse_compound_pattern_no_parens() {
        let input = "rel a() { match x { Option::None => {} } }";
        let ast = parse_str(input).unwrap();

        let item = ast.items.get(0).unwrap();
        let relation = match item {
            Item::Relation(r) => r,
            _ => panic!("Expected relation"),
        };
        let goal = relation.body.get(0).unwrap();
        let pattern_matching = match goal {
            Goal::PatternMatch(pm, _) => pm,
            _ => panic!("Expected pattern matching goal"),
        };

        let expected_pattern = Pattern::Compound(CompoundPattern {
            name: "Option::None".to_string(),
            args: vec![],
        });

        assert_eq!(pattern_matching.arms[0].pattern, expected_pattern);
    }

    #[test]
    fn test_parse_compound_construction_qualified() {
        let input = "rel a() { x == std::option::Option::Some(1) }";
        let ast = parse_str(input).unwrap();
        let item = ast.items.get(0).unwrap();
        let relation = match item {
            Item::Relation(r) => r,
            _ => panic!("Expected relation"),
        };
        let goal = relation.body.get(0).unwrap();
        let (_lhs, rhs) = match goal {
            Goal::Equality(_lhs, rhs, _) => (_lhs, rhs),
            _ => panic!("Expected equality goal"),
        };

        let expected_term = Term::Compound(
            CompoundConstruction {
                name: "std::option::Option::Some".to_string(),
                args: vec![Term::Literal(
                    Literal::Number("1".to_string()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        assert_eq!(*rhs, expected_term);
    }

    #[test]
    fn test_parse_compound_construction_no_parens() {
        let input = "rel a() { x == std::option::Option::None }";
        let ast = parse_str(input).unwrap();
        let item = ast.items.get(0).unwrap();
        let relation = match item {
            Item::Relation(r) => r,
            _ => panic!("Expected relation"),
        };
        let goal = relation.body.get(0).unwrap();
        let (_lhs, rhs) = match goal {
            Goal::Equality(_lhs, rhs, _) => (_lhs, rhs),
            _ => panic!("Expected equality goal"),
        };

        let expected_term = Term::Compound(
            CompoundConstruction {
                name: "std::option::Option::None".to_string(),
                args: vec![],
            },
            Span::dummy(),
        );

        assert_eq!(*rhs, expected_term);
    }

    #[test]
    fn test_parse_error_handling() {
        let input = "rel my_rel(a, b) { a = b }"; // Missing type, invalid goal
        let result = parse_str(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_complex_example() {
        let input = r#"rel test() {
            all {
                a == 1,
                any(strategy = dfs, depth = 3) {
                    b == 2,
                    c == 3
                }
            }
        }"#;
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Conjunction(
                    Conjunction {
                        body: vec![
                            Goal::Equality(
                                Term::Variable("a".to_string(), Span::dummy()),
                                Term::Literal(Literal::Number("1".to_string()), Span::dummy()),
                                Span::dummy(),
                            ),
                            Goal::Disjunction(
                                Disjunction {
                                    body: vec![
                                        Goal::Equality(
                                            Term::Variable("b".to_string(), Span::dummy()),
                                            Term::Literal(
                                                Literal::Number("2".to_string()),
                                                Span::dummy(),
                                            ),
                                            Span::dummy(),
                                        ),
                                        Goal::Equality(
                                            Term::Variable("c".to_string(), Span::dummy()),
                                            Term::Literal(
                                                Literal::Number("3".to_string()),
                                                Span::dummy(),
                                            ),
                                            Span::dummy(),
                                        ),
                                    ],
                                    params: Some(SearchParams {
                                        strategy: Some(SearchStrategy::Dfs),
                                        depth: Some(3),
                                        limit: None,
                                        custom_params: vec![],
                                    }),
                                },
                                Span::dummy(),
                            ),
                        ],
                        params: None,
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_relation_with_search_strategy() {
        let input = "rel my_rel() @bfs {}";
        let ast = parse_str(input).unwrap();
        let rel_def = match &ast.items[0] {
            Item::Relation(r) => r,
            _ => panic!("Expected relation definition"),
        };
        assert_eq!(rel_def.search_strategy, Some(SearchStrategy::Bfs));
        assert!(rel_def.attributes.is_empty());
    }

    #[test]
    fn test_parse_relation_with_attribute() {
        let input = "@test rel my_rel() {}";
        let ast = parse_str(input).unwrap();
        let rel_def = match &ast.items[0] {
            Item::Relation(r) => r,
            _ => panic!("Expected relation definition"),
        };
        assert_eq!(rel_def.attributes.len(), 1);
        assert_eq!(rel_def.attributes[0].name, "test");
        assert_eq!(rel_def.search_strategy, None);
    }

    #[test]
    fn test_parse_simple_conjunction() {
        let input = r#"rel test() {
            all {
                a == 1,
                b == 2
            }
        }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => {
                assert_eq!(rel.body.len(), 1);
                match &rel.body[0] {
                    Goal::Conjunction(conj, _) => {
                        let expected = Conjunction::new(vec![
                            Goal::Equality(
                                Term::Variable("a".to_string(), Span::dummy()),
                                Term::Literal(Literal::Number("1".to_string()), Span::dummy()),
                                Span::dummy(),
                            ),
                            Goal::Equality(
                                Term::Variable("b".to_string(), Span::dummy()),
                                Term::Literal(Literal::Number("2".to_string()), Span::dummy()),
                                Span::dummy(),
                            ),
                        ]);
                        assert_eq!(conj, &expected);
                    }
                    _ => panic!("Expected conjunction"),
                }
            }
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_simple_disjunction() {
        let input = r#"rel test() {
            any {
                a == 1,
                b == 2
            }
        }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => {
                assert_eq!(rel.body.len(), 1);
                match &rel.body[0] {
                    Goal::Disjunction(disj, _) => {
                        let expected = Disjunction::new(vec![
                            Goal::Equality(
                                Term::Variable("a".to_string(), Span::dummy()),
                                Term::Literal(Literal::Number("1".to_string()), Span::dummy()),
                                Span::dummy(),
                            ),
                            Goal::Equality(
                                Term::Variable("b".to_string(), Span::dummy()),
                                Term::Literal(Literal::Number("2".to_string()), Span::dummy()),
                                Span::dummy(),
                            ),
                        ]);
                        assert_eq!(disj, &expected);
                    }
                    _ => panic!("Expected disjunction"),
                }
            }
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_conjunction_with_strategy() {
        let input = r#"rel test() {
            all(strategy = dfs) {
                a == 1,
                b == 2
            }
        }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => match &rel.body[0] {
                Goal::Conjunction(conj, _) => {
                    let mut params = SearchParams::new();
                    params.strategy = Some(SearchStrategy::Dfs);
                    let expected = Conjunction::with_params(
                        vec![
                            Goal::Equality(
                                Term::Variable("a".to_string(), Span::dummy()),
                                Term::Literal(Literal::Number("1".to_string()), Span::dummy()),
                                Span::dummy(),
                            ),
                            Goal::Equality(
                                Term::Variable("b".to_string(), Span::dummy()),
                                Term::Literal(Literal::Number("2".to_string()), Span::dummy()),
                                Span::dummy(),
                            ),
                        ],
                        params,
                    );
                    assert_eq!(conj, &expected);
                }
                _ => panic!("Expected conjunction"),
            },
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_disjunction_with_limit() {
        let input = r#"rel test() {
            any(limit = 10) {
                a == 1,
                b == 2
            }
        }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => match &rel.body[0] {
                Goal::Disjunction(disj, _) => {
                    let mut params = SearchParams::new();
                    params.limit = Some(10);
                    let expected = Disjunction::with_params(
                        vec![
                            Goal::Equality(
                                Term::Variable("a".to_string(), Span::dummy()),
                                Term::Literal(Literal::Number("1".to_string()), Span::dummy()),
                                Span::dummy(),
                            ),
                            Goal::Equality(
                                Term::Variable("b".to_string(), Span::dummy()),
                                Term::Literal(Literal::Number("2".to_string()), Span::dummy()),
                                Span::dummy(),
                            ),
                        ],
                        params,
                    );
                    assert_eq!(disj, &expected);
                }
                _ => panic!("Expected disjunction"),
            },
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_conjunction_with_multiple_params() {
        let input = r#"rel test() {
            all(strategy = dfs, depth = 5, limit = 100) {
                a == 1,
                b == 2
            }
        }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => match &rel.body[0] {
                Goal::Conjunction(conj, _) => {
                    assert!(conj.params.is_some());
                    let params = conj.params.as_ref().unwrap();
                    assert_eq!(params.strategy, Some(SearchStrategy::Dfs));
                    assert_eq!(params.depth, Some(5));
                    assert_eq!(params.limit, Some(100));
                }
                _ => panic!("Expected conjunction"),
            },
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_nested_blocks() {
        let input = r#"rel test() {
            all {
                a == 1,
                any(strategy = dfs, depth = 3) {
                    b == 2,
                    c == 3
                }
            }
        }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => match &rel.body[0] {
                Goal::Conjunction(conj, _) => {
                    assert_eq!(conj.body.len(), 2);
                    match &conj.body[1] {
                        Goal::Disjunction(disj, _) => {
                            assert!(disj.params.is_some());
                            let params = disj.params.as_ref().unwrap();
                            assert_eq!(params.strategy, Some(SearchStrategy::Dfs));
                            assert_eq!(params.depth, Some(3));
                        }
                        _ => panic!("Expected nested disjunction"),
                    }
                }
                _ => panic!("Expected conjunction"),
            },
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_custom_params() {
        let input = r#"rel test() { 
            any(mode = "exhaustive", parallel = true) { a == b } 
        }"#;
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Disjunction(
                    Disjunction {
                        body: vec![Goal::Equality(
                            Term::Variable("a".to_string(), Span::dummy()),
                            Term::Variable("b".to_string(), Span::dummy()),
                            Span::dummy(),
                        )],
                        params: Some(SearchParams {
                            strategy: None,
                            limit: None,
                            depth: None,
                            custom_params: vec![
                                (
                                    "mode".to_string(),
                                    SearchParamValue::String("exhaustive".to_string()),
                                ),
                                ("parallel".to_string(), SearchParamValue::Boolean(true)),
                            ],
                        }),
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_invalid_strategy() {
        let input = r#"rel test() {
            all(strategy = invalid) {
                a == 1
            }
        }"#;
        assert!(parse_str(input).is_err());
    }

    #[test]
    fn test_parse_invalid_param_value() {
        let input = r#"rel test() {
            all(limit = "not a number") {
                a == 1
            }
        }"#;
        assert!(parse_str(input).is_err());
    }

    #[test]
    fn test_parse_empty_blocks() {
        let input = r#"rel test() {
            all { }
            any { }
        }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => {
                assert_eq!(rel.body.len(), 2);
                match &rel.body[0] {
                    Goal::Conjunction(conj, _) => {
                        let expected = Conjunction::new(vec![]);
                        assert_eq!(conj, &expected);
                    }
                    _ => panic!("Expected conjunction"),
                }
                match &rel.body[1] {
                    Goal::Disjunction(disj, _) => {
                        let expected = Disjunction::new(vec![]);
                        assert_eq!(disj, &expected);
                    }
                    _ => panic!("Expected disjunction"),
                }
            }
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_search_params() {
        let input = "rel test() @bfs { a == b }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: Some(SearchStrategy::Bfs),
                body: vec![Goal::Equality(
                    Term::Variable("a".to_string(), Span::dummy()),
                    Term::Variable("b".to_string(), Span::dummy()),
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_all_block() {
        let input = "rel test() { all(strategy = dfs, limit = 100) { a == b } }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Conjunction(
                    Conjunction {
                        body: vec![Goal::Equality(
                            Term::Variable("a".to_string(), Span::dummy()),
                            Term::Variable("b".to_string(), Span::dummy()),
                            Span::dummy(),
                        )],
                        params: Some(SearchParams {
                            strategy: Some(SearchStrategy::Dfs),
                            limit: Some(100),
                            depth: None,
                            custom_params: vec![],
                        }),
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_any_block() {
        let input = "rel test() { any(strategy = bfs, depth = 5) { a == b } }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Disjunction(
                    Disjunction {
                        body: vec![Goal::Equality(
                            Term::Variable("a".to_string(), Span::dummy()),
                            Term::Variable("b".to_string(), Span::dummy()),
                            Span::dummy(),
                        )],
                        params: Some(SearchParams {
                            strategy: Some(SearchStrategy::Bfs),
                            limit: None,
                            depth: Some(5),
                            custom_params: vec![],
                        }),
                    },
                    Span::dummy(),
                )],
            })],
            span: Span::dummy(),
        };
        assert_eq!(ast, expected);
    }

    // =============================================================================
    // Constraint Block Parsing Tests
    // =============================================================================

    #[test]
    fn test_parse_constraint_block_simple() {
        let input = r#"rel test() { constraint(domain="clpfd") { x in 1..5 } }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => {
                assert_eq!(rel.body.len(), 1);
                match &rel.body[0] {
                    Goal::ConstraintBlock(block, _) => {
                        assert_eq!(block.domain, "clpfd");
                        assert_eq!(block.body.raw_content, "x in 1..5 ");
                    }
                    _ => panic!("Expected constraint block"),
                }
            }
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_constraint_block_balanced_braces() {
        let input = r#"rel test() { constraint(domain="clpfd") { x in {{foo}, 1, 2} } }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => {
                assert_eq!(rel.body.len(), 1);
                match &rel.body[0] {
                    Goal::ConstraintBlock(block, _) => {
                        assert_eq!(block.domain, "clpfd");
                        assert_eq!(block.body.raw_content, "x in {{foo}, 1, 2} ");
                    }
                    _ => panic!("Expected constraint block"),
                }
            }
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_constraint_block_string_with_braces() {
        let input = r#"rel test() { constraint(domain="clpfd") { x in {"}", 1, 2}, y != {"{"} } }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => {
                assert_eq!(rel.body.len(), 1);
                match &rel.body[0] {
                    Goal::ConstraintBlock(block, _) => {
                        assert_eq!(block.domain, "clpfd");
                        assert_eq!(block.body.raw_content, r#"x in {"}", 1, 2}, y != {"{"} "#);
                    }
                    _ => panic!("Expected constraint block"),
                }
            }
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_constraint_block_complex_nested_braces() {
        let input = r#"rel test() { constraint(domain="clpfd") { x in {{min_val}, {max_val}}, y in {{start}, {end}..10} } }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => {
                assert_eq!(rel.body.len(), 1);
                match &rel.body[0] {
                    Goal::ConstraintBlock(block, _) => {
                        assert_eq!(block.domain, "clpfd");
                        assert_eq!(
                            block.body.raw_content,
                            "x in {{min_val}, {max_val}}, y in {{start}, {end}..10} "
                        );
                    }
                    _ => panic!("Expected constraint block"),
                }
            }
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_constraint_block_mixed_braces_and_strings() {
        let input = r#"rel test() { constraint(domain="clpfd") { x in {{foo}, "}", 2}, y in {"{", {bar}, "}"} } }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => {
                assert_eq!(rel.body.len(), 1);
                match &rel.body[0] {
                    Goal::ConstraintBlock(block, _) => {
                        assert_eq!(block.domain, "clpfd");
                        assert_eq!(
                            block.body.raw_content,
                            r#"x in {{foo}, "}", 2}, y in {"{", {bar}, "}"} "#
                        );
                    }
                    _ => panic!("Expected constraint block"),
                }
            }
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_constraint_block_deeply_nested_braces() {
        let input = r#"rel test() { constraint(domain="clpfd") { x in {{{nested}, {values}}, 1}, y in {{{{deep}}, nested}} } }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => {
                assert_eq!(rel.body.len(), 1);
                match &rel.body[0] {
                    Goal::ConstraintBlock(block, _) => {
                        assert_eq!(block.domain, "clpfd");
                        assert_eq!(
                            block.body.raw_content,
                            "x in {{{nested}, {values}}, 1}, y in {{{{deep}}, nested}} "
                        );
                    }
                    _ => panic!("Expected constraint block"),
                }
            }
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_constraint_block_strings_with_nested_quotes() {
        let input = r#"rel test() { constraint(domain="clpfd") { x in {"}", "}", "{"}, y != {"{{inner}}"} } }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => {
                assert_eq!(rel.body.len(), 1);
                match &rel.body[0] {
                    Goal::ConstraintBlock(block, _) => {
                        assert_eq!(block.domain, "clpfd");
                        assert_eq!(
                            block.body.raw_content,
                            r#"x in {"}", "}", "{"}, y != {"{{inner}}"} "#
                        );
                    }
                    _ => panic!("Expected constraint block"),
                }
            }
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_constraint_block_multiline_with_braces() {
        let input = r#"rel test() { 
            constraint(domain="clpfd") { 
                x in {{foo}, 1, 2},
                y in {"}", {bar}},
                z != {"{", "}"} 
            } 
        }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => {
                assert_eq!(rel.body.len(), 1);
                match &rel.body[0] {
                    Goal::ConstraintBlock(block, _) => {
                        assert_eq!(block.domain, "clpfd");
                        // The raw content should preserve the original formatting
                        assert!(block.body.raw_content.contains("x in {{foo}, 1, 2}"));
                        assert!(block.body.raw_content.contains(r#"y in {"}", {bar}}"#));
                        assert!(block.body.raw_content.contains(r#"z != {"{", "}"}"#));
                    }
                    _ => panic!("Expected constraint block"),
                }
            }
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_constraint_block_default_domain() {
        let input = r#"rel test() { constraint { x in {{foo}, 1} } }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => {
                assert_eq!(rel.body.len(), 1);
                match &rel.body[0] {
                    Goal::ConstraintBlock(block, _) => {
                        assert_eq!(block.domain, "clpfd"); // Default domain
                        assert_eq!(block.body.raw_content, "x in {{foo}, 1} ");
                    }
                    _ => panic!("Expected constraint block"),
                }
            }
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_parse_constraint_block_custom_domain() {
        let input = r#"rel test() { constraint(domain="clpz") { x + y == {{sum}} } }"#;
        let ast = parse_str(input).unwrap();
        match &ast.items[0] {
            Item::Relation(rel) => {
                assert_eq!(rel.body.len(), 1);
                match &rel.body[0] {
                    Goal::ConstraintBlock(block, _) => {
                        assert_eq!(block.domain, "clpz");
                        assert_eq!(block.body.raw_content, "x + y == {{sum}} ");
                    }
                    _ => panic!("Expected constraint block"),
                }
            }
            _ => panic!("Expected relation"),
        }
    }
}
