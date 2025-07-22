use pest::iterators::Pair;
use super::{AstBuilder, ParseError, ParseResult, Rule};
use crate::interpreter::parser::{ast::*, meta_parser};

impl<'a> AstBuilder<'a> {
    pub fn build_relation_name(&mut self, pair: Pair<Rule>) -> ParseResult<RelationName> {
        match pair.as_rule() {
            Rule::qualified_path => {
                match self.build_qualified_path(pair.clone()) {
                    Ok(path) => {
                        if let Some(name) = path.final_segment() {
                            // Create the correct module path by preserving the path type but removing the final segment
                            let module_path = match &path {
                                QualifiedPath::Global(_) => {
                                    QualifiedPath::Global(path.module_segments().to_vec())
                                }
                                QualifiedPath::Absolute(_) => {
                                    QualifiedPath::Absolute(path.module_segments().to_vec())
                                }
                                QualifiedPath::Relative(_) => {
                                    QualifiedPath::Relative(path.module_segments().to_vec())
                                }
                                QualifiedPath::Super(levels, _) => {
                                    QualifiedPath::Super(*levels, path.module_segments().to_vec())
                                }
                                QualifiedPath::Self_(_) => {
                                    QualifiedPath::Self_(path.module_segments().to_vec())
                                }

                                QualifiedPath::External(crate_name, _) => {
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
                        Ok(RelationName::Simple(self.create_symbol_from_pair(&pair)))
                    }
                }
            }
            Rule::ident => Ok(RelationName::Simple(self.create_symbol_from_pair(&pair))),
            _ => {
                // Handle the case where we have a nested structure (relation_call might contain qualified_path or ident)
                // Try to find the first inner qualified_path or ident
                let rule = pair.as_rule();
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::qualified_path => return self.build_relation_name(inner),
                        Rule::ident => return Ok(RelationName::Simple(self.create_symbol_from_pair(&inner))),
                        _ => continue,
                    }
                }
                Err(ParseError::UnexpectedRule(rule))
            }
        }
    }

    pub fn build_relation_call(&mut self, pair: Pair<Rule>) -> ParseResult<RelationCall> {
        let mut inner = pair.into_inner();
        let name_pair = inner.next().unwrap();
        let name = self.build_relation_name(name_pair)?;
        let mut args = vec![];
        for arg_pair in inner {
            args.push(self.build_call_argument(arg_pair)?);
        }
        Ok(RelationCall { name, args })
    }

    pub fn build_call_argument(&mut self, pair: Pair<Rule>) -> ParseResult<CallArgument> {
        let span = self.pair_to_span(&pair);

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
                    self.build_term(inner).map(CallArgument::Term)
                }
            }
        } else {
            // Fallback for backward compatibility - try to parse as term first, then meta expression
            match self.build_term(pair.clone()) {
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

    pub fn build_method_call(&mut self, pair: Pair<Rule>) -> ParseResult<MethodCall> {
        let mut inner = pair.into_inner();
        let receiver = Box::new(self.build_term(inner.next().unwrap())?);
        let method_pair = inner.next().unwrap();
        let method = self.create_symbol_from_pair(&method_pair);
        let mut args = vec![];
        for term_pair in inner {
            args.push(self.build_term(term_pair)?);
        }
        Ok(MethodCall {
            receiver,
            method,
            args,
        })
    }
}