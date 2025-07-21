use pest::iterators::Pair;
use super::{AstBuilder, ParseError, ParseResult, Rule};
use crate::interpreter::parser::{ast::*, meta_parser};

impl<'a> AstBuilder<'a> {
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
            Rule::variable => Ok(Term::Variable(pair.as_str().to_string(), span)),
            Rule::wildcard => Ok(Term::Wildcard(span)),
            Rule::list_construction => Ok(Term::List(self.build_list_construction(pair.clone())?, span)),
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
                Ok(Literal::String(s[1..s.len() - 1].to_string()))
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
            Rule::variable => {
                // Extract clean identifier from atomic variable rule to avoid whitespace
                let clean_name = pair.into_inner().next().unwrap().as_str().to_string(); // Get the ident
                Ok(Pattern::Variable(clean_name))
            }
            Rule::wildcard => Ok(Pattern::Wildcard),
            Rule::list_pattern => Ok(Pattern::List(self.build_list_pattern(pair)?)),
            Rule::named_struct_pattern => Ok(Pattern::NamedStruct(self.build_named_struct_pattern(pair)?)),
            Rule::named_variant_pattern => Ok(Pattern::NamedStruct(self.build_named_variant_pattern_as_struct(pair)?)),
            Rule::tuple_struct_pattern_with_parens => {
                Ok(Pattern::TupleStruct(self.build_tuple_struct_pattern_with_parens(pair)?))
            }
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

    pub fn build_named_struct_construction(&mut self, pair: Pair<Rule>) -> ParseResult<NamedStructConstruction> {
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();
        let mut fields = vec![];
        for field_pair in inner {
            fields.push(self.build_field_initializer(field_pair)?);
        }
        Ok(NamedStructConstruction { name, fields })
    }

    pub fn build_named_variant_construction_as_struct(&mut self, pair: Pair<Rule>) -> ParseResult<NamedStructConstruction> {
        let mut inner = pair.into_inner();
        let qualified_path_pair = inner.next().unwrap();
        let qualified_path = self.build_qualified_path(qualified_path_pair)?;
        let name = qualified_path.to_string();
        let mut fields = vec![];
        for field_pair in inner {
            fields.push(self.build_field_initializer(field_pair)?);
        }
        Ok(NamedStructConstruction { name, fields })
    }

    pub fn build_field_initializer(&mut self, pair: Pair<Rule>) -> ParseResult<FieldInitializer> {
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();
        let value = self.build_term(inner.next().unwrap())?;
        Ok(FieldInitializer { name, value })
    }

    pub fn build_tuple_struct_construction(&mut self, pair: Pair<Rule>) -> ParseResult<TupleStructConstruction> {
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();
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

    pub fn build_named_struct_pattern(&mut self, pair: Pair<Rule>) -> ParseResult<NamedStructPattern> {
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();
        let mut fields = vec![];
        for field_pair in inner {
            fields.push(self.build_field_pattern(field_pair)?);
        }
        Ok(NamedStructPattern { name, fields })
    }

    pub fn build_named_variant_pattern_as_struct(&mut self, pair: Pair<Rule>) -> ParseResult<NamedStructPattern> {
        let mut inner = pair.into_inner();
        let qualified_path_pair = inner.next().unwrap();
        let qualified_path = self.build_qualified_path(qualified_path_pair)?;
        let name = qualified_path.to_string();
        let mut fields = vec![];
        for field_pair in inner {
            fields.push(self.build_field_pattern(field_pair)?);
        }
        Ok(NamedStructPattern { name, fields })
    }

    pub fn build_field_pattern(&mut self, pair: Pair<Rule>) -> ParseResult<FieldPattern> {
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();
        let pattern = self.build_pattern(inner.next().unwrap())?;
        Ok(FieldPattern { name, pattern })
    }

    pub fn build_tuple_struct_pattern_with_parens(&mut self, pair: Pair<Rule>) -> ParseResult<TupleStructPattern> {
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str().to_string();
        let mut args = vec![];
        for pattern_pair in inner {
            args.push(self.build_pattern(pattern_pair)?);
        }
        Ok(TupleStructPattern { name, args })
    }

    pub fn build_tuple_struct_pattern_no_parens(&mut self, pair: Pair<Rule>) -> ParseResult<TupleStructPattern> {
        // Extract the qualified path from the inner pair to get clean string without whitespace
        let qualified_path_pair = pair.into_inner().next().unwrap(); // qualified_path
        let qualified_path = self.build_qualified_path(qualified_path_pair)?;
        let name = qualified_path.to_string();

        Ok(TupleStructPattern { name, args: vec![] })
    }
}