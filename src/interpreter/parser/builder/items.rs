use pest::iterators::Pair;
use super::{AstBuilder, ParseError, ParseResult, Rule};
use crate::interpreter::parser::ast::*;

impl<'a> AstBuilder<'a> {
    /// Extract the actual ident pair from a type_name rule, drilling down through the grammar hierarchy
    fn extract_ident_from_type_name(type_name_pair: Pair<Rule>) -> ParseResult<Pair<Rule>> {
        // type_name = { qualified_path | ident }
        let inner = type_name_pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::ident => Ok(inner),
            Rule::qualified_path => {
                // For simple identifiers wrapped in qualified_path, drill down to the ident
                // qualified_path -> relative_path -> simple_segments -> ident
                let path_inner = inner.into_inner().next().unwrap();
                match path_inner.as_rule() {
                    Rule::ident => Ok(path_inner),
                    Rule::relative_path => {
                        let simple_segments = path_inner.into_inner().next().unwrap();
                        let ident = simple_segments.into_inner().next().unwrap();
                        Ok(ident)
                    }
                    _ => Err(ParseError::UnexpectedRule(path_inner.as_rule()))
                }
            }
            _ => Err(ParseError::UnexpectedRule(inner.as_rule()))
        }
    }

    pub fn build_program(&mut self, pair: Pair<Rule>) -> ParseResult<Program> {
        let span = self.pair_to_span(&pair);
        let mut items = vec![];
        for item_pair in pair.into_inner() {
            if let Rule::EOI = item_pair.as_rule() {
                continue;
            }
            items.push(self.build_item(item_pair)?);
        }
        Ok(Program { items, span })
    }

    pub fn build_item(&mut self, pair: Pair<Rule>) -> ParseResult<Item> {
        match pair.as_rule() {
            Rule::use_statement => Ok(Item::Use(self.build_use_statement(pair)?)),
            Rule::mod_declaration => {
                // Handle both simple declarations and body definitions
                let inner = pair.clone().into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::mod_declaration_simple => {
                        Ok(Item::ModuleDeclaration(self.build_mod_declaration(pair)?))
                    }
                    Rule::mod_declaration_body => Ok(Item::Module(self.build_mod_definition(inner)?)),
                    _ => Err(ParseError::UnexpectedRule(inner.as_rule())),
                }
            }
            Rule::struct_definition => Ok(Item::Struct(self.build_struct_definition(pair)?)),
            Rule::enum_definition => Ok(Item::Enum(self.build_enum_definition(pair)?)),
            Rule::impl_block => Ok(Item::Impl(self.build_impl_block(pair)?)),
            Rule::predicate_definition => Ok(Item::Predicate(self.build_predicate_definition(pair)?)),
            _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
        }
    }

    pub fn build_mod_declaration(&mut self, pair: Pair<Rule>) -> ParseResult<ModuleDeclaration> {
        let _span = self.pair_to_span(&pair);
        let inner = pair.into_inner().next().unwrap(); // Get the specific variant

        match inner.as_rule() {
            Rule::mod_declaration_simple => self.build_mod_declaration_simple(inner),
            Rule::mod_declaration_body => self.build_mod_declaration_body(inner),
            _ => Err(ParseError::UnexpectedRule(inner.as_rule())),
        }
    }

    pub fn build_mod_declaration_simple(&mut self, pair: Pair<Rule>) -> ParseResult<ModuleDeclaration> {
        let span = self.pair_to_span(&pair);
        let inner = pair.into_inner();

        // Look at all pairs to determine structure
        let pairs: Vec<_> = inner.clone().collect();

        let (visibility, name) = if !pairs.is_empty() && pairs[0].as_rule() == Rule::visibility {
            // Has visibility: visibility, mod_keyword, ident, ";"
            let vis = self.build_visibility(pairs[0].clone())?;
            let name = self.create_symbol_from_pair(&pairs[2]); // ident comes after visibility and mod_keyword
            (vis, name)
        } else {
            // No visibility: mod_keyword, ident, ";"
            let name = self.create_symbol_from_pair(&pairs[1]); // ident comes after mod_keyword
            (Visibility::Private, name)
        };

        Ok(ModuleDeclaration {
            visibility,
            name,
            span,
        })
    }

    pub fn build_mod_declaration_body(&mut self, pair: Pair<Rule>) -> ParseResult<ModuleDeclaration> {
        let span = self.pair_to_span(&pair);
        let inner = pair.into_inner();

        // Look at all pairs to determine structure
        let pairs: Vec<_> = inner.clone().collect();

        let (visibility, name) = if !pairs.is_empty() && pairs[0].as_rule() == Rule::visibility {
            // Has visibility: visibility, mod_keyword, ident, search_strategy?, "{", body content, "}"
            let vis = self.build_visibility(pairs[0].clone())?;
            let name = self.create_symbol_from_pair(&pairs[2]); // ident comes after visibility and mod_keyword
            (vis, name)
        } else {
            // No visibility: mod_keyword, ident, search_strategy?, "{", body content, "}"
            let name = self.create_symbol_from_pair(&pairs[1]); // ident comes after mod_keyword
            (Visibility::Private, name)
        };

        Ok(ModuleDeclaration {
            visibility,
            name,
            span,
        })
    }

    pub fn build_mod_definition(&mut self, pair: Pair<Rule>) -> ParseResult<ModuleDefinition> {
        let span = self.pair_to_span(&pair);
        let inner = pair.into_inner();

        // Look at all pairs to determine structure
        let pairs: Vec<_> = inner.collect();

        let (visibility, _name_idx) = if !pairs.is_empty() && pairs[0].as_rule() == Rule::visibility {
            (self.build_visibility(pairs[0].clone())?, 1) // visibility, name (we skip "mod" keyword)
        } else {
            (Visibility::Private, 0) // name (we skip "mod" keyword)
        };

        // Find the ident among the pairs
        let name_pair = pairs
            .iter()
            .find(|p| p.as_rule() == Rule::ident)
            .ok_or(ParseError::UnexpectedRule(Rule::ident))?;
        let name = self.create_symbol_from_pair(name_pair);
        let mut search_strategy = None;
        let mut items = vec![];

        for part in pairs.iter() {
            match part.as_rule() {
                Rule::search_strategy => search_strategy = Some(self.build_search_strategy(part.clone())?),
                Rule::use_statement
                | Rule::mod_declaration
                | Rule::struct_definition
                | Rule::impl_block
                | Rule::predicate_definition => items.push(self.build_item(part.clone())?),
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

    pub fn build_struct_definition(&mut self, pair: Pair<Rule>) -> ParseResult<StructDefinition> {
        let span = self.pair_to_span(&pair);
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
            let vis = self.build_visibility(pairs[0].clone())?;
            // Extract the actual ident from the type_name rule
            let type_name_pair = &pairs[2];
            let ident_pair = Self::extract_ident_from_type_name(type_name_pair.clone())?;
            let name = self.create_symbol_from_pair(&ident_pair);
            (vis, name)
        } else {
            // No visibility: struct_keyword, type_name
            // Extract the actual ident from the type_name rule  
            let type_name_pair = &pairs[1];
            let ident_pair = Self::extract_ident_from_type_name(type_name_pair.clone())?;
            let name = self.create_symbol_from_pair(&ident_pair);
            (Visibility::Private, name)
        };
        let kind = match def_pair.as_rule() {
            Rule::tuple_struct_def => {
                let mut types = vec![];
                for type_pair in def_pair.into_inner() {
                    types.push(self.create_symbol_from_pair(&type_pair));
                }
                StructKind::Tuple(types)
            }
            Rule::named_struct_def => {
                let mut fields = vec![];
                for field_pair in def_pair.into_inner() {
                    fields.push(self.build_named_field(field_pair)?);
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

    pub fn build_enum_definition(&mut self, pair: Pair<Rule>) -> ParseResult<EnumDefinition> {
        let span = self.pair_to_span(&pair);
        let inner = pair.into_inner();
        let pairs: Vec<_> = inner.collect();
        
        let (visibility, name) = if !pairs.is_empty() && pairs[0].as_rule() == Rule::visibility {
            // Has visibility: visibility, enum_keyword, type_name, variants
            let vis = self.build_visibility(pairs[0].clone())?;
            // Extract the actual ident from the type_name rule
            let type_name_pair = &pairs[2];
            let ident_pair = Self::extract_ident_from_type_name(type_name_pair.clone())?;
            let name = self.create_symbol_from_pair(&ident_pair);
            (vis, name)
        } else {
            // No visibility: enum_keyword, type_name, variants
            // Extract the actual ident from the type_name rule
            let type_name_pair = &pairs[1];
            let ident_pair = Self::extract_ident_from_type_name(type_name_pair.clone())?;
            let name = self.create_symbol_from_pair(&ident_pair);
            (Visibility::Private, name)
        };
        
        let mut variants = vec![];
        // Skip visibility, enum_keyword, type_name to find variant pairs
        let start_idx = if pairs[0].as_rule() == Rule::visibility { 3 } else { 2 };
        for i in start_idx..pairs.len() {
            if pairs[i].as_rule() == Rule::enum_variant {
                variants.push(self.build_enum_variant(pairs[i].clone())?);
            }
        }
        
        Ok(EnumDefinition {
            visibility,
            name,
            variants,
            span,
        })
    }

    pub fn build_enum_variant(&mut self, pair: Pair<Rule>) -> ParseResult<EnumVariant> {
        let span = self.pair_to_span(&pair);
        let mut inner = pair.into_inner();
        let name_pair = inner.next().unwrap();
        let name = self.create_symbol_from_pair(&name_pair);
        
        let kind = if let Some(variant_def) = inner.next() {
            match variant_def.as_rule() {
                Rule::enum_variant_tuple => {
                    let mut types = vec![];
                    for type_pair in variant_def.into_inner() {
                        types.push(self.create_symbol_from_pair(&type_pair));
                    }
                    VariantKind::Tuple(types)
                }
                Rule::enum_variant_named => {
                    let mut fields = vec![];
                    for field_pair in variant_def.into_inner() {
                        fields.push(self.build_named_field(field_pair)?);
                    }
                    VariantKind::Named(fields)
                }
                _ => return Err(ParseError::UnexpectedRule(variant_def.as_rule())),
            }
        } else {
            VariantKind::Unit
        };
        
        Ok(EnumVariant { name, kind, span })
    }

    pub fn build_named_field(&mut self, pair: Pair<Rule>) -> ParseResult<NamedField> {
        let span = self.pair_to_span(&pair);
        let inner = pair.into_inner();

        // Look at all pairs to determine structure
        let pairs: Vec<_> = inner.collect();

        let (visibility, name, type_name) =
            if !pairs.is_empty() && pairs[0].as_rule() == Rule::visibility {
                // Has visibility: visibility, ident, type_name
                let vis = self.build_visibility(pairs[0].clone())?;
                let name = self.create_symbol_from_pair(&pairs[1]); // ident after visibility
                let type_name = self.build_type_name(pairs[2].clone())?; // type_name after visibility and ident
                (vis, name, type_name)
            } else {
                // No visibility: ident, type_name
                let name = self.create_symbol_from_pair(&pairs[0]); // first element is ident
                let type_name = self.build_type_name(pairs[1].clone())?; // type_name after ident
                (Visibility::Private, name, type_name)
            };
        // name is InternedSymbol, type_name is InternedSymbol

        Ok(NamedField {
            visibility,
            name,
            type_name,
            span,
        })
    }

    pub fn build_impl_block(&mut self, pair: Pair<Rule>) -> ParseResult<ImplBlock> {
        let span = self.pair_to_span(&pair);
        let mut inner = pair.into_inner();
        let type_name_pair = inner.next().unwrap();
        let type_name = self.create_symbol_from_pair(&type_name_pair);
        let mut predicates = vec![];
        for rel_pair in inner {
            if rel_pair.as_rule() == Rule::predicate_definition {
                predicates.push(self.build_predicate_definition(rel_pair)?);
            }
        }
        Ok(ImplBlock {
            type_name,
            predicates,
            span,
        })
    }

    pub fn build_predicate_definition(&mut self, pair: Pair<Rule>) -> ParseResult<PredicateDefinition> {
        let span = self.pair_to_span(&pair);
        let mut inner = pair.into_inner();
        let mut attributes = vec![];
        let mut search_strategy = None;

        // First collect all attributes
        while let Some(p) = inner.peek() {
            if p.as_rule() == Rule::attribute {
                attributes.push(self.build_attribute(inner.next().unwrap())?);
            } else {
                break;
            }
        }

        // Check if next element is visibility
        let next_pair = inner.peek().unwrap();
        let visibility = if next_pair.as_rule() == Rule::visibility {
            self.build_visibility(inner.next().unwrap())?
        } else {
            Visibility::Private
        };

        // Now we must have `relation_keyword`, `ident`, `(params)`, optionally `search_strategy`, and `{body}`
        let relation_kind_pair = inner.next().unwrap();
        let predicate_kind = match relation_kind_pair.as_str() {
            "rel" => PredicateKind::Relation,
            "macro" => PredicateKind::Macro,
            _ => return Err(ParseError::UnexpectedRule(relation_kind_pair.as_rule())),
        };

        let name_pair = inner.next().unwrap();
        let name = self.create_symbol_from_pair(&name_pair);

        let mut parameters = vec![];
        if let Some(p) = inner.peek() {
            if p.as_rule() == Rule::parameter {
                parameters.push(self.build_parameter(inner.next().unwrap())?);
                while let Some(p) = inner.peek() {
                    if p.as_rule() == Rule::parameter {
                        parameters.push(self.build_parameter(inner.next().unwrap())?);
                    } else {
                        break;
                    }
                }
            }
        }

        if let Some(p) = inner.peek() {
            if p.as_rule() == Rule::search_strategy {
                search_strategy = Some(self.build_search_strategy(inner.next().unwrap())?);
            }
        }

        // Remove search strategy from general attributes if it was captured there
        if search_strategy.is_some() {
            attributes.retain(|a| a.name != "bfs" && a.name != "dfs");
        }

        let body = if let Some(p) = inner.peek() {
            if p.as_rule() == Rule::goal_body {
                self.build_goal_body(inner.next().unwrap())?
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
            (PredicateKind::Relation, true) => {
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
}