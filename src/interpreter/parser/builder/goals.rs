use pest::iterators::Pair;
use super::{AstBuilder, ParseError, ParseResult, Rule};
use crate::interpreter::parser::ast::*;

impl<'a> AstBuilder<'a> {
    pub fn build_goal_body(&mut self, pair: Pair<Rule>) -> ParseResult<GoalBody> {
        let mut body = vec![];
        for goal_pair in pair.into_inner() {
            body.push(self.build_goal(goal_pair)?);
        }
        Ok(body)
    }

    pub fn build_goal(&mut self, pair: Pair<Rule>) -> ParseResult<Goal> {
        if pair.as_rule() == Rule::goal {
            // If we get a generic goal, we need to extract the specific goal type
            let inner = pair.into_inner().next().unwrap();
            return self.build_goal(inner);
        }

        match pair.as_rule() {
            Rule::meta_statement => Ok(Goal::MetaStatement(
                self.build_meta_statement(pair.clone())?,
                self.pair_to_span(&pair),
            )),
            Rule::let_declaration => Ok(Goal::Let(
                self.build_let_declaration(pair.clone())?,
                self.pair_to_span(&pair),
            )),
            Rule::fresh_variables => Ok(Goal::Fresh(
                self.build_fresh_variables(pair.clone())?,
                self.pair_to_span(&pair),
            )),
            Rule::any_block => self.build_any_block(pair),
            Rule::all_block => self.build_all_block(pair),
            Rule::constraint_block => self.build_constraint_block(pair),
            Rule::pattern_matching => Ok(Goal::PatternMatch(
                self.build_pattern_matching(pair.clone())?,
                self.pair_to_span(&pair),
            )),
            Rule::relation_call => Ok(Goal::RelationCall(
                self.build_relation_call(pair.clone())?,
                self.pair_to_span(&pair),
            )),
            Rule::method_call => Ok(Goal::MethodCall(
                self.build_method_call(pair.clone())?,
                self.pair_to_span(&pair),
            )),
            Rule::equality_goal => {
                let span = self.pair_to_span(&pair);
                let mut inner = pair.into_inner();
                let lhs = self.build_term(inner.next().unwrap())?;
                let _equality_op = inner.next().unwrap(); // Skip the atomic equality_op
                let rhs = self.build_term(inner.next().unwrap())?;
                Ok(Goal::Equality(lhs, rhs, span))
            }
            Rule::disequality_goal => {
                let span = self.pair_to_span(&pair);
                let mut inner = pair.into_inner();
                let lhs = self.build_term(inner.next().unwrap())?;
                let _inequality_op = inner.next().unwrap(); // Skip the atomic inequality_op
                let rhs = self.build_term(inner.next().unwrap())?;
                Ok(Goal::Disequality(lhs, rhs, span))
            }
            Rule::parenthesized_goal => {
                let span = self.pair_to_span(&pair);
                let body = self.build_goal_body(pair.into_inner().next().unwrap())?;
                Ok(Goal::Parenthesized(body, span))
            }
            Rule::literal => {
                let span = self.pair_to_span(&pair);
                let literal = self.build_literal(pair)?;
                match literal {
                    Literal::Boolean(b) => Ok(Goal::BooleanLiteral(b, span)),
                    _ => Err(ParseError::UnexpectedRule(Rule::literal)),
                }
            }
            _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
        }
    }

    pub fn build_any_block(&mut self, pair: Pair<Rule>) -> ParseResult<Goal> {
        let span = self.pair_to_span(&pair);
        let mut inner = pair.into_inner();
        let mut params = None;
        let mut body = vec![];

        while let Some(part) = inner.next() {
            match part.as_rule() {
                Rule::search_params => {
                    params = Some(self.build_search_params(part)?);
                }
                Rule::goal_body => {
                    body = self.build_goal_body(part)?;
                }
                _ => return Err(ParseError::UnexpectedRule(part.as_rule())),
            }
        }

        Ok(Goal::Disjunction(Disjunction { body, params }, span))
    }

    pub fn build_all_block(&mut self, pair: Pair<Rule>) -> ParseResult<Goal> {
        let span = self.pair_to_span(&pair);
        let mut inner = pair.into_inner();
        let mut params = None;
        let mut body = vec![];

        while let Some(part) = inner.next() {
            match part.as_rule() {
                Rule::search_params => {
                    params = Some(self.build_search_params(part)?);
                }
                Rule::goal_body => {
                    body = self.build_goal_body(part)?;
                }
                _ => return Err(ParseError::UnexpectedRule(part.as_rule())),
            }
        }

        Ok(Goal::Conjunction(Conjunction { body, params }, span))
    }

    pub fn build_constraint_block(&mut self, pair: Pair<Rule>) -> ParseResult<Goal> {
        let span = self.pair_to_span(&pair);
        let mut inner = pair.into_inner();
        let mut domain = "clpfd".to_string(); // Default domain
        let mut raw_content = "".to_string();
        let mut body_span = Location::dummy(); // Default span if body not found

        // The first pairs can be constraint_params
        if let Some(p) = inner.peek() {
            if p.as_rule() == Rule::constraint_params {
                domain = self.parse_constraint_params(inner.next().unwrap())?;
            }
        }

        // The next part must be the constraint_body
        if let Some(body_pair) = inner.next() {
            if body_pair.as_rule() == Rule::constraint_body {
                raw_content = body_pair.as_str().to_string();
                body_span = self.pair_to_span(&body_pair);
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

    fn parse_constraint_params(&mut self, pair: Pair<Rule>) -> ParseResult<String> {
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

    pub fn build_search_params(&mut self, pair: Pair<Rule>) -> ParseResult<SearchParams> {
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
                    let name_pair = inner
                        .next()
                        .ok_or_else(|| ParseError::MissingRule(Rule::custom_param))?;
                    let name = self.create_symbol_from_pair(&name_pair);
                    let value_pair = inner
                        .next()
                        .ok_or_else(|| ParseError::MissingRule(Rule::custom_param))?;
                    let value = if value_pair.as_rule() == Rule::param_value {
                        let inner_value = value_pair
                            .into_inner()
                            .next()
                            .ok_or_else(|| ParseError::MissingRule(Rule::param_value))?;
                        self.build_param_value(inner_value)?
                    } else {
                        self.build_param_value(value_pair)?
                    };
                    params = params.with_custom_param(name, value);
                }
                _ => return Err(ParseError::UnexpectedRule(param_pair.as_rule())),
            }
        }

        Ok(params)
    }

    pub fn build_param_value(&mut self, pair: Pair<Rule>) -> ParseResult<SearchParamValue> {
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
            Rule::ident => Ok(SearchParamValue::Identifier(self.create_symbol_from_pair(&pair))),
            _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
        }
    }

    pub fn build_let_declaration(&mut self, pair: Pair<Rule>) -> ParseResult<LetDeclaration> {
        let mut inner = pair.into_inner();
        let var_name_pair = inner.next().unwrap();
        let var_name = self.create_symbol_from_pair(&var_name_pair);
        let value = if let Some(term_pair) = inner.next() {
            Some(self.build_term(term_pair)?)
        } else {
            None
        };
        Ok(LetDeclaration { var_name, value })
    }

    pub fn build_fresh_variables(&mut self, pair: Pair<Rule>) -> ParseResult<FreshVariables> {
        let inner = pair.into_inner();
        let mut vars = vec![];
        let mut body = None;

        for part in inner {
            match part.as_rule() {
                Rule::ident => {
                    let symbol = self.create_symbol_from_pair(&part);
                    vars.push(Parameter {
                        name: symbol,
                        type_annotation: None, // No type annotation support in grammar yet
                    });
                },
                Rule::goal_body => body = Some(self.build_goal_body(part)?),
                _ => (),
            }
        }
        Ok(FreshVariables {
            vars,
            body: body.unwrap_or_default(),
        })
    }

    pub fn build_disjunction(&mut self, pair: Pair<Rule>) -> ParseResult<Goal> {
        let span = self.pair_to_span(&pair);
        let mut inner = pair.into_inner();
        let mut params = None;
        let mut body = vec![];

        while let Some(part) = inner.next() {
            match part.as_rule() {
                Rule::search_params => {
                    params = Some(self.build_search_params(part)?);
                }
                Rule::goal_body => {
                    body = self.build_goal_body(part)?;
                }
                _ => return Err(ParseError::UnexpectedRule(part.as_rule())),
            }
        }

        Ok(Goal::Disjunction(Disjunction { body, params }, span))
    }

    pub fn build_conjunction(&mut self, pair: Pair<Rule>) -> ParseResult<Goal> {
        let span = self.pair_to_span(&pair);
        let mut inner = pair.into_inner();
        let mut params = None;
        let mut body = vec![];

        while let Some(part) = inner.next() {
            match part.as_rule() {
                Rule::search_params => {
                    params = Some(self.build_search_params(part)?);
                }
                Rule::goal_body => {
                    body = self.build_goal_body(part)?;
                }
                _ => return Err(ParseError::UnexpectedRule(part.as_rule())),
            }
        }

        Ok(Goal::Conjunction(Conjunction { body, params }, span))
    }

    pub fn build_pattern_matching(&mut self, pair: Pair<Rule>) -> ParseResult<PatternMatching> {
        let mut inner = pair.into_inner();
        let term = self.build_term(inner.next().unwrap())?;
        let mut arms = vec![];
        for arm_pair in inner {
            arms.push(self.build_pattern_arm(arm_pair)?);
        }
        Ok(PatternMatching { term, arms })
    }

    pub fn build_pattern_arm(&mut self, pair: Pair<Rule>) -> ParseResult<PatternArm> {
        let mut inner = pair.into_inner();
        let pattern = self.build_pattern(inner.next().unwrap())?;
        
        // The next elements could be: if_keyword, goal, pattern_arrow, body
        // or just: pattern_arrow, body
        let mut guard = None;
        let next_pair = inner.next().unwrap();
        
        // Check if this is the if_keyword (guard present) or pattern arrow (no guard)
        if next_pair.as_rule() == Rule::if_keyword {
            // We have a guard: if_keyword followed by goal
            let goal_pair = inner.next().unwrap(); // Get the goal that follows if_keyword
            guard = Some(self.build_goal(goal_pair)?);
            
            // Now get the pattern arrow
            let arrow_pair = inner.next().unwrap();
            if arrow_pair.as_rule() != Rule::pattern_arrow {
                return Err(ParseError::UnexpectedRule(arrow_pair.as_rule()));
            }
        } else if next_pair.as_rule() == Rule::pattern_arrow {
            // No guard, this is the pattern arrow directly
            // Continue with body parsing
        } else {
            return Err(ParseError::UnexpectedRule(next_pair.as_rule()));
        }
        
        let body_part = inner.next().unwrap();
        let body = match body_part.as_rule() {
            Rule::goal => vec![self.build_goal(body_part)?],
            Rule::goal_body => self.build_goal_body(body_part)?,
            _ => return Err(ParseError::UnexpectedRule(body_part.as_rule())),
        };
        Ok(PatternArm { pattern, guard, body })
    }
}