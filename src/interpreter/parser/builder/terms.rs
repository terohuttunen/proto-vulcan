use super::{AstBuilder, ParseError, ParseResult, Rule};
use crate::interpreter::parser::{ast::*, meta_parser};
use pest::iterators::Pair;

impl<'a> AstBuilder<'a> {
    /// Process escape sequences in a string literal
    fn process_escape_sequences(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars();
        
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(escaped) = chars.next() {
                    match escaped {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '\\' => result.push('\\'),
                        '"' => result.push('"'),
                        '\'' => result.push('\''),
                        '0' => result.push('\0'),
                        _ => {
                            // Unknown escape sequence - preserve literally
                            result.push('\\');
                            result.push(escaped);
                        }
                    }
                } else {
                    // Trailing backslash - preserve literally
                    result.push('\\');
                }
            } else {
                result.push(ch);
            }
        }
        
        result
    }
    pub fn build_term(&mut self, pair: Pair<Rule>) -> ParseResult<Term> {
        if pair.as_rule() == Rule::term {
            // If we get a generic term, we need to extract the specific term type
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| ParseError::MissingRule(Rule::term))?;
            return self.build_term(inner);
        }

        let span = self.pair_to_span(&pair);
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
                Ok(Term::Literal(self.build_literal(lit_pair)?, span))
            }
            Rule::variable => Ok(Term::Variable(self.create_symbol_from_pair(&pair))),
            Rule::wildcard => Ok(Term::Wildcard(span)),
            Rule::list_construction => Ok(Term::List(
                self.build_list_construction(pair.clone())?,
                span,
            )),
            Rule::named_struct_construction => Ok(Term::NamedStruct(
                self.build_named_struct_construction(pair.clone())?,
                span,
            )),
            Rule::named_variant_construction => Ok(Term::NamedStruct(
                self.build_named_variant_construction_as_struct(pair.clone())?,
                span,
            )),
            Rule::tuple_struct_construction => Ok(Term::TupleStruct(
                self.build_tuple_struct_construction(pair.clone())?,
                span,
            )),
            Rule::tuple_struct_construction_no_parens => {
                // Extract the qualified path from the inner pair to get clean string without whitespace
                let qualified_path_pair = pair.into_inner().next().unwrap(); // qualified_path
                let qualified_path = self.build_qualified_path(qualified_path_pair)?;

                // Semantic disambiguation: if this is a simple identifier (no ::) with no args,
                // treat it as a variable instead of a compound construction
                if let QualifiedPath::Relative(segments) = &qualified_path {
                    if segments.len() == 1 {
                        // Simple identifier - treat as variable
                        Ok(Term::Variable(segments[0].clone()))
                    } else {
                        // Multi-segment qualified path - treat as tuple struct construction
                        Ok(Term::TupleStruct(
                            TupleStructConstruction {
                                name: qualified_path,
                                args: vec![],
                            },
                            span,
                        ))
                    }
                } else {
                    // Non-relative qualified paths are always tuple struct constructions
                    Ok(Term::TupleStruct(
                        TupleStructConstruction {
                            name: qualified_path,
                            args: vec![],
                        },
                        span,
                    ))
                }
            }
            Rule::path_term => {
                // Extract the qualified path from the inner pair to get clean string without whitespace
                let qualified_path_pair = pair.into_inner().next().unwrap(); // qualified_path
                let qualified_path = self.build_qualified_path(qualified_path_pair)?;

                // Special case: standalone 'self' should be treated as a variable in method contexts
                if let QualifiedPath::Self_(segments) = &qualified_path {
                    if segments.is_empty() {
                        // Standalone 'self' - treat as variable
                        return Ok(Term::Variable(
                            crate::interpreter::symbol_table::InternedSymbol::from_text("self")
                        ));
                    }
                }

                // Semantic disambiguation: if this is a simple identifier (no ::) with no args,
                // treat it as a variable instead of a compound construction
                if let QualifiedPath::Relative(segments) = &qualified_path {
                    if segments.len() == 1 {
                        // Simple identifier - treat as variable
                        Ok(Term::Variable(segments[0].clone()))
                    } else {
                        // Multi-segment qualified path - could be enum variant, treat as tuple struct construction
                        Ok(Term::TupleStruct(
                            TupleStructConstruction {
                                name: qualified_path,
                                args: vec![],
                            },
                            span,
                        ))
                    }
                } else {
                    // Non-relative qualified paths are always tuple struct constructions
                    Ok(Term::TupleStruct(
                        TupleStructConstruction {
                            name: qualified_path,
                            args: vec![],
                        },
                        span,
                    ))
                }
            }
            Rule::parenthesized_term => Ok(Term::Parenthesized(
                Box::new(self.build_term(pair.into_inner().next().unwrap())?),
                span,
            )),
            _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
        }
    }

    pub fn build_literal(&mut self, pair: Pair<Rule>) -> ParseResult<Literal> {
        if pair.as_rule() == Rule::literal {
            // If we get a generic literal, we need to extract the specific literal type
            let inner = pair.into_inner().next().unwrap();
            return self.build_literal(inner);
        }

        match pair.as_rule() {
            Rule::boolean_literal => Ok(Literal::Boolean(pair.as_str().parse().unwrap())),
            Rule::number_literal => Ok(Literal::Number(pair.as_str().to_string())),
            Rule::string_literal => {
                let s = pair.as_str();
                let unquoted = &s[1..s.len() - 1];
                let processed = self.process_escape_sequences(unquoted);
                Ok(Literal::String(processed))
            }
            Rule::char_literal => {
                let s = pair.as_str();
                Ok(Literal::Char(s[1..s.len() - 1].chars().next().unwrap()))
            }
            _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
        }
    }

    pub fn build_pattern(&mut self, pair: Pair<Rule>) -> ParseResult<Pattern> {
        if pair.as_rule() == Rule::pattern {
            // If we get a generic pattern, we need to extract the specific pattern type
            let inner = pair.into_inner().next().unwrap();
            return self.build_pattern(inner);
        }

        match pair.as_rule() {
            Rule::literal => Ok(Pattern::Literal(self.build_literal(pair)?)),
            Rule::variable => Ok(Pattern::Variable(self.create_symbol_from_pair(&pair))),
            Rule::wildcard => Ok(Pattern::Wildcard),
            Rule::list_pattern => Ok(Pattern::List(self.build_list_pattern(pair)?)),
            Rule::named_struct_pattern => {
                Ok(Pattern::NamedStruct(self.build_named_struct_pattern(pair)?))
            }
            Rule::named_variant_pattern => Ok(Pattern::NamedStruct(
                self.build_named_variant_pattern_as_struct(pair)?,
            )),
            Rule::tuple_struct_pattern_with_parens => Ok(Pattern::TupleStruct(
                self.build_tuple_struct_pattern_with_parens(pair)?,
            )),
            Rule::tuple_struct_pattern_no_parens => {
                let compound = self.build_tuple_struct_pattern_no_parens(pair)?;

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

    pub fn build_named_struct_construction(
        &mut self,
        pair: Pair<Rule>,
    ) -> ParseResult<NamedStructConstruction> {
        let mut inner = pair.into_inner();
        let name_pair = inner.next().unwrap();
        let name = self.create_symbol_from_pair(&name_pair);
        let mut fields = vec![];
        for field_pair in inner {
            fields.push(self.build_field_initializer(field_pair)?);
        }
        Ok(NamedStructConstruction { name, fields })
    }

    pub fn build_named_variant_construction_as_struct(
        &mut self,
        pair: Pair<Rule>,
    ) -> ParseResult<NamedStructConstruction> {
        let mut inner = pair.into_inner();
        let qualified_path_pair = inner.next().unwrap();

        // Use build_qualified_path to get clean symbols without whitespace issues
        let name =
            if let Ok(qualified_path) = self.build_qualified_path(qualified_path_pair.clone()) {
                match qualified_path {
                    QualifiedPath::Relative(segments) if segments.len() == 1 => segments[0].clone(),
                    _ => self.create_symbol_from_pair(&qualified_path_pair),
                }
            } else {
                self.create_symbol_from_pair(&qualified_path_pair)
            };

        let mut fields = vec![];
        for field_pair in inner {
            fields.push(self.build_field_initializer(field_pair)?);
        }
        Ok(NamedStructConstruction { name, fields })
    }

    pub fn build_field_initializer(&mut self, pair: Pair<Rule>) -> ParseResult<FieldInitializer> {
        let mut inner = pair.into_inner();
        let name_pair = inner.next().unwrap();
        let name = self.create_symbol_from_pair(&name_pair);
        let value = self.build_term(inner.next().unwrap())?;
        Ok(FieldInitializer { name, value })
    }

    pub fn build_tuple_struct_construction(
        &mut self,
        pair: Pair<Rule>,
    ) -> ParseResult<TupleStructConstruction> {
        let mut inner = pair.into_inner();
        let name_pair = inner.next().unwrap();
        let name = if name_pair.as_rule() == Rule::qualified_path {
            self.build_qualified_path(name_pair)?
        } else {
            // Simple identifier - create a relative qualified path with one segment
            let symbol = self.create_symbol_from_pair(&name_pair);
            QualifiedPath::Relative(vec![symbol])
        };
        let mut args = vec![];
        for term_pair in inner {
            args.push(self.build_term(term_pair)?);
        }
        Ok(TupleStructConstruction { name, args })
    }

    pub fn build_list_construction(&mut self, pair: Pair<Rule>) -> ParseResult<ListConstruction> {
        let mut elements = vec![];
        let mut tail = None;

        if let Some(term_list_pair) = pair.into_inner().next() {
            let mut inner = term_list_pair.into_inner();
            while let Some(part) = inner.next() {
                match part.as_rule() {
                    Rule::term => {
                        elements.push(self.build_term(part)?);
                    }
                    Rule::term_tail => {
                        if let Some(tail_term) = part.into_inner().next() {
                            tail = Some(Box::new(self.build_term(tail_term)?));
                        }
                    }
                    _ => return Err(ParseError::UnexpectedRule(part.as_rule())),
                }
            }
        }

        Ok(ListConstruction { elements, tail })
    }

    pub fn build_list_pattern(&mut self, pair: Pair<Rule>) -> ParseResult<ListPattern> {
        let mut elements = vec![];
        let mut tail = None;

        if let Some(pattern_list_pair) = pair.into_inner().next() {
            let mut inner = pattern_list_pair.into_inner();
            while let Some(p) = inner.next() {
                match p.as_rule() {
                    Rule::pattern => {
                        elements.push(self.build_pattern(p)?);
                    }
                    Rule::list_tail => {
                        let tail_pattern_pair = p.into_inner().next().unwrap();
                        tail = Some(Box::new(self.build_pattern(tail_pattern_pair)?));
                        break; // No more elements after tail
                    }
                    _ => return Err(ParseError::UnexpectedRule(p.as_rule())),
                }
            }
        }

        Ok(ListPattern { elements, tail })
    }

    pub fn build_named_struct_pattern(
        &mut self,
        pair: Pair<Rule>,
    ) -> ParseResult<NamedStructPattern> {
        let mut inner = pair.into_inner();
        let name_pair = inner.next().unwrap();
        let name = self.create_symbol_from_pair(&name_pair);
        let mut fields = vec![];
        for field_pair in inner {
            fields.push(self.build_field_pattern(field_pair)?);
        }
        Ok(NamedStructPattern { name, fields })
    }

    pub fn build_named_variant_pattern_as_struct(
        &mut self,
        pair: Pair<Rule>,
    ) -> ParseResult<NamedStructPattern> {
        let mut inner = pair.into_inner();
        let qualified_path_pair = inner.next().unwrap();

        // Use build_qualified_path to get clean symbols without whitespace issues
        let name =
            if let Ok(qualified_path) = self.build_qualified_path(qualified_path_pair.clone()) {
                match qualified_path {
                    QualifiedPath::Relative(segments) if segments.len() == 1 => segments[0].clone(),
                    _ => self.create_symbol_from_pair(&qualified_path_pair),
                }
            } else {
                self.create_symbol_from_pair(&qualified_path_pair)
            };

        let mut fields = vec![];
        for field_pair in inner {
            fields.push(self.build_field_pattern(field_pair)?);
        }
        Ok(NamedStructPattern { name, fields })
    }

    pub fn build_field_pattern(&mut self, pair: Pair<Rule>) -> ParseResult<FieldPattern> {
        let mut inner = pair.into_inner();
        let name_pair = inner.next().unwrap();

        let name = self.create_symbol_from_pair(&name_pair);

        let pattern_pair = inner.next().unwrap();
        let pattern = self.build_pattern(pattern_pair)?;
        Ok(FieldPattern { name, pattern })
    }

    pub fn build_tuple_struct_pattern_with_parens(
        &mut self,
        pair: Pair<Rule>,
    ) -> ParseResult<TupleStructPattern> {
        let mut inner = pair.into_inner();
        let name_pair = inner.next().unwrap();
        let name = self.create_symbol_from_pair(&name_pair);
        let mut args = vec![];
        for pattern_pair in inner {
            args.push(self.build_pattern(pattern_pair)?);
        }
        Ok(TupleStructPattern { name, args })
    }

    pub fn build_tuple_struct_pattern_no_parens(
        &mut self,
        pair: Pair<Rule>,
    ) -> ParseResult<TupleStructPattern> {
        // Extract the qualified path from the inner pair to get clean string without whitespace
        let qualified_path_pair = pair.into_inner().next().unwrap(); // qualified_path

        // For qualified paths that are simple identifiers, drill down to the innermost ident
        // to avoid whitespace issues as user indicated all whitespace issues fixed with into_inner
        let name =
            if let Ok(qualified_path) = self.build_qualified_path(qualified_path_pair.clone()) {
                match qualified_path {
                    QualifiedPath::Relative(segments) if segments.len() == 1 => {
                        // Simple identifier case - use the already properly parsed symbol
                        segments[0].clone()
                    }
                    _ => {
                        // Complex qualified path - build it normally
                        self.create_symbol_from_pair(&qualified_path_pair)
                    }
                }
            } else {
                // Fallback to original logic
                self.create_symbol_from_pair(&qualified_path_pair)
            };

        Ok(TupleStructPattern { name, args: vec![] })
    }
}
