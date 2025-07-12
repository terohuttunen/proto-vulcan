use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use thiserror::Error;

pub mod ast;
use ast::*;

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
    let mut items = vec![];
    for item_pair in pair.into_inner() {
        if let Rule::EOI = item_pair.as_rule() {
            continue;
        }
        items.push(build_item(item_pair)?);
    }
    Ok(Program { items })
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
    let path_pair = pair.into_inner().next().unwrap();
    let path = build_use_path(path_pair)?;
    Ok(UseStatement { path })
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
    })
}

fn build_struct_definition(pair: Pair<Rule>) -> ParseResult<StructDefinition> {
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

    Ok(StructDefinition { is_pub, name, kind })
}

fn build_named_field(pair: Pair<Rule>) -> ParseResult<NamedField> {
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
    })
}

fn build_impl_block(pair: Pair<Rule>) -> ParseResult<ImplBlock> {
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
    })
}

fn build_relation_definition(pair: Pair<Rule>) -> ParseResult<RelationDefinition> {
    let mut inner = pair.into_inner();
    let mut is_pub = false;

    // Check if first token is pub_keyword
    let first_pair = inner.next().unwrap();
    let name_pair = if first_pair.as_rule() == Rule::pub_keyword {
        is_pub = true;
        inner.next().unwrap() // Skip "rel", get name
    } else {
        first_pair // This should be the name (after "rel" was consumed)
    };

    let name = name_pair.as_str().to_string();

    let mut parameters = vec![];
    let mut search_strategy = None;
    let mut body = None;

    for p in inner {
        match p.as_rule() {
            Rule::parameter => parameters.push(build_parameter(p)?),
            Rule::search_strategy => search_strategy = Some(build_search_strategy(p)?),
            Rule::goal_body => body = Some(build_goal_body(p)?),
            _ => (),
        }
    }

    Ok(RelationDefinition {
        is_pub,
        name,
        parameters,
        search_strategy,
        body: body.unwrap_or_default(),
    })
}

fn build_parameter(pair: Pair<Rule>) -> ParseResult<Parameter> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let type_name = inner.next().map(|p| p.as_str().to_string());
    Ok(Parameter { name, type_name })
}

fn build_search_strategy(pair: Pair<Rule>) -> ParseResult<SearchStrategy> {
    // The search_strategy rule is "@" ~ ("bfs" | "dfs")
    // But Pest combines them into a single token like "@dfs"
    let strategy_str = pair.as_str();
    match strategy_str {
        "@bfs" => Ok(SearchStrategy::Bfs),
        "@dfs" => Ok(SearchStrategy::Dfs),
        _ => {
            // If it's not the combined form, try parsing the inner tokens
            let mut inner = pair.into_inner();
            if let Some(first) = inner.next() {
                if first.as_str() == "@" {
                    if let Some(second) = inner.next() {
                        match second.as_str() {
                            "bfs" => Ok(SearchStrategy::Bfs),
                            "dfs" => Ok(SearchStrategy::Dfs),
                            _ => unreachable!(),
                        }
                    } else {
                        Err(ParseError::MissingRule(Rule::search_strategy))
                    }
                } else {
                    match first.as_str() {
                        "bfs" => Ok(SearchStrategy::Bfs),
                        "dfs" => Ok(SearchStrategy::Dfs),
                        _ => unreachable!(),
                    }
                }
            } else {
                Err(ParseError::MissingRule(Rule::search_strategy))
            }
        }
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
        Rule::let_declaration => Ok(Goal::Let(build_let_declaration(pair)?)),
        Rule::fresh_variables => Ok(Goal::Fresh(build_fresh_variables(pair)?)),
        Rule::disjunction => Ok(Goal::Disjunction(build_disjunction(pair)?)),
        Rule::conjunction => Ok(Goal::Conjunction(build_conjunction(pair)?)),
        Rule::pattern_matching => Ok(Goal::PatternMatch(build_pattern_matching(pair)?)),
        Rule::relation_call => Ok(Goal::RelationCall(build_relation_call(pair)?)),
        Rule::method_call => Ok(Goal::MethodCall(build_method_call(pair)?)),
        Rule::equality_goal => {
            let mut inner = pair.into_inner();
            let lhs = build_term(inner.next().unwrap())?;
            let rhs = build_term(inner.next().unwrap())?;
            Ok(Goal::Equality(lhs, rhs))
        }
        Rule::disequality_goal => {
            let mut inner = pair.into_inner();
            let lhs = build_term(inner.next().unwrap())?;
            let rhs = build_term(inner.next().unwrap())?;
            Ok(Goal::Disequality(lhs, rhs))
        }
        Rule::parenthesized_goal => {
            let body = build_goal_body(pair.into_inner().next().unwrap())?;
            Ok(Goal::Parenthesized(body))
        }
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

fn build_disjunction(pair: Pair<Rule>) -> ParseResult<Disjunction> {
    let body = build_goal_body(pair.into_inner().next().unwrap())?;
    Ok(Disjunction { body })
}

fn build_conjunction(pair: Pair<Rule>) -> ParseResult<Conjunction> {
    let body = build_goal_body(pair.into_inner().next().unwrap())?;
    Ok(Conjunction { body })
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
    let body = build_goal_body(inner.next().unwrap())?;
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
        let inner = pair.into_inner().next().unwrap();
        return build_term(inner);
    }

    match pair.as_rule() {
        Rule::literal => Ok(Term::Literal(build_literal(
            pair.into_inner().next().unwrap(),
        )?)),
        Rule::variable => Ok(Term::Variable(pair.as_str().to_string())),
        Rule::list_construction => {
            if let Some(term_list_pair) = pair.into_inner().next() {
                Ok(Term::List(build_list_construction(term_list_pair)?))
            } else {
                // Empty list
                Ok(Term::List(ListConstruction {
                    elements: vec![],
                    tail: None,
                }))
            }
        }
        Rule::named_struct_construction => {
            Ok(Term::NamedStruct(build_named_struct_construction(pair)?))
        }
        Rule::compound_construction => Ok(Term::Compound(build_compound_construction(pair)?)),
        Rule::parenthesized_term => Ok(Term::Parenthesized(Box::new(build_term(
            pair.into_inner().next().unwrap(),
        )?))),
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
        Rule::compound_pattern => Ok(Pattern::Compound(build_compound_pattern(pair)?)),
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

fn build_compound_pattern(pair: Pair<Rule>) -> ParseResult<CompoundPattern> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut args = vec![];
    for pattern_pair in inner {
        args.push(build_pattern(pattern_pair)?);
    }
    Ok(CompoundPattern { name, args })
}

fn build_list_construction(pair: Pair<Rule>) -> ParseResult<ListConstruction> {
    let mut elements = vec![];
    let mut tail = None;

    let mut inner = pair.into_inner();
    while let Some(p) = inner.next() {
        match p.as_rule() {
            Rule::term => {
                elements.push(build_term(p)?);
            }
            Rule::term_tail => {
                let tail_term_pair = p.into_inner().next().unwrap();
                tail = Some(Box::new(build_term(tail_term_pair)?));
                break; // No more elements after tail
            }
            _ => return Err(ParseError::UnexpectedRule(p.as_rule())),
        }
    }

    Ok(ListConstruction { elements, tail })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_program() {
        let input = "";
        let ast = parse_str(input).unwrap();
        let expected = Program { items: vec![] };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_simple_relation() {
        let input = "rel my_rel(a, b) { a == b }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                name: "my_rel".to_string(),
                parameters: vec![
                    Parameter {
                        name: "a".to_string(),
                        type_name: None,
                    },
                    Parameter {
                        name: "b".to_string(),
                        type_name: None,
                    },
                ],
                search_strategy: None,
                body: vec![Goal::Equality(
                    Term::Variable("a".to_string()),
                    Term::Variable("b".to_string()),
                )],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_pub_relation() {
        let input = "pub rel my_rel(a: T, b: U) @dfs { a == b }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: true,
                name: "my_rel".to_string(),
                parameters: vec![
                    Parameter {
                        name: "a".to_string(),
                        type_name: Some("T".to_string()),
                    },
                    Parameter {
                        name: "b".to_string(),
                        type_name: Some("U".to_string()),
                    },
                ],
                search_strategy: Some(SearchStrategy::Dfs),
                body: vec![Goal::Equality(
                    Term::Variable("a".to_string()),
                    Term::Variable("b".to_string()),
                )],
            })],
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
            })],
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
            })],
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
            })],
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
            })],
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
                    },
                    NamedField {
                        is_pub: false,
                        name: "other".to_string(),
                        type_name: "U".to_string(),
                    },
                ]),
            })],
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
                relations: vec![RelationDefinition {
                    is_pub: false,
                    name: "new".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "x".to_string(),
                            type_name: None,
                        },
                        Parameter {
                            name: "y".to_string(),
                            type_name: None,
                        },
                        Parameter {
                            name: "p".to_string(),
                            type_name: None,
                        },
                    ],
                    search_strategy: None,
                    body: vec![Goal::Equality(
                        Term::Variable("p".to_string()),
                        Term::NamedStruct(NamedStructConstruction {
                            name: "Point".to_string(),
                            fields: vec![
                                FieldInitializer {
                                    name: "x".to_string(),
                                    value: Term::Variable("x".to_string()),
                                },
                                FieldInitializer {
                                    name: "y".to_string(),
                                    value: Term::Variable("y".to_string()),
                                },
                            ],
                        }),
                    )],
                }],
            })],
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
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![
                    Goal::Equality(
                        Term::Variable("a".to_string()),
                        Term::Literal(Literal::Boolean(true)),
                    ),
                    Goal::Equality(
                        Term::Variable("b".to_string()),
                        Term::Literal(Literal::Boolean(false)),
                    ),
                    Goal::Equality(
                        Term::Variable("c".to_string()),
                        Term::Literal(Literal::Number("42".to_string())),
                    ),
                    Goal::Equality(
                        Term::Variable("d".to_string()),
                        Term::Literal(Literal::Number("-17".to_string())),
                    ),
                    Goal::Equality(
                        Term::Variable("e".to_string()),
                        Term::Literal(Literal::String("hello".to_string())),
                    ),
                    Goal::Equality(
                        Term::Variable("f".to_string()),
                        Term::Literal(Literal::Char('x')),
                    ),
                ],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_list_construction() {
        let input = "rel test() { a == [1, 2, 3] }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Equality(
                    Term::Variable("a".to_string()),
                    Term::List(ListConstruction {
                        elements: vec![
                            Term::Literal(Literal::Number("1".to_string())),
                            Term::Literal(Literal::Number("2".to_string())),
                            Term::Literal(Literal::Number("3".to_string())),
                        ],
                        tail: None,
                    }),
                )],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_compound_construction() {
        let input = "rel test() { a == Some(42) }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Equality(
                    Term::Variable("a".to_string()),
                    Term::Compound(CompoundConstruction {
                        name: "Some".to_string(),
                        args: vec![Term::Literal(Literal::Number("42".to_string()))],
                    }),
                )],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_disjunction() {
        let input = r#"rel test() { 
            conde {
                a == 1,
                a == 2
            }
        }"#;
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Disjunction(Disjunction {
                    body: vec![
                        Goal::Equality(
                            Term::Variable("a".to_string()),
                            Term::Literal(Literal::Number("1".to_string())),
                        ),
                        Goal::Equality(
                            Term::Variable("a".to_string()),
                            Term::Literal(Literal::Number("2".to_string())),
                        ),
                    ],
                })],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_conjunction() {
        let input = "rel test() { [a == 1, b == 2] }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Conjunction(Conjunction {
                    body: vec![
                        Goal::Equality(
                            Term::Variable("a".to_string()),
                            Term::Literal(Literal::Number("1".to_string())),
                        ),
                        Goal::Equality(
                            Term::Variable("b".to_string()),
                            Term::Literal(Literal::Number("2".to_string())),
                        ),
                    ],
                })],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_fresh_variables() {
        let input = "rel test() { |x, y| { x == y } }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Fresh(FreshVariables {
                    vars: vec!["x".to_string(), "y".to_string()],
                    body: vec![Goal::Equality(
                        Term::Variable("x".to_string()),
                        Term::Variable("y".to_string()),
                    )],
                })],
            })],
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
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![Parameter {
                    name: "l".to_string(),
                    type_name: None,
                }],
                search_strategy: None,
                body: vec![Goal::PatternMatch(PatternMatching {
                    term: Term::Variable("l".to_string()),
                    arms: vec![
                        PatternArm {
                            pattern: Pattern::List(ListPattern {
                                elements: vec![],
                                tail: None,
                            }),
                            body: vec![Goal::RelationCall(RelationCall {
                                name: "succeed".to_string(),
                                args: vec![],
                            })],
                        },
                        PatternArm {
                            pattern: Pattern::List(ListPattern {
                                elements: vec![Pattern::Variable("a".to_string())],
                                tail: None,
                            }),
                            body: vec![Goal::Equality(
                                Term::Variable("a".to_string()),
                                Term::Literal(Literal::Number("1".to_string())),
                            )],
                        },
                        PatternArm {
                            pattern: Pattern::Wildcard,
                            body: vec![Goal::RelationCall(RelationCall {
                                name: "fail".to_string(),
                                args: vec![],
                            })],
                        },
                    ],
                })],
            })],
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
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![Parameter {
                    name: "l".to_string(),
                    type_name: None,
                }],
                search_strategy: None,
                body: vec![Goal::PatternMatch(PatternMatching {
                    term: Term::Variable("l".to_string()),
                    arms: vec![PatternArm {
                        pattern: Pattern::List(ListPattern {
                            elements: vec![
                                Pattern::Variable("a".to_string()),
                                Pattern::Variable("b".to_string()),
                            ],
                            tail: Some(Box::new(Pattern::Variable("t".to_string()))),
                        }),
                        body: vec![Goal::Equality(
                            Term::Variable("a".to_string()),
                            Term::Variable("b".to_string()),
                        )],
                    }],
                })],
            })],
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
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![
                    Goal::Let(LetDeclaration {
                        var_name: "x".to_string(),
                        value: Some(Term::Literal(Literal::Number("42".to_string()))),
                    }),
                    Goal::Let(LetDeclaration {
                        var_name: "y".to_string(),
                        value: None,
                    }),
                    Goal::Equality(
                        Term::Variable("x".to_string()),
                        Term::Variable("y".to_string()),
                    ),
                ],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_method_call() {
        let input = "rel test() { x.method(a, b) }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::MethodCall(MethodCall {
                    receiver: Box::new(Term::Variable("x".to_string())),
                    method: "method".to_string(),
                    args: vec![
                        Term::Variable("a".to_string()),
                        Term::Variable("b".to_string()),
                    ],
                })],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_relation_call() {
        let input = "rel test() { my_relation(a, b, c) }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::RelationCall(RelationCall {
                    name: "my_relation".to_string(),
                    args: vec![
                        Term::Variable("a".to_string()),
                        Term::Variable("b".to_string()),
                        Term::Variable("c".to_string()),
                    ],
                })],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_relation_call_no_args() {
        let input = "rel test() { succeed() }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::RelationCall(RelationCall {
                    name: "succeed".to_string(),
                    args: vec![],
                })],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_disequality() {
        let input = "rel test() { a != b }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Disequality(
                    Term::Variable("a".to_string()),
                    Term::Variable("b".to_string()),
                )],
            })],
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
                    }),
                    Item::Struct(StructDefinition {
                        is_pub: false,
                        name: "Point".to_string(),
                        kind: StructKind::Named(vec![
                            NamedField {
                                is_pub: false,
                                name: "x".to_string(),
                                type_name: "i32".to_string(),
                            },
                            NamedField {
                                is_pub: false,
                                name: "y".to_string(),
                                type_name: "i32".to_string(),
                            },
                        ]),
                    }),
                    Item::Relation(RelationDefinition {
                        is_pub: false,
                        name: "test".to_string(),
                        parameters: vec![],
                        search_strategy: None,
                        body: vec![Goal::RelationCall(RelationCall {
                            name: "succeed".to_string(),
                            args: vec![],
                        })],
                    }),
                ],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_parenthesized_goal() {
        let input = "rel test() { (a == b, c == d) }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Parenthesized(vec![
                    Goal::Equality(
                        Term::Variable("a".to_string()),
                        Term::Variable("b".to_string()),
                    ),
                    Goal::Equality(
                        Term::Variable("c".to_string()),
                        Term::Variable("d".to_string()),
                    ),
                ])],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_parenthesized_term() {
        let input = "rel test() { a == (b) }";
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![Goal::Equality(
                    Term::Variable("a".to_string()),
                    Term::Parenthesized(Box::new(Term::Variable("b".to_string()))),
                )],
            })],
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
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![Parameter {
                    name: "p".to_string(),
                    type_name: None,
                }],
                search_strategy: None,
                body: vec![Goal::PatternMatch(PatternMatching {
                    term: Term::Variable("p".to_string()),
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
                            Term::Variable("a".to_string()),
                            Term::Variable("b".to_string()),
                        )],
                    }],
                })],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_compound_pattern() {
        let input = r#"rel test(x) {
            match x {
                Some(a) => { a == 42 }
            }
        }"#;
        let ast = parse_str(input).unwrap();
        let expected = Program {
            items: vec![Item::Relation(RelationDefinition {
                is_pub: false,
                name: "test".to_string(),
                parameters: vec![Parameter {
                    name: "x".to_string(),
                    type_name: None,
                }],
                search_strategy: None,
                body: vec![Goal::PatternMatch(PatternMatching {
                    term: Term::Variable("x".to_string()),
                    arms: vec![PatternArm {
                        pattern: Pattern::Compound(CompoundPattern {
                            name: "Some".to_string(),
                            args: vec![Pattern::Variable("a".to_string())],
                        }),
                        body: vec![Goal::Equality(
                            Term::Variable("a".to_string()),
                            Term::Literal(Literal::Number("42".to_string())),
                        )],
                    }],
                })],
            })],
        };
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_error_handling() {
        let input = "invalid syntax here";
        let result = parse_str(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_complex_example() {
        let input = r#"
        use std::collections::HashMap;
        
        pub struct Point { 
            pub x: i32, 
            pub y: i32 
        }
        
        impl Point {
            pub rel new(x, y, p) @dfs {
                p == Point { x: x, y: y }
            }
        }
        
        pub rel complex_goal(a, b, c) {
            conde {
                [a == 1, b == 2],
                |x, y| {
                    a == x,
                    b == y,
                    c == [x, y]
                }
            }
        }
        "#;
        let ast = parse_str(input).unwrap();

        // Just verify that it parses successfully and has the right structure
        assert_eq!(ast.items.len(), 4);

        // Check use statement
        if let Item::Use(use_stmt) = &ast.items[0] {
            assert_eq!(
                use_stmt.path,
                UsePath::Simple(vec![
                    "std".to_string(),
                    "collections".to_string(),
                    "HashMap".to_string(),
                ])
            );
        } else {
            panic!("Expected use statement");
        }

        // Check struct definition
        if let Item::Struct(struct_def) = &ast.items[1] {
            assert_eq!(struct_def.name, "Point");
            assert!(struct_def.is_pub);
        } else {
            panic!("Expected struct definition");
        }

        // Check impl block
        if let Item::Impl(impl_block) = &ast.items[2] {
            assert_eq!(impl_block.type_name, "Point");
            assert_eq!(impl_block.relations.len(), 1);
        } else {
            panic!("Expected impl block");
        }

        // Check relation definition
        if let Item::Relation(relation_def) = &ast.items[3] {
            assert_eq!(relation_def.name, "complex_goal");
            assert!(relation_def.is_pub);
        } else {
            panic!("Expected relation definition");
        }
    }
}
