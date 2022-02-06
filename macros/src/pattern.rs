use crate::CompoundPattern;
use crate::{Clause, TreeTerm};
use quote::{quote, ToTokens};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Brace;
use syn::{braced, Error, Ident, Token};

pub struct PatternVariableSet {
    idents: HashSet<Ident>,

    // The `compound` flags affects the variable builder selection and how the
    // variable is accessed in the compound pattern arguments.
    //
    //  * When variable is at compound pattern argument position, then the type
    //    of the argument is derived from the compound term builder. Therefore,
    //    at compound argument position the variable is accessed by only cloning
    //    a reference, without the enclosing Into::into()-call.
    //
    // Compound flag      let #x =                      Access
    //     true           CompoundTerm::new_wildcard()  Clone::clone(&#x)
    //     false          LTerm::var(#x)                Into::into(Clone::clone(&#x))
    is_compound: HashSet<Ident>,
}

impl PatternVariableSet {
    pub fn new() -> PatternVariableSet {
        PatternVariableSet {
            idents: HashSet::new(),
            is_compound: HashSet::new(),
        }
    }

    pub fn set_compound(&mut self, ident: &Ident) {
        self.is_compound.insert(ident.clone());
    }

    pub fn is_compound(&self, ident: &Ident) -> bool {
        self.is_compound.contains(ident)
    }
}

impl std::ops::Deref for PatternVariableSet {
    type Target = HashSet<Ident>;

    fn deref(&self) -> &Self::Target {
        &self.idents
    }
}

impl std::ops::DerefMut for PatternVariableSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.idents
    }
}

#[derive(Clone, Debug)]
pub enum Pattern {
    Term(TreeTerm),
    Compound(CompoundPattern),
}

impl Pattern {
    pub fn get_vars(&self, vars: &mut PatternVariableSet) {
        match self {
            Pattern::Term(term) => term.get_vars(vars),
            Pattern::Compound(compound) => compound.get_vars(vars),
        }
    }
}

impl Parse for Pattern {
    fn parse(input: ParseStream) -> Result<Self> {
        if CompoundPattern::is_next_compound(input) {
            Ok(Pattern::Compound(CompoundPattern::parse(input)?))
        } else {
            Ok(Pattern::Term(TreeTerm::parse(input)?))
        }
    }
}

impl ToTokens for Pattern {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Pattern::Term(treeterm) => treeterm.to_tokens(tokens),
            Pattern::Compound(compound) => compound.to_tokens(tokens),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct PatternArm {
    patterns: Vec<Pattern>,
    arrow: Token![=>],
    brace_token: Option<Brace>,
    body: Punctuated<Clause, Token![,]>,
}

impl Parse for PatternArm {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut patterns = vec![];
        loop {
            let pattern: Pattern = input.parse()?;
            patterns.push(pattern);

            if input.peek(Token![|]) {
                let _: Token![|] = input.parse()?;
            } else if input.peek(Token![=>]) {
                break;
            }
        }

        for pattern in patterns.iter() {
            let mut pattern_vars = PatternVariableSet::new();
            pattern.get_vars(&mut pattern_vars);
            for var_ident in pattern_vars.iter() {
                if var_ident.to_string() == "__term__" {
                    return Err(Error::new(
                        var_ident.span(),
                        "A pattern variable cannot be named '__term__'",
                    ));
                }
            }
        }

        let arrow: Token![=>] = input.parse()?;

        if input.peek(Brace) {
            let content;
            let brace_token = braced!(content in input);
            let body = content.parse_terminated(Clause::parse)?;
            Ok(PatternArm {
                patterns,
                arrow,
                brace_token: Some(brace_token),
                body,
            })
        } else if input.peek(Token![,]) {
            Ok(PatternArm {
                patterns,
                arrow,
                brace_token: None,
                body: Punctuated::new(),
            })
        } else {
            let mut body: Punctuated<Clause, Token![,]> = Punctuated::new();
            body.push(input.parse()?);
            Ok(PatternArm {
                patterns,
                arrow,
                brace_token: None,
                body,
            })
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct PatternMatchOperator {
    name: Ident,
    term: TreeTerm,
    brace_token: Brace,
    arms: Punctuated<PatternArm, Token![,]>,
}

impl Parse for PatternMatchOperator {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident;
        if input.peek(Ident) {
            name = input.parse()?;
        } else {
            let token: Token![match] = input.parse()?;
            name = Ident::new("match", token.span);
        };
        let content;
        Ok(PatternMatchOperator {
            name,
            term: input.parse()?,
            brace_token: braced!(content in input),
            arms: content.parse_terminated(PatternArm::parse)?,
        })
    }
}

impl ToTokens for PatternMatchOperator {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.name;
        let term = &self.term;

        let mut patterns: Vec<Pattern> = vec![];
        let mut vars: Vec<Vec<Ident>> = vec![];
        let mut compounds: Vec<Vec<Ident>> = vec![];
        let mut clauses: Vec<Punctuated<proc_macro2::TokenStream, Token![,]>> = vec![];
        for arm in self.arms.iter() {
            // Repeat |-expression patterns with multiple single pattern entries
            for pattern in arm.patterns.iter() {
                patterns.push(pattern.clone());
                let mut pattern_vars = PatternVariableSet::new();
                pattern.get_vars(&mut pattern_vars);
                let mut treeterm_pattern_vars = vec![];
                let mut compound_pattern_vars = vec![];
                pattern_vars.iter().for_each(|x| {
                    if pattern_vars.is_compound(x) {
                        compound_pattern_vars.push(x.clone());
                    } else {
                        treeterm_pattern_vars.push(x.clone());
                    }
                });
                vars.push(treeterm_pattern_vars);
                compounds.push(compound_pattern_vars);
                let mut arm_clauses: Punctuated<proc_macro2::TokenStream, Token![,]> =
                    Punctuated::new();
                for clause in arm.body.iter() {
                    let tokens = quote! {
                        ::proto_vulcan::GoalCast::cast_into( #clause )
                    };
                    arm_clauses.push(tokens);
                }
                clauses.push(arm_clauses);
            }
        }

        let output = if name.to_string() == "match" {
            quote! {
                ::proto_vulcan::operator::conde::Conde::from_conjunctions (
                    &[ #( &{
                        // Define alias for the `term` so that pattern-variables do not redefine it
                        // before the equality-relation with pattern is created.
                        let __term__ = #term;
                        // Define new variables found in the pattern
                        #( let #vars = ::proto_vulcan::lterm::LTerm::var(stringify!(#vars)); )*
                        #( let #compounds = ::proto_vulcan::compound::CompoundTerm::new_var(stringify!(#compounds)); )*
                        let __pattern__ = #patterns;
                        [::proto_vulcan::GoalCast::cast_into(
                            ::proto_vulcan::relation::eq(__term__, __pattern__)),
                         #clauses]
                    } ),* ],
                )
            }
        } else {
            quote! {
                #name ( ::proto_vulcan::operator::PatternMatchOperatorParam::new(
                    &[ #( &{
                        // Define alias for the `term` so that pattern-variables do not redefine it
                        // before the equality-relation with pattern is created.
                        let __term__ = #term;
                        // Define new variables found in the pattern
                        #( let #vars = ::proto_vulcan::lterm::LTerm::var(stringify!(#vars)); )*
                        #( let #compounds = ::proto_vulcan::compound::CompoundTerm::new_var(stringify!(#compounds)); )*
                        let __pattern__ = #patterns;
                        [::proto_vulcan::GoalCast::cast_into(
                            ::proto_vulcan::relation::eq(__term__, __pattern__)),
                         #clauses]
                    } ),* ],
                ))
            }
        };
        output.to_tokens(tokens);
    }
}
