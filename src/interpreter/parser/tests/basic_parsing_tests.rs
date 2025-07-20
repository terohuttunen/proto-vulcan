//! Basic parsing functionality tests
//!
//! Tests for core parsing functionality including programs, relations, 
//! structs, goals, terms, patterns, and other fundamental language constructs.

use super::super::*;

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
fn test_parse_tuple_struct_construction() {
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
                Term::TupleStruct(
                    TupleStructConstruction {
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

    let expected_pattern = Pattern::TupleStruct(TupleStructPattern {
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

    let expected_pattern = Pattern::TupleStruct(TupleStructPattern {
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

    let expected_pattern = Pattern::TupleStruct(TupleStructPattern {
        name: "Option::None".to_string(),
        args: vec![],
    });

    assert_eq!(pattern_matching.arms[0].pattern, expected_pattern);
}

#[test]
fn test_parse_tuple_struct_construction_qualified() {
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

    let expected_term = Term::TupleStruct(
        TupleStructConstruction {
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
fn test_parse_tuple_struct_construction_no_parens() {
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

    let expected_term = Term::TupleStruct(
        TupleStructConstruction {
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

    for (_i, item) in items.enumerate() {
        if item.starts_with("mod ") && item.ends_with(";") {
            let _result = VulcanParser::parse(Rule::mod_declaration, item);
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