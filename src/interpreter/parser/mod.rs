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
        Rule::tuple_struct_construction => Ok(Term::TupleStruct(
            build_tuple_struct_construction(pair.clone())?,
            span,
        )),
        Rule::tuple_struct_construction_no_parens => {
            // Extract the qualified path from the inner pair to get clean string without whitespace
            let qualified_path_pair = pair.into_inner().next().unwrap(); // qualified_path
            let qualified_path = build_qualified_path(qualified_path_pair)?;
            let name = qualified_path.to_string();

            // Semantic disambiguation: if this is a simple identifier (no ::) with no args,
            // treat it as a variable instead of a compound construction
            if !name.contains("::") {
                Ok(Term::Variable(name, span))
            } else {
                Ok(Term::TupleStruct(
                    TupleStructConstruction { name, args: vec![] },
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

fn build_tuple_struct_construction(pair: Pair<Rule>) -> ParseResult<TupleStructConstruction> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut args = vec![];
    for term_pair in inner {
        args.push(build_term(term_pair)?);
    }
    Ok(TupleStructConstruction { name, args })
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
        Rule::tuple_struct_pattern_with_parens => {
            Ok(Pattern::TupleStruct(build_tuple_struct_pattern_with_parens(pair)?))
        }
        Rule::tuple_struct_pattern_no_parens => {
            let compound = build_tuple_struct_pattern_no_parens(pair)?;

            // Semantic disambiguation: if this is a simple identifier (no ::) with no args,
            // treat it as a variable instead of a compound pattern
            if !compound.name.contains("::") && compound.args.is_empty() {
                Ok(Pattern::Variable(compound.name))
            } else {
                Ok(Pattern::TupleStruct(compound))
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

fn build_tuple_struct_pattern_with_parens(pair: Pair<Rule>) -> ParseResult<TupleStructPattern> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut args = vec![];
    for pattern_pair in inner {
        args.push(build_pattern(pattern_pair)?);
    }
    Ok(TupleStructPattern { name, args })
}

fn build_tuple_struct_pattern_no_parens(pair: Pair<Rule>) -> ParseResult<TupleStructPattern> {
    // Extract the qualified path from the inner pair to get clean string without whitespace
    let qualified_path_pair = pair.into_inner().next().unwrap(); // qualified_path
    let qualified_path = build_qualified_path(qualified_path_pair)?;
    let name = qualified_path.to_string();

    Ok(TupleStructPattern { name, args: vec![] })
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
mod tests;

