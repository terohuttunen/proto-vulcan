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
        Rule::mod_declaration => {
            // Handle both simple declarations and body definitions
            let inner = pair.clone().into_inner().next().unwrap();
            match inner.as_rule() {
                Rule::mod_declaration_simple => {
                    Ok(Item::ModuleDeclaration(build_mod_declaration(pair)?))
                }
                Rule::mod_declaration_body => Ok(Item::Module(build_mod_definition(inner)?)),
                _ => Err(ParseError::UnexpectedRule(inner.as_rule())),
            }
        }
        Rule::struct_definition => Ok(Item::Struct(build_struct_definition(pair)?)),
        Rule::impl_block => Ok(Item::Impl(build_impl_block(pair)?)),
        Rule::predicate_definition => Ok(Item::Predicate(build_predicate_definition(pair)?)),
        _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
    }
}

fn build_use_statement(pair: Pair<Rule>) -> ParseResult<UseStatement> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let _use_keyword = inner.next().unwrap(); // Skip the use_keyword
    let path_pair = inner.next().unwrap(); // Get the use_path
    let path = build_use_path(path_pair)?;
    Ok(UseStatement { path, span })
}

fn build_mod_declaration(pair: Pair<Rule>) -> ParseResult<ModuleDeclaration> {
    let span = pair_to_span(&pair);
    let inner = pair.into_inner().next().unwrap(); // Get the specific variant

    match inner.as_rule() {
        Rule::mod_declaration_simple => build_mod_declaration_simple(inner),
        Rule::mod_declaration_body => build_mod_declaration_body(inner),
        _ => Err(ParseError::UnexpectedRule(inner.as_rule())),
    }
}

fn build_mod_declaration_simple(pair: Pair<Rule>) -> ParseResult<ModuleDeclaration> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();

    // Look at all pairs to determine structure
    let pairs: Vec<_> = inner.clone().collect();

    let (visibility, name) = if !pairs.is_empty() && pairs[0].as_rule() == Rule::visibility {
        // Has visibility: visibility, mod_keyword, ident, ";"
        let vis = build_visibility(pairs[0].clone())?;
        let name = pairs[2].as_str().to_string(); // ident comes after visibility and mod_keyword
        (vis, name)
    } else {
        // No visibility: mod_keyword, ident, ";"
        let name = pairs[1].as_str().to_string(); // ident comes after mod_keyword
        (ast::Visibility::Private, name)
    };

    Ok(ModuleDeclaration {
        visibility,
        name,
        span,
    })
}

fn build_mod_declaration_body(pair: Pair<Rule>) -> ParseResult<ModuleDeclaration> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();

    // Look at all pairs to determine structure
    let pairs: Vec<_> = inner.clone().collect();

    let (visibility, name) = if !pairs.is_empty() && pairs[0].as_rule() == Rule::visibility {
        // Has visibility: visibility, mod_keyword, ident, search_strategy?, "{", body content, "}"
        let vis = build_visibility(pairs[0].clone())?;
        let name = pairs[2].as_str().to_string(); // ident comes after visibility and mod_keyword
        (vis, name)
    } else {
        // No visibility: mod_keyword, ident, search_strategy?, "{", body content, "}"
        let name = pairs[1].as_str().to_string(); // ident comes after mod_keyword
        (ast::Visibility::Private, name)
    };

    Ok(ModuleDeclaration {
        visibility,
        name,
        span,
    })
}

fn build_use_path(pair: Pair<Rule>) -> ParseResult<UsePath> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| ParseError::MissingRule(Rule::use_path))?;

    match inner.as_rule() {
        Rule::use_path_simple => {
            // Simple import: use use_path_base;
            let base_pair = inner.into_inner().next().unwrap(); // use_path_base
            let qualified_path = build_use_path_base(base_pair)?;

            // For simple use paths, split the path from the item name
            let (path, item) = match qualified_path {
                QualifiedPath::Relative(mut segments) if !segments.is_empty() => {
                    let item = segments.pop().unwrap();
                    let path = QualifiedPath::Relative(segments);
                    (path, item)
                }
                QualifiedPath::Global(mut segments) if !segments.is_empty() => {
                    let item = segments.pop().unwrap();
                    let path = QualifiedPath::Global(segments);
                    (path, item)
                }
                QualifiedPath::Absolute(mut segments) if !segments.is_empty() => {
                    let item = segments.pop().unwrap();
                    let path = QualifiedPath::Absolute(segments);
                    (path, item)
                }

                QualifiedPath::Self_(mut segments) if !segments.is_empty() => {
                    let item = segments.pop().unwrap();
                    let path = QualifiedPath::Self_(segments);
                    (path, item)
                }
                QualifiedPath::Super(levels, mut segments) if !segments.is_empty() => {
                    let item = segments.pop().unwrap();
                    let path = QualifiedPath::Super(levels, segments);
                    (path, item)
                }
                QualifiedPath::External(crate_name, mut segments) if !segments.is_empty() => {
                    let item = segments.pop().unwrap();
                    let path = QualifiedPath::External(crate_name, segments);
                    (path, item)
                }
                // Handle cases where we have a single item after the path root
                QualifiedPath::Relative(segments) if segments.len() == 1 => {
                    let item = segments[0].clone();
                    let path = QualifiedPath::Relative(vec![]);
                    (path, item)
                }
                QualifiedPath::Global(segments) if segments.len() == 1 => {
                    let item = segments[0].clone();
                    let path = QualifiedPath::Global(vec![]);
                    (path, item)
                }
                QualifiedPath::Absolute(segments) if segments.len() == 1 => {
                    let item = segments[0].clone();
                    let path = QualifiedPath::Absolute(vec![]);
                    (path, item)
                }
                QualifiedPath::Self_(segments) if segments.len() == 1 => {
                    let item = segments[0].clone();
                    let path = QualifiedPath::Self_(vec![]);
                    (path, item)
                }
                QualifiedPath::Super(levels, segments) if segments.len() == 1 => {
                    let item = segments[0].clone();
                    let path = QualifiedPath::Super(levels, vec![]);
                    (path, item)
                }
                QualifiedPath::External(crate_name, segments) if segments.len() == 1 => {
                    let item = segments[0].clone();
                    let path = QualifiedPath::External(crate_name, vec![]);
                    (path, item)
                }
                _ => return Err(ParseError::UnexpectedRule(Rule::qualified_path)),
            };

            Ok(UsePath::Simple(path, item))
        }
        Rule::use_path_glob => {
            // Glob import: use use_path_base::*;
            let mut parts = inner.into_inner();
            let use_path_base_pair = parts.next().unwrap();

            let qualified_path = build_use_path_base(use_path_base_pair)?;
            Ok(UsePath::Glob(qualified_path))
        }
        Rule::use_path_list => {
            // List import: use use_path_base{...};
            let mut parts = inner.into_inner();
            let use_path_base_pair = parts.next().unwrap();
            let _path_sep = parts.next().unwrap(); // Skip the path_sep
            let list_part = parts.next().unwrap(); // Get the list_import

            let qualified_path = build_use_path_base(use_path_base_pair)?;

            let mut imports = vec![];
            for import_item in list_part.into_inner() {
                let mut item_inner = import_item.into_inner();
                let name = item_inner.next().unwrap().as_str().to_string();
                let alias = item_inner.next().map(|p| p.as_str().to_string());
                imports.push((name, alias));
            }
            Ok(UsePath::List(qualified_path, imports))
        }
        _ => Err(ParseError::UnexpectedRule(inner.as_rule())),
    }
}

fn build_type_name(pair: Pair<Rule>) -> ParseResult<String> {
    match pair.as_rule() {
        Rule::type_name => {
            let inner = pair.into_inner().next().unwrap();
            match inner.as_rule() {
                Rule::qualified_path => {
                    let qualified_path = build_qualified_path(inner)?;
                    Ok(format!("{}", qualified_path))
                }
                Rule::ident => Ok(inner.as_str().to_string()),
                _ => Err(ParseError::UnexpectedRule(inner.as_rule())),
            }
        }
        Rule::ident => Ok(pair.as_str().to_string()),
        _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
    }
}

fn build_use_path_segments(pair: Pair<Rule>) -> ParseResult<Vec<String>> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::absolute_path | Rule::crate_path | Rule::super_path | Rule::self_path => {
            // Use existing qualified path logic but extract just the segments
            let qualified_path = build_qualified_path(inner)?;
            Ok(qualified_path.segments().to_vec())
        }
        Rule::simple_segments => {
            // Simple path like "a::b::c"
            let segments: Vec<String> = inner
                .into_inner()
                .filter(|p| p.as_rule() == Rule::ident)
                .map(|p| p.as_str().to_string())
                .collect();
            Ok(segments)
        }
        _ => Err(ParseError::UnexpectedRule(inner.as_rule())),
    }
}

fn build_qualified_path(pair: Pair<Rule>) -> ParseResult<QualifiedPath> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::absolute_path => {
            // absolute_path = { "::" ~ simple_segments }
            // Find the simple_segments among the parts
            let parts: Vec<_> = inner.into_inner().collect();

            // Look for simple_segments among the parts
            if let Some(simple_segments) =
                parts.iter().find(|p| p.as_rule() == Rule::simple_segments)
            {
                let segments: Vec<String> = simple_segments
                    .clone()
                    .into_inner()
                    .filter(|p| p.as_rule() == Rule::ident)
                    .map(|p| p.as_str().to_string())
                    .collect();
                Ok(QualifiedPath::Global(segments))
            } else {
                // Just "::" with no segments
                Ok(QualifiedPath::Global(vec![]))
            }
        }
        Rule::crate_path => {
            // crate_path = { crate_keyword ~ (path_sep ~ simple_segments)? }
            // Find the simple_segments among the parts
            let parts: Vec<_> = inner.into_inner().collect();

            // Look for simple_segments among the parts
            if let Some(simple_segments) =
                parts.iter().find(|p| p.as_rule() == Rule::simple_segments)
            {
                let segments: Vec<String> = simple_segments
                    .clone()
                    .into_inner()
                    .filter(|p| p.as_rule() == Rule::ident)
                    .map(|p| p.as_str().to_string())
                    .collect();
                Ok(QualifiedPath::Absolute(segments))
            } else {
                // Just "crate" with no segments
                Ok(QualifiedPath::Absolute(vec![]))
            }
        }
        Rule::super_path => {
            // super_path = { super_keyword ~ (path_sep ~ simple_segments)? }
            // Find the simple_segments among the parts
            let parts: Vec<_> = inner.into_inner().collect();

            // Look for simple_segments among the parts
            if let Some(simple_segments) =
                parts.iter().find(|p| p.as_rule() == Rule::simple_segments)
            {
                let segments: Vec<String> = simple_segments
                    .clone()
                    .into_inner()
                    .filter(|p| p.as_rule() == Rule::ident)
                    .map(|p| p.as_str().to_string())
                    .collect();
                Ok(QualifiedPath::Super(0, segments))
            } else {
                // Just "super" with no segments
                Ok(QualifiedPath::Super(0, vec![]))
            }
        }
        Rule::self_path => {
            // self_path = { self_keyword ~ (path_sep ~ simple_segments)? }
            // Find the simple_segments among the parts
            let parts: Vec<_> = inner.into_inner().collect();

            // Look for simple_segments among the parts
            if let Some(simple_segments) =
                parts.iter().find(|p| p.as_rule() == Rule::simple_segments)
            {
                let segments: Vec<String> = simple_segments
                    .clone()
                    .into_inner()
                    .filter(|p| p.as_rule() == Rule::ident)
                    .map(|p| p.as_str().to_string())
                    .collect();
                Ok(QualifiedPath::Self_(segments))
            } else {
                // Just "self" with no segments
                Ok(QualifiedPath::Self_(vec![]))
            }
        }

        Rule::external_path => {
            // external_path = { ident ~ "::" ~ simple_segments }
            // This creates only 2 tokens: [ident, simple_segments]
            let mut parts = inner.into_inner();

            let crate_name = parts.next().unwrap().as_str().to_string();
            let simple_segments = parts.next().unwrap(); // This should always exist

            let segments: Vec<String> = simple_segments
                .into_inner()
                .filter(|p| p.as_rule() == Rule::ident)
                .map(|p| p.as_str().to_string())
                .collect();

            Ok(QualifiedPath::External(crate_name, segments))
        }
        Rule::relative_path => {
            // relative_path = { simple_segments }
            let simple_segments = inner.into_inner().next().unwrap();
            let segments: Vec<String> = simple_segments
                .into_inner()
                .filter(|p| p.as_rule() == Rule::ident)
                .map(|p| p.as_str().to_string())
                .collect();

            // Check if the first segment is an external crate (like "std")
            if let Some(first) = segments.first() {
                if first == "std" && segments.len() > 1 {
                    // Convert std::... to External("std", [...])
                    let mut remaining = segments;
                    remaining.remove(0); // Remove "std"
                    return Ok(QualifiedPath::External("std".to_string(), remaining));
                }
            }

            Ok(QualifiedPath::Relative(segments))
        }
        _ => Err(ParseError::UnexpectedRule(inner.as_rule())),
    }
}

fn build_use_path_base(pair: Pair<Rule>) -> ParseResult<QualifiedPath> {
    match pair.as_rule() {
        Rule::use_path_base => {
            // Use_path_base now contains qualified_path
            let inner = pair.into_inner().next().unwrap();
            build_qualified_path(inner)
        }
        Rule::qualified_path => build_qualified_path(pair),
        _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
    }
}

fn build_mod_definition(pair: Pair<Rule>) -> ParseResult<ModuleDefinition> {
    let span = pair_to_span(&pair);
    let inner = pair.into_inner();

    // Look at all pairs to determine structure
    let pairs: Vec<_> = inner.collect();

    let (visibility, name_idx) = if !pairs.is_empty() && pairs[0].as_rule() == Rule::visibility {
        (build_visibility(pairs[0].clone())?, 1) // visibility, name (we skip "mod" keyword)
    } else {
        (ast::Visibility::Private, 0) // name (we skip "mod" keyword)
    };

    // Find the ident among the pairs
    let name = pairs
        .iter()
        .find(|p| p.as_rule() == Rule::ident)
        .ok_or(ParseError::UnexpectedRule(Rule::ident))?
        .as_str()
        .to_string();
    let mut search_strategy = None;
    let mut items = vec![];

    for part in pairs.iter() {
        match part.as_rule() {
            Rule::search_strategy => search_strategy = Some(build_search_strategy(part.clone())?),
            Rule::use_statement
            | Rule::mod_declaration
            | Rule::mod_declaration
            | Rule::struct_definition
            | Rule::impl_block
            | Rule::predicate_definition => items.push(build_item(part.clone())?),
            _ => (),
        }
    }
    Ok(ModuleDefinition {
        visibility,
        name,
        search_strategy,
        items,
        span,
    })
}

fn build_struct_definition(pair: Pair<Rule>) -> ParseResult<StructDefinition> {
    let span = pair_to_span(&pair);
    let inner = pair.into_inner();

    // Look at all pairs to determine structure
    let pairs: Vec<_> = inner.collect();

    // Find the definition pair by iterating through all pairs
    let mut definition_pair = None;
    for pair in &pairs {
        if pair.as_rule() == Rule::named_struct_def || pair.as_rule() == Rule::tuple_struct_def {
            definition_pair = Some(pair.clone());
            break;
        }
    }

    let def_pair =
        definition_pair.ok_or_else(|| ParseError::MissingRule(Rule::named_struct_def))?;

    let (visibility, name) = if !pairs.is_empty() && pairs[0].as_rule() == Rule::visibility {
        // Has visibility: visibility, struct_keyword, type_name
        let vis = build_visibility(pairs[0].clone())?;
        let name = build_type_name(pairs[2].clone())?; // type_name after visibility and struct_keyword
        (vis, name)
    } else {
        // No visibility: struct_keyword, type_name
        let name = build_type_name(pairs[1].clone())?; // type_name after struct_keyword
        (ast::Visibility::Private, name)
    };
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
        visibility,
        name,
        kind,
        span,
    })
}

fn build_named_field(pair: Pair<Rule>) -> ParseResult<NamedField> {
    let span = pair_to_span(&pair);
    let inner = pair.into_inner();

    // Look at all pairs to determine structure
    let pairs: Vec<_> = inner.collect();

    let (visibility, name, type_name) =
        if !pairs.is_empty() && pairs[0].as_rule() == Rule::visibility {
            // Has visibility: visibility, ident, type_name
            let vis = build_visibility(pairs[0].clone())?;
            let name = pairs[1].as_str().to_string(); // ident after visibility
            let type_name = build_type_name(pairs[2].clone())?; // type_name after visibility and ident
            (vis, name, type_name)
        } else {
            // No visibility: ident, type_name
            let name = pairs[0].as_str().to_string(); // first element is ident
            let type_name = build_type_name(pairs[1].clone())?; // type_name after ident
            (ast::Visibility::Private, name, type_name)
        };
    // name and type_name are already Strings from above

    Ok(NamedField {
        visibility,
        name,
        type_name,
        span,
    })
}

fn build_impl_block(pair: Pair<Rule>) -> ParseResult<ImplBlock> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let type_name = build_type_name(inner.next().unwrap())?;
    let mut predicates = vec![];
    for rel_pair in inner {
        if rel_pair.as_rule() == Rule::predicate_definition {
            predicates.push(build_predicate_definition(rel_pair)?);
        }
    }
    Ok(ImplBlock {
        type_name,
        predicates,
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

fn build_predicate_definition(pair: Pair<Rule>) -> ParseResult<PredicateDefinition> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();
    let mut attributes = vec![];
    let mut search_strategy = None;

    // First collect all attributes
    while let Some(p) = inner.peek() {
        if p.as_rule() == Rule::attribute {
            attributes.push(build_attribute(inner.next().unwrap())?);
        } else {
            break;
        }
    }

    // Check if next element is visibility
    let next_pair = inner.peek().unwrap();
    let visibility = if next_pair.as_rule() == Rule::visibility {
        build_visibility(inner.next().unwrap())?
    } else {
        ast::Visibility::Private
    };

    // Now we must have `relation_keyword`, `ident`, `(params)`, optionally `search_strategy`, and `{body}`
    let relation_kind_pair = inner.next().unwrap();
    let predicate_kind = match relation_kind_pair.as_str() {
        "rel" => ast::PredicateKind::Relation,
        "macro" => ast::PredicateKind::Macro,
        _ => return Err(ParseError::UnexpectedRule(relation_kind_pair.as_rule())),
    };

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

    // Validate that the correct keyword is used based on parameter types
    // Only non-relational parameters (int, string, bool) require 'macro' keyword
    // Relational parameters (rel(arity)) should use 'rel' keyword
    let has_non_relational_params = parameters.iter().any(|p| {
        if let Some(type_annotation) = &p.type_annotation {
            matches!(
                type_annotation,
                crate::interpreter::metaprogramming::TypeAnnotation::Int
                    | crate::interpreter::metaprogramming::TypeAnnotation::String
                    | crate::interpreter::metaprogramming::TypeAnnotation::Bool
            )
        } else {
            false
        }
    });

    match (predicate_kind, has_non_relational_params) {
        (ast::PredicateKind::Relation, true) => {
            return Err(ParseError::Pest(pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message:
                        "Predicates with non-relational parameters (int, string, bool) must use 'macro' keyword instead of 'rel'"
                            .to_string(),
                },
                relation_kind_pair.as_span(),
            )));
        }
        _ => {} // Valid combinations: rel with only relational params, macro with any params
    }

    Ok(PredicateDefinition {
        span,
        visibility,
        predicate_kind,
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
        .map(|p| parse_meta_type_annotation(p))
        .transpose()?;
    Ok(Parameter {
        name,
        type_annotation,
    })
}

fn parse_meta_type_annotation(pair: Pair<Rule>) -> ParseResult<TypeAnnotation> {
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
                let type_name = build_type_name(inner)?;
                Ok(TypeAnnotation::Custom(type_name))
            }
            _ => parse_type_annotation(inner.as_str()),
        }
    } else {
        // No inner pairs - this is a terminal type (int, string, bool)
        parse_type_annotation(pair_str)
    }
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
        Rule::relation_call => Ok(Goal::RelationCall(
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

fn build_relation_name(pair: Pair<Rule>) -> ParseResult<RelationName> {
    match pair.as_rule() {
        Rule::qualified_path => {
            match build_qualified_path(pair.clone()) {
                Ok(path) => {
                    if let Some(name) = path.final_segment() {
                        // Create the correct module path by preserving the path type but removing the final segment
                        let module_path = match &path {
                            QualifiedPath::Global(segments) => {
                                QualifiedPath::Global(path.module_segments().to_vec())
                            }
                            QualifiedPath::Absolute(segments) => {
                                QualifiedPath::Absolute(path.module_segments().to_vec())
                            }
                            QualifiedPath::Relative(segments) => {
                                QualifiedPath::Relative(path.module_segments().to_vec())
                            }
                            QualifiedPath::Super(levels, segments) => {
                                QualifiedPath::Super(*levels, path.module_segments().to_vec())
                            }
                            QualifiedPath::Self_(segments) => {
                                QualifiedPath::Self_(path.module_segments().to_vec())
                            }

                            QualifiedPath::External(crate_name, segments) => {
                                QualifiedPath::External(
                                    crate_name.clone(),
                                    path.module_segments().to_vec(),
                                )
                            }
                        };

                        // Semantic disambiguation: if the module path is empty, treat as simple name
                        if path.module_segments().is_empty() {
                            Ok(RelationName::Simple(name.clone()))
                        } else {
                            Ok(RelationName::Qualified(QualifiedName::new(
                                module_path,
                                name.clone(),
                            )))
                        }
                    } else {
                        Err(ParseError::MissingRule(Rule::ident))
                    }
                }
                Err(_) => {
                    // build_qualified_path failed, this might be a simple identifier that matched qualified_path rule
                    // Try to extract as a simple identifier
                    Ok(RelationName::Simple(pair.as_str().to_string()))
                }
            }
        }
        Rule::ident => Ok(RelationName::Simple(pair.as_str().to_string())),
        _ => {
            // Handle the case where we have a nested structure (relation_call might contain qualified_path or ident)
            // Try to find the first inner qualified_path or ident
            let rule = pair.as_rule();
            for inner in pair.into_inner() {
                match inner.as_rule() {
                    Rule::qualified_path => return build_relation_name(inner),
                    Rule::ident => return Ok(RelationName::Simple(inner.as_str().to_string())),
                    _ => continue,
                }
            }
            Err(ParseError::UnexpectedRule(rule))
        }
    }
}

fn build_relation_call(pair: Pair<Rule>) -> ParseResult<RelationCall> {
    let mut inner = pair.into_inner();
    let name_pair = inner.next().unwrap();
    let name = build_relation_name(name_pair)?;
    let mut args = vec![];
    for arg_pair in inner {
        args.push(build_call_argument(arg_pair)?);
    }
    Ok(RelationCall { name, args })
}

fn build_call_argument(pair: Pair<Rule>) -> ParseResult<CallArgument> {
    let span = pair_to_span(&pair);

    if pair.as_rule() == Rule::call_argument {
        // With the new grammar, call_argument has inner content that's either term or arithmetic_expr
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::arithmetic_expr => {
                let content = inner.as_str();
                match meta_parser::parse_meta_expression(content, &span) {
                    Ok(expr) => Ok(CallArgument::MetaExpression(expr)),
                    Err(_) => Err(ParseError::UnexpectedRule(Rule::arithmetic_expr)),
                }
            }
            _ => {
                // Assume it's a term rule
                build_term(inner).map(CallArgument::Term)
            }
        }
    } else {
        // Fallback for backward compatibility - try to parse as term first, then meta expression
        match build_term(pair.clone()) {
            Ok(term) => Ok(CallArgument::Term(term)),
            Err(_) => {
                // If term parsing fails, try to parse as a meta expression
                let content = pair.as_str();
                match meta_parser::parse_meta_expression(content, &span) {
                    Ok(expr) => Ok(CallArgument::MetaExpression(expr)),
                    Err(_) => {
                        // If both fail, return an error
                        Err(ParseError::UnexpectedRule(pair.as_rule()))
                    }
                }
            }
        }
    }
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
        Rule::compound_construction => Ok(Term::Compound(
            build_compound_construction(pair.clone())?,
            span,
        )),
        Rule::compound_construction_no_parens => {
            // Extract the qualified path from the inner pair to get clean string without whitespace
            let qualified_path_pair = pair.into_inner().next().unwrap(); // qualified_path
            let qualified_path = build_qualified_path(qualified_path_pair)?;
            let name = qualified_path.to_string();

            // Semantic disambiguation: if this is a simple identifier (no ::) with no args,
            // treat it as a variable instead of a compound construction
            if !name.contains("::") {
                Ok(Term::Variable(name, span))
            } else {
                Ok(Term::Compound(
                    CompoundConstruction { name, args: vec![] },
                    span,
                ))
            }
        }
        Rule::path_term => {
            let path_str = pair.as_str().to_string();
            Ok(Term::Variable(path_str, span))
        }
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
        Rule::variable => {
            // Extract clean identifier from atomic variable rule to avoid whitespace
            let clean_name = pair.into_inner().next().unwrap().as_str().to_string(); // Get the ident
            Ok(Pattern::Variable(clean_name))
        }
        Rule::wildcard => Ok(Pattern::Wildcard),
        Rule::list_pattern => Ok(Pattern::List(build_list_pattern(pair)?)),
        Rule::named_struct_pattern => Ok(Pattern::NamedStruct(build_named_struct_pattern(pair)?)),
        Rule::compound_pattern_with_parens => {
            Ok(Pattern::Compound(build_compound_pattern_with_parens(pair)?))
        }
        Rule::compound_pattern_no_parens => {
            let compound = build_compound_pattern_no_parens(pair)?;

            // Semantic disambiguation: if this is a simple identifier (no ::) with no args,
            // treat it as a variable instead of a compound pattern
            if !compound.name.contains("::") && compound.args.is_empty() {
                Ok(Pattern::Variable(compound.name))
            } else {
                Ok(Pattern::Compound(compound))
            }
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
    // Extract the qualified path from the inner pair to get clean string without whitespace
    let qualified_path_pair = pair.into_inner().next().unwrap(); // qualified_path
    let qualified_path = build_qualified_path(qualified_path_pair)?;
    let name = qualified_path.to_string();

    Ok(CompoundPattern { name, args: vec![] })
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
            let (condition, then_body, else_ifs, else_body) = build_meta_if_statement(inner)?;
            Ok(MetaStatement::If {
                condition,
                then_body,
                else_ifs,
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
    Vec<(
        crate::interpreter::metaprogramming::MetaExpression,
        GoalBody,
    )>,
    Option<GoalBody>,
)> {
    let span = pair_to_span(&pair);
    let mut inner = pair.into_inner();

    // Parse initial if condition and body
    let content = inner.next().unwrap().as_str();
    let condition = meta_parser::parse_meta_expression(content, &span)
        .map_err(|_| ParseError::UnexpectedRule(Rule::meta_if_statement))?;
    let then_body = build_goal_body(inner.next().unwrap())?;

    // Parse remaining elements (else if clauses and final else)
    let mut else_ifs = Vec::new();
    let mut else_body = None;

    while let Some(element) = inner.next() {
        match element.as_rule() {
            Rule::meta_expr_content => {
                // This should be an else if condition
                let else_if_condition = meta_parser::parse_meta_expression(element.as_str(), &span)
                    .map_err(|_| ParseError::UnexpectedRule(Rule::meta_if_statement))?;
                let else_if_body = build_goal_body(inner.next().unwrap())?;
                else_ifs.push((else_if_condition, else_if_body));
            }
            Rule::goal_body => {
                // This should be the final else body
                else_body = Some(build_goal_body(element)?);
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

fn build_visibility(pair: Pair<Rule>) -> ParseResult<ast::Visibility> {
    // Check if the visibility pair has any content (empty string means private)
    if pair.as_str().is_empty() {
        return Ok(ast::Visibility::Private);
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
                            Rule::crate_keyword => Ok(ast::Visibility::Crate),
                            Rule::super_keyword => Ok(ast::Visibility::Super),
                            Rule::self_keyword => Ok(ast::Visibility::SelfModule),
                            Rule::qualified_path => {
                                let qualified_path = build_qualified_path(scope_inner)?;
                                Ok(ast::Visibility::Restricted(qualified_path))
                            }
                            _ => Err(ParseError::UnexpectedRule(scope_inner.as_rule())),
                        }
                    } else {
                        // Empty scope
                        Ok(ast::Visibility::Public)
                    }
                } else {
                    Err(ParseError::UnexpectedRule(scope_pair.as_rule()))
                }
            } else {
                // Just pub without scope
                Ok(ast::Visibility::Public)
            }
        } else {
            Err(ParseError::UnexpectedRule(pub_pair.as_rule()))
        }
    } else {
        // No pub keyword means private
        Ok(ast::Visibility::Private)
    }
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
        let input = "pub macro my_rel(a: int, b: string) @dfs { a == b }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Public,
                predicate_kind: ast::PredicateKind::Macro,
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
                path: UsePath::Simple(
                    QualifiedPath::Relative(vec!["a".to_string(), "b".to_string()]),
                    "c".to_string(),
                ),
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
                path: UsePath::Glob(QualifiedPath::Relative(vec![
                    "a".to_string(),
                    "b".to_string(),
                ])),
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
                    QualifiedPath::Relative(vec!["a".to_string()]),
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
                visibility: ast::Visibility::Private,
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
                visibility: ast::Visibility::Public,
                name: "MyStruct".to_string(),
                kind: StructKind::Named(vec![
                    NamedField {
                        visibility: ast::Visibility::Public,
                        name: "field".to_string(),
                        type_name: "T".to_string(),
                        span: Span::dummy(),
                    },
                    NamedField {
                        visibility: ast::Visibility::Private,
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
                predicates: vec![PredicateDefinition {
                    span: Span::dummy(),
                    visibility: ast::Visibility::Private,
                    predicate_kind: ast::PredicateKind::Relation,
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
        let input = "rel test() { a == Option::Some(42) }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Equality(
                    Term::Variable("a".to_string(), Span::dummy()),
                    Term::Compound(
                        CompoundConstruction {
                            name: "Option::Some".to_string(),
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
                                        name: RelationName::Simple("succeed".to_string()),
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
                                        name: RelationName::Simple("fail".to_string()),
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
                                        name: RelationName::Simple("succeed".to_string()),
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
                                        name: RelationName::Simple("fail".to_string()),
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::RelationCall(
                    RelationCall {
                        name: RelationName::Simple("my_relation".to_string()),
                        args: vec![
                            CallArgument::Term(Term::Variable("a".to_string(), Span::dummy())),
                            CallArgument::Term(Term::Variable("b".to_string(), Span::dummy())),
                            CallArgument::Term(Term::Variable("c".to_string(), Span::dummy())),
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
                attributes: vec![],
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::RelationCall(
                    RelationCall {
                        name: RelationName::Simple("succeed".to_string()),
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
    fn test_parse_call_with_arithmetic_expression() {
        let input = "rel test() { factorial(n - 1, result) }";
        let result = parse_str(input);

        // For now, just check that it doesn't panic and see what we get
        match result {
            Ok(ast) => {
                println!("Parsed successfully: {:#?}", ast);
                // For now, just assert it parses without error
                assert!(true);
            }
            Err(e) => {
                println!("Parse error: {:#?}", e);
                panic!("Failed to parse: {:?}", e);
            }
        }
    }

    #[test]
    fn test_parse_disequality() {
        let input = "rel test() { a != b }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
                visibility: ast::Visibility::Private,
                name: "my_module".to_string(),
                search_strategy: Some(SearchStrategy::Bfs),
                items: vec![
                    Item::Use(UseStatement {
                        path: UsePath::Simple(
                            QualifiedPath::External(
                                "std".to_string(),
                                vec!["collections".to_string()],
                            ),
                            "HashMap".to_string(),
                        ),
                        span: Span::dummy(),
                    }),
                    Item::Struct(StructDefinition {
                        visibility: ast::Visibility::Private,
                        name: "Point".to_string(),
                        kind: StructKind::Named(vec![
                            NamedField {
                                visibility: ast::Visibility::Private,
                                name: "x".to_string(),
                                type_name: "i32".to_string(),
                                span: Span::dummy(),
                            },
                            NamedField {
                                visibility: ast::Visibility::Private,
                                name: "y".to_string(),
                                type_name: "i32".to_string(),
                                span: Span::dummy(),
                            },
                        ]),
                        span: Span::dummy(),
                    }),
                    Item::Predicate(PredicateDefinition {
                        span: Span::dummy(),
                        visibility: ast::Visibility::Private,
                        predicate_kind: ast::PredicateKind::Relation,
                        attributes: vec![],
                        name: "test".to_string(),
                        parameters: vec![],
                        search_strategy: None,
                        body: vec![Goal::RelationCall(
                            RelationCall {
                                name: RelationName::Simple("succeed".to_string()),
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            Item::Predicate(r) => r,
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
            Item::Predicate(r) => r,
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
            Item::Predicate(r) => r,
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
            Item::Predicate(r) => r,
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
            Item::Predicate(r) => r,
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            Item::Predicate(r) => r,
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
            Item::Predicate(r) => r,
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
            Item::Predicate(rel) => {
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
            Item::Predicate(rel) => {
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
            Item::Predicate(rel) => match &rel.body[0] {
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
            Item::Predicate(rel) => match &rel.body[0] {
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
            Item::Predicate(rel) => match &rel.body[0] {
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
            Item::Predicate(rel) => match &rel.body[0] {
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            Item::Predicate(rel) => {
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            items: vec![Item::Predicate(PredicateDefinition {
                span: Span::dummy(),
                visibility: ast::Visibility::Private,
                predicate_kind: ast::PredicateKind::Relation,
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
            Item::Predicate(rel) => {
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
            Item::Predicate(rel) => {
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
            Item::Predicate(rel) => {
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
            Item::Predicate(rel) => {
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
            Item::Predicate(rel) => {
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
            Item::Predicate(rel) => {
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
            Item::Predicate(rel) => {
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
            Item::Predicate(rel) => {
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
            Item::Predicate(rel) => {
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
            Item::Predicate(rel) => {
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

    #[test]
    fn test_debug_isolated_mod_declarations() {
        // Test individual mod declarations
        let input1 = "mod child;";
        let result1 = VulcanParser::parse(Rule::mod_declaration, input1);
        println!("DEBUG: Parsing '{}' -> {:?}", input1, result1.is_ok());
        assert!(result1.is_ok());

        let input2 = "pub mod utils;";
        let result2 = VulcanParser::parse(Rule::mod_declaration, input2);
        println!("DEBUG: Parsing '{}' -> {:?}", input2, result2.is_ok());
        assert!(result2.is_ok());

        // Test the exact content inside a module
        let module_content = r#"mod child;
                pub mod utils;
                
                rel helper() {
                    succeed()
                }"#;

        // Test if we can parse the module content without the surrounding module brackets

        let items = module_content
            .split('\n')
            .map(|line| line.trim())
            .filter(|line| !line.is_empty());

        for (i, item) in items.enumerate() {
            if item.starts_with("mod ") && item.ends_with(";") {
                let result = VulcanParser::parse(Rule::mod_declaration, item);
            } else if item.starts_with("rel ") {
                // Test if this could be confusing the parser
            }
        }
    }

    #[test]
    fn test_simple_global_qualified_path() {
        let input = "use ::std::HashMap;";
        let result = parse_str(input);
        if let Err(e) = &result {
            println!("Parse error: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "Failed to parse simple global path: {:?}",
            result
        );
    }

    #[test]
    fn test_simple_type_annotation() {
        let input = "rel test(x: ::std::HashMap) {}";
        let result = parse_str(input);
        if let Err(e) = &result {
            println!("Parse error: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "Failed to parse type annotation: {:?}",
            result
        );
    }

    #[test]
    fn test_simple_list_import() {
        let input = "use crate::types::{TypeA, TypeB};";
        let result = parse_str(input);
        if let Err(e) = &result {
            println!("Parse error: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "Failed to parse simple list import: {:?}",
            result
        );
    }

    #[test]
    fn test_simple_crate_path() {
        let input = "use crate::types;";
        let result = parse_str(input);
        if let Err(e) = &result {
            println!("Parse error: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "Failed to parse simple crate path: {:?}",
            result
        );
    }

    #[test]
    fn test_list_import_with_double_colons() {
        let input = "use crate::types::{TypeA, TypeB};";
        let result = parse_str(input);
        if let Err(e) = &result {
            println!("Parse error: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "Failed to parse list import with double colons: {:?}",
            result
        );
    }

    #[test]
    fn test_global_absolute_path() {
        let input = "rel test(x: ::std::collections::HashMap) {}";
        let result = parse_str(input);
        if let Err(e) = &result {
            println!("Parse error: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "Failed to parse global absolute path: {:?}",
            result
        );
    }
}

#[cfg(test)]
mod qualified_path_tests {
    use super::*;
    use crate::interpreter::parser::ast::{QualifiedName, QualifiedPath, RelationName};

    #[test]
    fn test_simple_relation_call() {
        let input = "rel test() { member(x, list) }";
        let result = parse_str(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_std_qualified_path() {
        let input = "rel test() { std::list::member(x, list) }";
        let result = parse_str(input);
        assert!(result.is_ok());

        // Extract the relation call and verify it parsed as std path
        if let Ok(program) = result {
            if let Some(Item::Predicate(pred)) = program.items.first() {
                if let Some(Goal::RelationCall(call, _)) = pred.body.first() {
                    if let RelationName::Qualified(qualified) = &call.name {
                        assert!(
                            matches!(qualified.path, QualifiedPath::External(ref crate_name, _) if crate_name == "std")
                        );
                        assert_eq!(qualified.name, "member");
                        assert_eq!(qualified.path.segments(), &["list"]);
                    } else {
                        panic!("Expected qualified relation name");
                    }
                } else {
                    panic!("Expected relation call in body");
                }
            } else {
                panic!("Expected predicate item");
            }
        }
    }

    #[test]
    fn test_crate_absolute_path() {
        let input = "rel test() { crate::solver::solve(constraint, result) }";
        let result = parse_str(input);
        assert!(result.is_ok());

        if let Ok(program) = result {
            if let Some(Item::Predicate(pred)) = program.items.first() {
                if let Some(Goal::RelationCall(call, _)) = pred.body.first() {
                    if let RelationName::Qualified(qualified) = &call.name {
                        assert!(matches!(qualified.path, QualifiedPath::Absolute(_)));
                        assert_eq!(qualified.name, "solve");
                        assert_eq!(qualified.path.segments(), &["solver"]);
                    } else {
                        panic!("Expected qualified relation name");
                    }
                }
            }
        }
    }

    #[test]
    fn test_global_namespace_path() {
        let input = "rel test() { ::global::item(x) }";
        let result = parse_str(input);
        assert!(result.is_ok());

        if let Ok(program) = result {
            if let Some(Item::Predicate(pred)) = program.items.first() {
                if let Some(Goal::RelationCall(call, _)) = pred.body.first() {
                    if let RelationName::Qualified(qualified) = &call.name {
                        assert!(matches!(qualified.path, QualifiedPath::Global(_)));
                        assert_eq!(qualified.name, "item");
                        assert_eq!(qualified.path.segments(), &["global"]);
                    } else {
                        panic!("Expected qualified relation name");
                    }
                }
            }
        }
    }

    #[test]
    fn test_super_path() {
        let input = "rel test() { super::parent::function(x) }";
        let result = parse_str(input);
        assert!(result.is_ok());

        if let Ok(program) = result {
            if let Some(Item::Predicate(pred)) = program.items.first() {
                if let Some(Goal::RelationCall(call, _)) = pred.body.first() {
                    if let RelationName::Qualified(qualified) = &call.name {
                        assert!(matches!(qualified.path, QualifiedPath::Super(0, _)));
                        assert_eq!(qualified.name, "function");
                        assert_eq!(qualified.path.segments(), &["parent"]);
                    } else {
                        panic!("Expected qualified relation name");
                    }
                }
            }
        }
    }

    #[test]
    fn test_self_path() {
        let input = "rel test() { self::local::helper(x) }";
        let result = parse_str(input);
        assert!(result.is_ok());

        if let Ok(program) = result {
            if let Some(Item::Predicate(pred)) = program.items.first() {
                if let Some(Goal::RelationCall(call, _)) = pred.body.first() {
                    if let RelationName::Qualified(qualified) = &call.name {
                        assert!(matches!(qualified.path, QualifiedPath::Self_(_)));
                        assert_eq!(qualified.name, "helper");
                        assert_eq!(qualified.path.segments(), &["local"]);
                    } else {
                        panic!("Expected qualified relation name");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod mod_declaration_tests {
    use super::*;
    use crate::interpreter::parser::ast::{Item, ModuleDeclaration};

    #[test]
    fn test_simple_mod_declaration() {
        let input = "mod my_module;";
        let result = VulcanParser::parse(Rule::mod_declaration, input);
        assert!(result.is_ok());

        let pair = result.unwrap().next().unwrap();
        let mod_decl = build_mod_declaration(pair).unwrap();

        assert_eq!(mod_decl.name, "my_module");
        assert!(mod_decl.visibility == ast::Visibility::Private);
    }

    #[test]
    fn test_pub_mod_declaration() {
        let input = "pub mod utils;";
        let result = VulcanParser::parse(Rule::mod_declaration, input);
        assert!(result.is_ok());

        let pair = result.unwrap().next().unwrap();
        let mod_decl = build_mod_declaration(pair).unwrap();

        assert_eq!(mod_decl.name, "utils");
        assert!(mod_decl.visibility == ast::Visibility::Public);
    }

    #[test]
    fn test_mod_declaration_in_program() {
        let input = r#"
            mod utils;
            pub mod solver;
            
            rel test() {
                succeed()
            }
        "#;

        let result = VulcanParser::parse(Rule::program, input);
        assert!(result.is_ok());

        let pair = result.unwrap().next().unwrap();
        let program = build_program(pair).unwrap();

        assert_eq!(program.items.len(), 3);

        // Check first module declaration
        if let Item::ModuleDeclaration(mod_decl) = &program.items[0] {
            assert_eq!(mod_decl.name, "utils");
            assert!(mod_decl.visibility == ast::Visibility::Private);
        } else {
            panic!("Expected ModuleDeclaration");
        }

        // Check second module declaration
        if let Item::ModuleDeclaration(mod_decl) = &program.items[1] {
            assert_eq!(mod_decl.name, "solver");
            assert!(mod_decl.visibility == ast::Visibility::Public);
        } else {
            panic!("Expected ModuleDeclaration");
        }

        // Check predicate
        assert!(matches!(program.items[2], Item::Predicate(_)));
    }

    #[test]
    fn test_mod_declaration_in_module() {
        let input = r#"
            mod parent {
                mod child;
                pub mod utils;
                
                rel helper() {
                    succeed()
                }
            }
        "#;

        let result = VulcanParser::parse(Rule::program, input);
        assert!(result.is_ok());

        let pair = result.unwrap().next().unwrap();
        let program = build_program(pair).unwrap();

        assert_eq!(program.items.len(), 1);

        if let Item::Module(module) = &program.items[0] {
            assert_eq!(module.name, "parent");
            assert_eq!(module.items.len(), 3);

            // Check child module declaration
            if let Item::ModuleDeclaration(mod_decl) = &module.items[0] {
                assert_eq!(mod_decl.name, "child");
                assert!(mod_decl.visibility == ast::Visibility::Private);
            } else {
                panic!("Expected ModuleDeclaration");
            }

            // Check utils module declaration
            if let Item::ModuleDeclaration(mod_decl) = &module.items[1] {
                assert_eq!(mod_decl.name, "utils");
                assert!(mod_decl.visibility == ast::Visibility::Public);
            } else {
                panic!("Expected ModuleDeclaration");
            }

            // Check predicate
            assert!(matches!(module.items[2], Item::Predicate(_)));
        } else {
            panic!("Expected Module");
        }
    }

    #[test]
    fn test_display_mod_declaration() {
        let mod_decl = ModuleDeclaration {
            visibility: ast::Visibility::Private,
            name: "test_module".to_string(),
            span: Span::dummy(),
        };
        assert_eq!(format!("{}", mod_decl), "mod test_module;");

        let pub_mod_decl = ModuleDeclaration {
            visibility: ast::Visibility::Public,
            name: "public_module".to_string(),
            span: Span::dummy(),
        };
        assert_eq!(format!("{}", pub_mod_decl), "pub mod public_module;");
    }
}

// Add comprehensive module system tests
#[cfg(test)]
mod comprehensive_module_tests {
    use super::*;

    #[test]
    fn test_visibility_modifiers() {
        // Test all visibility modifier combinations
        let test_cases = vec![
            ("pub struct Foo {}", "pub"),
            ("pub(crate) struct Foo {}", "pub(crate)"),
            ("pub(super) struct Foo {}", "pub(super)"),
            ("pub(self) struct Foo {}", "pub(self)"),
            ("struct Foo {}", "private"),
            ("pub rel test() {}", "pub rel"),
            ("pub(crate) rel test() {}", "pub(crate) rel"),
        ];

        for (input, description) in test_cases {
            let result = parse_str(input);
            assert!(
                result.is_ok(),
                "Failed to parse {}: {:?}",
                description,
                result
            );

            let ast = result.unwrap();
            assert!(!ast.items.is_empty(), "No items parsed for {}", description);

            // Verify visibility is correctly set
            match &ast.items[0] {
                Item::Struct(s) => match description {
                    "pub" => assert_eq!(s.visibility, ast::Visibility::Public),
                    "pub(crate)" => assert_eq!(s.visibility, ast::Visibility::Crate),
                    "pub(super)" => assert_eq!(s.visibility, ast::Visibility::Super),
                    "pub(self)" => assert_eq!(s.visibility, ast::Visibility::SelfModule),
                    "private" => assert_eq!(s.visibility, ast::Visibility::Private),
                    _ => {}
                },
                Item::Predicate(p) => match description {
                    "pub relation" => assert_eq!(p.visibility, ast::Visibility::Public),
                    "pub(crate) relation" => assert_eq!(p.visibility, ast::Visibility::Crate),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    #[test]
    fn test_module_declarations() {
        let test_cases = vec![
            ("mod foo;", "foo", ast::Visibility::Private),
            ("pub mod bar;", "bar", ast::Visibility::Public),
            ("pub(crate) mod baz;", "baz", ast::Visibility::Crate),
        ];

        for (input, expected_name, expected_vis) in test_cases {
            let result = parse_str(input);
            assert!(
                result.is_ok(),
                "Failed to parse module declaration: {}",
                input
            );

            let ast = result.unwrap();
            assert_eq!(ast.items.len(), 1);

            match &ast.items[0] {
                Item::ModuleDeclaration(decl) => {
                    assert_eq!(decl.name, expected_name);
                    assert_eq!(decl.visibility, expected_vis);
                }
                _ => panic!("Expected module declaration"),
            }
        }
    }

    #[test]
    fn test_qualified_paths_comprehensive() {
        let test_cases = vec![
            "::std::collections::HashMap", // Global absolute
            "crate::my_module::Type",      // Crate relative
            "super::parent::Item",         // Super path
            "self::current::Thing",        // Self path
            "std::vec::Vec",               // Standard library
            "external_crate::Type",        // External crate
            "local::path::Item",           // Relative path
        ];

        for path in test_cases {
            let input = format!("rel test(x: {}) {{}}", path);
            let result = parse_str(&input);
            assert!(result.is_ok(), "Failed to parse qualified path: {}", path);
        }
    }

    #[test]
    fn test_use_statements_comprehensive() {
        let test_cases = vec![
            // Simple use statements
            ("use std::vec::Vec;", "simple standard library"),
            ("use crate::module::Type;", "simple crate relative"),
            ("use super::parent::Item;", "simple super path"),
            // Glob imports
            ("use std::collections::*;", "glob standard library"),
            ("use crate::types::*;", "glob crate relative"),
            // List imports
            ("use std::collections::{HashMap, HashSet};", "list import"),
            ("use crate::types::{A, B, C};", "list crate types"),
        ];

        for (input, description) in test_cases {
            let result = parse_str(input);
            assert!(
                result.is_ok(),
                "Failed to parse use statement {}: {:?}",
                description,
                result
            );

            let ast = result.unwrap();
            assert_eq!(
                ast.items.len(),
                1,
                "Wrong number of items for {}",
                description
            );

            match &ast.items[0] {
                Item::Use(use_stmt) => match description {
                    desc if desc.contains("simple") => {
                        assert!(matches!(use_stmt.path, ast::UsePath::Simple(_, _)));
                    }
                    desc if desc.contains("glob") => {
                        assert!(matches!(use_stmt.path, ast::UsePath::Glob(_)));
                    }
                    desc if desc.contains("list") => {
                        assert!(matches!(use_stmt.path, ast::UsePath::List(_, _)));
                    }
                    _ => {}
                },
                _ => panic!("Expected use statement for {}", description),
            }
        }
    }

    #[test]
    fn test_nested_modules() {
        let input = r#"
        pub mod outer {
            pub struct OuterType {}
            
            pub mod inner {
                pub rel inner_rel() {}
            }
            
            mod private_inner {
                rel private_rel() {}
            }
        }
        "#;

        let result = parse_str(input);
        assert!(
            result.is_ok(),
            "Failed to parse nested modules: {:?}",
            result
        );

        let ast = result.unwrap();
        assert_eq!(ast.items.len(), 1);

        match &ast.items[0] {
            Item::Module(module) => {
                assert_eq!(module.name, "outer");
                assert_eq!(module.visibility, ast::Visibility::Public);
                assert!(module.items.len() >= 3); // struct + 2 modules
            }
            _ => panic!("Expected module definition"),
        }
    }

    #[test]
    fn test_complex_program_with_all_features() {
        let input = r#"
        use std::collections::HashMap;
        use crate::types::{TypeA, TypeB};
        
        pub mod my_module;
        
        pub struct Config {
            pub name: String,
            pub(crate) internal_id: i32,
        }
        
        pub(crate) rel configure(config: Config) {}
        
        mod utils {
            pub(super) rel helper() {}
        }
        "#;

        let result = parse_str(input);
        assert!(
            result.is_ok(),
            "Failed to parse complex program: {:?}",
            result
        );

        let ast = result.unwrap();
        assert!(ast.items.len() >= 5, "Should have multiple top-level items");

        // Verify we have the expected item types
        let mut use_count = 0;
        let mut mod_decl_count = 0;
        let mut struct_count = 0;
        let mut relation_count = 0;
        let mut mod_def_count = 0;

        for item in &ast.items {
            match item {
                Item::Use(_) => use_count += 1,
                Item::ModuleDeclaration(_) => mod_decl_count += 1,
                Item::Struct(_) => struct_count += 1,
                Item::Predicate(_) => relation_count += 1,
                Item::Module(_) => mod_def_count += 1,
                _ => {}
            }
        }

        assert!(use_count >= 2, "Should have multiple use statements");
        assert!(mod_decl_count >= 1, "Should have module declaration");
        assert!(struct_count >= 1, "Should have struct definition");
        assert!(relation_count >= 1, "Should have relation definition");
        assert!(mod_def_count >= 1, "Should have module definition");
    }

    #[test]
    fn test_field_visibility() {
        let input = r#"
        struct Example {
            pub public_field: String,
            pub(crate) crate_field: i32,
            pub(super) super_field: bool,
            private_field: f64,
        }
        "#;

        let result = parse_str(input);
        assert!(
            result.is_ok(),
            "Failed to parse struct with field visibility: {:?}",
            result
        );

        let ast = result.unwrap();
        assert_eq!(ast.items.len(), 1);

        match &ast.items[0] {
            Item::Struct(s) => match &s.kind {
                ast::StructKind::Named(fields) => {
                    assert_eq!(fields.len(), 4);

                    assert_eq!(fields[0].visibility, ast::Visibility::Public);
                    assert_eq!(fields[0].name, "public_field");

                    assert_eq!(fields[1].visibility, ast::Visibility::Crate);
                    assert_eq!(fields[1].name, "crate_field");

                    assert_eq!(fields[2].visibility, ast::Visibility::Super);
                    assert_eq!(fields[2].name, "super_field");

                    assert_eq!(fields[3].visibility, ast::Visibility::Private);
                    assert_eq!(fields[3].name, "private_field");
                }
                _ => panic!("Expected named struct"),
            },
            _ => panic!("Expected struct definition"),
        }
    }

    #[test]
    fn test_attribute_preservation() {
        let input = r#"
        @test(expected = [["hello"]])
        pub rel test_with_attrs() {
            eq("hello", "hello")
        }
        "#;

        let result = parse_str(input);
        assert!(
            result.is_ok(),
            "Failed to parse relation with attributes: {:?}",
            result
        );

        let ast = result.unwrap();
        assert_eq!(ast.items.len(), 1);

        match &ast.items[0] {
            Item::Predicate(p) => {
                assert_eq!(p.visibility, ast::Visibility::Public);
                assert!(!p.attributes.is_empty(), "Attributes should be preserved");
                assert_eq!(p.name, "test_with_attrs");
            }
            _ => panic!("Expected predicate definition"),
        }
    }
}
