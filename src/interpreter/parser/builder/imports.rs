use pest::iterators::Pair;
use super::{AstBuilder, ParseError, ParseResult, Rule};
use crate::interpreter::parser::ast::*;

impl<'a> AstBuilder<'a> {
    pub fn build_use_statement(&mut self, pair: Pair<Rule>) -> ParseResult<UseStatement> {
        let span = self.pair_to_span(&pair);
        let mut inner = pair.into_inner();
        let _use_keyword = inner.next().unwrap(); // Skip the use_keyword
        let path_pair = inner.next().unwrap(); // Get the use_path
        let path = self.build_use_path(path_pair)?;
        Ok(UseStatement { path, span })
    }

    pub fn build_use_path(&mut self, pair: Pair<Rule>) -> ParseResult<UsePath> {
        let inner = pair
            .into_inner()
            .next()
            .ok_or_else(|| ParseError::MissingRule(Rule::use_path))?;

        match inner.as_rule() {
            Rule::use_path_simple => {
                // Simple import: use use_path_base;
                let base_pair = inner.into_inner().next().unwrap(); // use_path_base
                let qualified_path = self.build_use_path_base(base_pair)?;

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

                let qualified_path = self.build_use_path_base(use_path_base_pair)?;
                Ok(UsePath::Glob(qualified_path))
            }
            Rule::use_path_list => {
                // List import: use use_path_base{...};
                let mut parts = inner.into_inner();
                let use_path_base_pair = parts.next().unwrap();
                let _path_sep = parts.next().unwrap(); // Skip the path_sep
                let list_part = parts.next().unwrap(); // Get the list_import

                let qualified_path = self.build_use_path_base(use_path_base_pair)?;

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

    pub fn build_use_path_base(&mut self, pair: Pair<Rule>) -> ParseResult<QualifiedPath> {
        match pair.as_rule() {
            Rule::use_path_base => {
                // Use_path_base now contains qualified_path
                let inner = pair.into_inner().next().unwrap();
                self.build_qualified_path(inner)
            }
            Rule::qualified_path => self.build_qualified_path(pair),
            _ => Err(ParseError::UnexpectedRule(pair.as_rule())),
        }
    }

    pub fn build_qualified_path(&mut self, pair: Pair<Rule>) -> ParseResult<QualifiedPath> {
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

    pub fn build_type_name(&mut self, pair: Pair<Rule>) -> ParseResult<String> {
        match pair.as_rule() {
            Rule::type_name => {
                let inner = pair.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::qualified_path => {
                        let qualified_path = self.build_qualified_path(inner)?;
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

    pub fn build_use_path_segments(&mut self, pair: Pair<Rule>) -> ParseResult<Vec<String>> {
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::absolute_path | Rule::crate_path | Rule::super_path | Rule::self_path => {
                // Use existing qualified path logic but extract just the segments
                let qualified_path = self.build_qualified_path(inner)?;
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
}