use crate::PatternVariableSet;
use crate::{Pattern, TreeTerm};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::{Brace, Paren};
use syn::{braced, parenthesized, Ident, Token};

#[derive(Clone, Debug)]
pub struct UnnamedCompoundConstructorArgument {
    pattern: Pattern,
}

impl Parse for UnnamedCompoundConstructorArgument {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(UnnamedCompoundConstructorArgument {
            pattern: input.parse()?,
        })
    }
}

impl ToTokens for UnnamedCompoundConstructorArgument {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match &self.pattern {
            Pattern::Term(treeterm) => match treeterm {
                TreeTerm::Var(x) => {
                    let output = quote! { ::proto_vulcan::Upcast::to_super(&#x) };
                    output.to_tokens(tokens);
                }
                TreeTerm::Any(_) => {
                    let output = quote! { ::proto_vulcan::compound::CompoundTerm::new_wildcard() };
                    output.to_tokens(tokens);
                }
                _ => treeterm.to_tokens(tokens),
            },
            _ => self.pattern.to_tokens(tokens),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NamedCompoundConstructorArgument {
    ident: Ident,
    colon_token: Token![:],
    pattern: Pattern,
}

impl Parse for NamedCompoundConstructorArgument {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident;
        let colon_token;
        let pattern;
        if input.peek(Ident) && input.peek2(Token![:]) {
            ident = input.parse()?;
            colon_token = input.parse()?;
            pattern = input.parse()?;
        } else {
            pattern = input.parse()?;
            match pattern {
                Pattern::Term(TreeTerm::Var(ref var_ident)) => {
                    ident = var_ident.clone();
                    colon_token = syn::parse_quote!(:);
                }
                _ => return Err(input.error("Expected variable identifier for unnamed field.")),
            }
        }

        Ok(NamedCompoundConstructorArgument {
            ident,
            colon_token,
            pattern,
        })
    }
}

impl ToTokens for NamedCompoundConstructorArgument {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = &self.ident;
        let colon_token = &self.colon_token;
        match &self.pattern {
            Pattern::Term(treeterm) => match treeterm {
                TreeTerm::Var(x) => {
                    // No Into::into() for compound pattern arguments to constrain type.
                    let output =
                        quote! { #ident #colon_token ::proto_vulcan::Upcast::to_super(&#x) };
                    output.to_tokens(tokens);
                }
                TreeTerm::Any(_) => {
                    let output = quote! { #ident #colon_token ::proto_vulcan::compound::CompoundTerm::new_wildcard() };
                    output.to_tokens(tokens);
                }
                _ => {
                    let output = quote! { #ident #colon_token #treeterm };
                    output.to_tokens(tokens);
                }
            },
            _ => {
                let pattern = &self.pattern;
                let output = quote! { #ident #colon_token #pattern };
                output.to_tokens(tokens);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TupleCompoundConstructorArgument {
    pattern: Pattern,
}

impl Parse for TupleCompoundConstructorArgument {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(TupleCompoundConstructorArgument {
            pattern: input.parse()?,
        })
    }
}

impl ToTokens for TupleCompoundConstructorArgument {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match &self.pattern {
            Pattern::Term(treeterm) => match treeterm {
                TreeTerm::Var(x) => {
                    let output = quote! { ::proto_vulcan::Upcast::to_super(&#x) };
                    output.to_tokens(tokens);
                }
                TreeTerm::Any(_) => {
                    let output = quote! { ::proto_vulcan::compound::CompoundTerm::new_wildcard() };
                    output.to_tokens(tokens);
                }
                _ => treeterm.to_tokens(tokens),
            },
            _ => self.pattern.to_tokens(tokens),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct UnnamedCompoundConstructor {
    compound_path: CompoundPath,
    paren_token: Option<Paren>,
    arguments: Option<Punctuated<UnnamedCompoundConstructorArgument, Token![,]>>,
}

impl ToTokens for UnnamedCompoundConstructor {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let compound_path = &self.compound_path;
        let output;
        if self.arguments.is_some() {
            let arguments: Vec<UnnamedCompoundConstructorArgument> =
                self.arguments.as_ref().unwrap().iter().cloned().collect();
            output = quote! { ::proto_vulcan::Upcast::into_super(::proto_vulcan::Downcast::into_sub( #compound_path (  #( #arguments ),* ) ) )};
        } else {
            output = quote! { ::proto_vulcan::Upcast::into_super(::proto_vulcan::Downcast::into_sub( #compound_path ) )};
        }
        output.to_tokens(tokens);
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct NamedCompoundConstructor {
    compound_path: CompoundPath,
    brace_token: Option<Brace>,
    arguments: Option<Punctuated<NamedCompoundConstructorArgument, Token![,]>>,
}

impl ToTokens for NamedCompoundConstructor {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let compound_path = &self.compound_path;
        let output;
        if self.arguments.is_some() {
            let arguments: Vec<NamedCompoundConstructorArgument> =
                self.arguments.as_ref().unwrap().iter().cloned().collect();
            output = quote! { ::proto_vulcan::Upcast::into_super(::proto_vulcan::Downcast::into_sub( #compound_path {  #( #arguments ),* } ) )};
        } else {
            output = quote! { ::proto_vulcan::Upcast::into_super(::proto_vulcan::Downcast::into_sub( #compound_path { } ) )};
        }
        output.to_tokens(tokens);
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct TupleCompoundConstructor {
    paren_token: Paren,
    arguments: Punctuated<TupleCompoundConstructorArgument, Token![,]>,
}

impl ToTokens for TupleCompoundConstructor {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let arguments: Vec<TupleCompoundConstructorArgument> =
            self.arguments.iter().cloned().collect();
        let output = quote! { ::proto_vulcan::Upcast::into_super(::proto_vulcan::Downcast::into_sub( (  #( #arguments ),* ) ) )};
        output.to_tokens(tokens);
    }
}

#[derive(Clone, Debug)]
pub enum CompoundConstructor {
    Unnamed(UnnamedCompoundConstructor),
    Named(NamedCompoundConstructor),
    Tuple(TupleCompoundConstructor),
}

impl Parse for CompoundConstructor {
    fn parse(input: ParseStream) -> Result<Self> {
        // Handle Tuple-constructor first
        if input.peek(Paren) {
            let content;
            return Ok(CompoundConstructor::Tuple(TupleCompoundConstructor {
                paren_token: parenthesized!(content in input),
                arguments: content.parse_terminated(TupleCompoundConstructorArgument::parse)?,
            }));
        }

        let compound_path: CompoundPath = input.parse()?;

        if input.peek(Brace) {
            let content;
            Ok(CompoundConstructor::Named(NamedCompoundConstructor {
                compound_path,
                brace_token: Some(braced!(content in input)),
                arguments: Some(content.parse_terminated(NamedCompoundConstructorArgument::parse)?),
            }))
        } else if input.peek(Paren) {
            let content;
            Ok(CompoundConstructor::Unnamed(UnnamedCompoundConstructor {
                compound_path,
                paren_token: Some(parenthesized!(content in input)),
                arguments: Some(
                    content.parse_terminated(UnnamedCompoundConstructorArgument::parse)?,
                ),
            }))
        } else {
            Ok(CompoundConstructor::Unnamed(UnnamedCompoundConstructor {
                compound_path,
                paren_token: None,
                arguments: None,
            }))
        }
    }
}

impl ToTokens for CompoundConstructor {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            CompoundConstructor::Unnamed(pattern) => pattern.to_tokens(tokens),
            CompoundConstructor::Named(pattern) => pattern.to_tokens(tokens),
            CompoundConstructor::Tuple(pattern) => pattern.to_tokens(tokens),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct CompoundPath {
    leading_colon: Option<Token![::]>,
    path: Punctuated<Ident, Token![::]>,
    num_snake_case: usize,
    prefix: Punctuated<Ident, Token![::]>,
    typename: Ident,
    variant: Option<Ident>,
}

impl Parse for CompoundPath {
    fn parse(input: ParseStream) -> Result<Self> {
        let leading_colon = if input.peek(syn::token::Colon2) {
            Some(input.parse()?)
        } else {
            None
        };

        let mut num_snake_case: usize = 0;

        let mut path: Punctuated<Ident, syn::token::Colon2> = Punctuated::new();
        loop {
            let ident: Ident;
            if input.peek(Token![crate]) {
                let _: Token![crate] = input.parse()?;
                ident = quote::format_ident!("crate");
            } else {
                ident = input.parse()?;
            }

            if ident.to_string().chars().any(|c| c.is_uppercase()) {
                num_snake_case += 1;
            }
            path.push_value(ident);

            if !input.peek(Token![::]) {
                break;
            }

            let punct = input.parse()?;
            path.push_punct(punct);
        }

        let mut prefix = path.clone();
        let typename;
        let mut variant = None;
        match num_snake_case {
            0 => return Err(input.error("Type patterns must have snake-case names.")),
            1 => {
                typename = prefix.pop().unwrap().into_value();
            }
            2 => {
                variant = Some(prefix.pop().unwrap().into_value());
                typename = prefix.pop().unwrap().into_value();
            }
            _ => return Err(input.error("Ambiguous path with more than two snake-case segments")),
        }

        Ok(CompoundPath {
            leading_colon,
            path,
            num_snake_case,
            prefix,
            typename,
            variant,
        })
    }
}

impl ToTokens for CompoundPath {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let leading_colon = &self.leading_colon;
        let prefix = &self.prefix;
        let typename = &self.typename;
        let variant = &self.variant;

        let mut new_path = prefix.clone();
        if typename == "Some" {
            new_path.push(typename.clone());
        } else {
            let compound_mod_name = quote::format_ident!("{}_compound", typename);
            let object_typename = quote::format_ident!("_Inner{}", typename);

            if variant.is_some() {
                new_path.push(compound_mod_name);
                new_path.push(variant.clone().unwrap());
            } else {
                new_path.push(compound_mod_name);
                new_path.push(object_typename);
            }
        }

        let output = quote!(#leading_colon #new_path);
        output.to_tokens(tokens);
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct UnnamedCompoundPattern {
    compound_path: CompoundPath,
    paren_token: Option<Paren>,
    arguments: Option<Punctuated<CompoundArgument, Token![,]>>,
}

impl UnnamedCompoundPattern {
    fn get_vars(&self, vars: &mut PatternVariableSet) {
        if self.arguments.is_some() {
            for pattern in self.arguments.as_ref().unwrap().iter() {
                pattern.get_vars(vars);
            }
        }
    }
}

impl ToTokens for UnnamedCompoundPattern {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let compound_path = &self.compound_path;
        if self.arguments.is_some() {
            let arguments: Vec<&CompoundArgument> =
                self.arguments.as_ref().unwrap().iter().collect();
            let output = quote! { ::proto_vulcan::Upcast::into_super(::proto_vulcan::Downcast::into_sub( #compound_path ( #( #arguments ),* ) )) };
            output.to_tokens(tokens);
        } else {
            let output = quote! { ::proto_vulcan::Upcast::into_super(::proto_vulcan::Downcast::into_sub( #compound_path )) };
            output.to_tokens(tokens);
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct NamedCompoundPattern {
    compound_path: CompoundPath,
    brace_token: Option<Brace>,
    arguments: Option<Punctuated<NamedCompoundArgument, Token![,]>>,
}

impl NamedCompoundPattern {
    fn get_vars(&self, vars: &mut PatternVariableSet) {
        if self.arguments.is_some() {
            for pattern in self.arguments.as_ref().unwrap().iter() {
                pattern.get_vars(vars);
            }
        }
    }
}

impl ToTokens for NamedCompoundPattern {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let compound_path = &self.compound_path;
        if self.arguments.is_some() {
            let arguments: Vec<NamedCompoundArgument> =
                self.arguments.as_ref().unwrap().iter().cloned().collect();

            let output = quote! { ::proto_vulcan::Upcast::into_super(::proto_vulcan::Downcast::into_sub( #compound_path { #( #arguments ),* } )) };
            output.to_tokens(tokens);
        } else {
            let output = quote! { ::proto_vulcan::Upcast::into_super(::proto_vulcan::Downcast::into_sub( #compound_path { } )) };
            output.to_tokens(tokens);
        }
    }
}

#[derive(Clone, Debug)]
pub enum CompoundPattern {
    Unnamed(UnnamedCompoundPattern),
    Named(NamedCompoundPattern),
    //Tuple
}

impl CompoundPattern {
    pub fn is_next_compound(input: ParseStream) -> bool {
        if input.peek(Token![::])
            || input.peek2(Token![::])
            || input.peek2(Paren)
            || input.peek2(Brace)
        {
            true
        } else {
            false
        }
    }

    pub fn get_vars(&self, vars: &mut PatternVariableSet) {
        match self {
            CompoundPattern::Unnamed(pattern) => pattern.get_vars(vars),
            CompoundPattern::Named(pattern) => pattern.get_vars(vars),
        }
    }
}

impl Parse for CompoundPattern {
    fn parse(input: ParseStream) -> Result<Self> {
        let compound_path: CompoundPath = input.parse()?;

        if input.peek(Brace) {
            let content;
            Ok(CompoundPattern::Named(NamedCompoundPattern {
                compound_path,
                brace_token: Some(braced!(content in input)),
                arguments: Some(content.parse_terminated(NamedCompoundArgument::parse)?),
            }))
        } else if input.peek(Paren) {
            let content;
            Ok(CompoundPattern::Unnamed(UnnamedCompoundPattern {
                compound_path,
                paren_token: Some(parenthesized!(content in input)),
                arguments: Some(content.parse_terminated(CompoundArgument::parse)?),
            }))
        } else {
            Ok(CompoundPattern::Unnamed(UnnamedCompoundPattern {
                compound_path,
                paren_token: None,
                arguments: None,
            }))
        }
    }
}

impl ToTokens for CompoundPattern {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            CompoundPattern::Unnamed(pattern) => pattern.to_tokens(tokens),
            CompoundPattern::Named(pattern) => pattern.to_tokens(tokens),
        }
    }
}

#[derive(Clone, Debug)]
struct CompoundArgument {
    pattern: Pattern,
}

impl CompoundArgument {
    fn get_vars(&self, vars: &mut PatternVariableSet) {
        self.pattern.get_vars(vars);
        if let Pattern::Term(TreeTerm::Var(ref x)) = self.pattern {
            vars.set_compound(x);
        }
    }
}

impl Parse for CompoundArgument {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(CompoundArgument {
            pattern: input.parse()?,
        })
    }
}

impl ToTokens for CompoundArgument {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match &self.pattern {
            Pattern::Term(treeterm) => match treeterm {
                TreeTerm::Var(x) => {
                    // No Into::into() for compound pattern arguments to constrain type.
                    let output = quote! { ::std::clone::Clone::clone(&#x) };
                    output.to_tokens(tokens);
                }
                TreeTerm::Any(_) => {
                    let output = quote! { ::proto_vulcan::compound::CompoundTerm::new_wildcard() };
                    output.to_tokens(tokens);
                }
                term if term.is_empty() => {
                    let output = quote! { ::proto_vulcan::compound::CompoundTerm::new_none() };
                    output.to_tokens(tokens);
                }
                _ => treeterm.to_tokens(tokens),
            },
            _ => self.pattern.to_tokens(tokens),
        }
    }
}

#[derive(Clone, Debug)]
struct NamedCompoundArgument {
    ident: Ident,
    colon_token: Token![:],
    pattern: Pattern,
}

impl NamedCompoundArgument {
    fn get_vars(&self, vars: &mut PatternVariableSet) {
        self.pattern.get_vars(vars);
        if let Pattern::Term(TreeTerm::Var(ref x)) = self.pattern {
            vars.set_compound(x);
        }
    }
}

impl Parse for NamedCompoundArgument {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident;
        let colon_token;
        let pattern;
        if input.peek(Ident) && input.peek2(Token![:]) {
            ident = input.parse()?;
            colon_token = input.parse()?;
            pattern = input.parse()?;
        } else {
            pattern = input.parse()?;
            match pattern {
                Pattern::Term(TreeTerm::Var(ref var_ident)) => {
                    ident = var_ident.clone();
                    colon_token = syn::parse_quote!(:);
                }
                _ => return Err(input.error("Expected variable identifier for unnamed field.")),
            }
        }

        Ok(NamedCompoundArgument {
            ident,
            colon_token,
            pattern,
        })
    }
}

impl ToTokens for NamedCompoundArgument {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = &self.ident;
        let colon_token = &self.colon_token;
        match &self.pattern {
            Pattern::Term(treeterm) => match treeterm {
                TreeTerm::Var(x) => {
                    // No Into::into() for compound pattern arguments to constrain type.
                    let output = quote! { #ident #colon_token ::std::clone::Clone::clone(&#x) };
                    output.to_tokens(tokens);
                }
                TreeTerm::Any(_) => {
                    let output = quote! { #ident #colon_token ::proto_vulcan::compound::CompoundTerm::new_wildcard() };
                    output.to_tokens(tokens);
                }
                term if term.is_empty() => {
                    let output = quote! { #ident #colon_token ::proto_vulcan::compound::CompoundTerm::new_none() };
                    output.to_tokens(tokens);
                }
                _ => {
                    let output = quote! { #ident #colon_token #treeterm };
                    output.to_tokens(tokens);
                }
            },
            _ => {
                let pattern = &self.pattern;
                let output = quote! { #ident #colon_token #pattern };
                output.to_tokens(tokens);
            }
        }
    }
}
