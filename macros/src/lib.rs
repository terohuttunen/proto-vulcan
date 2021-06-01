extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Brace, Bracket, Paren};
use syn::{braced, bracketed, parenthesized, parse_macro_input, Error, Ident, Token};

#[allow(dead_code)]
#[derive(Clone)]
struct Project {
    project: Ident,
    or1_token: Token![|],
    variables: Punctuated<Ident, Token![,]>,
    or2_token: Token![|],
    brace_token: Brace,
    body: Punctuated<Clause, Token![,]>,
}

impl Parse for Project {
    fn parse(input: ParseStream) -> Result<Self> {
        let project: Ident = input.parse()?;
        if project.to_string().as_str() != "project" {
            return Err(Error::new(
                project.span(),
                "Identifier \"project\" expected",
            ));
        }

        let or1_token: Token![|] = input.parse()?;
        let mut variables = Punctuated::new();
        loop {
            if input.peek(Token![|]) {
                break;
            }
            let var: Ident = input.parse()?;
            variables.push_value(var);
            if input.peek(Token![|]) {
                break;
            }
            let punct: Token![,] = input.parse()?;
            variables.push_punct(punct);
        }
        let or2_token: Token![|] = input.parse()?;

        let content;
        Ok(Project {
            project,
            or1_token,
            variables,
            or2_token,
            brace_token: braced!(content in input),
            body: content.parse_terminated(Clause::parse)?,
        })
    }
}

impl ToTokens for Project {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let variables: Vec<&Ident> = self.variables.iter().collect();
        let body: Vec<&Clause> = self.body.iter().collect();
        let output = quote! {{
            #( let #variables = ::proto_vulcan::lterm::LTerm::projection(::std::clone::Clone::clone(&#variables)); )*
            ::proto_vulcan::operator::project::project(::proto_vulcan::operator::ProjectOperatorParam {
                var_list: vec![ #( ::std::clone::Clone::clone(&#variables) ),* ],
                body: &[ #( &[ #body  ] ),* ],
            })
        }};
        output.to_tokens(tokens);
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct FnGoal {
    fngoal: Ident,
    m: Option<Token![move]>,
    or1_token: Token![|],
    engine: Ident,
    state: Ident,
    or2_token: Token![|],
    body: syn::Block,
}

impl Parse for FnGoal {
    fn parse(input: ParseStream) -> Result<Self> {
        let fngoal: Ident = input.parse()?;
        if fngoal.to_string().as_str() != "fngoal" {
            return Err(Error::new(fngoal.span(), "Identifier \"fngoal\" expected"));
        }

        let m = if input.peek(Token![move]) {
            Some(input.parse()?)
        } else {
            None
        };

        let or1_token: Token![|] = input.parse()?;
        let engine: Ident = input.parse()?;
        let _: Token![,] = input.parse()?;
        let state: Ident = input.parse()?;
        let or2_token: Token![|] = input.parse()?;

        Ok(FnGoal {
            fngoal,
            m,
            or1_token,
            engine,
            state,
            or2_token,
            body: input.parse()?,
        })
    }
}

impl ToTokens for FnGoal {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let m = &self.m;
        let engine = &self.engine;
        let state = &self.state;
        let body: &syn::Block = &self.body;
        let output = quote! {{
            ::proto_vulcan::operator::fngoal::FnGoal::new(Box::new(#m |#engine, #state| { #body } ))
        }};
        output.to_tokens(tokens);
    }
}

#[derive(Clone)]
struct TypedVariable {
    name: Ident,
    path: syn::Path,
}

impl Parse for TypedVariable {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let path;
        if input.peek(Token![:]) {
            let _: Token![:] = input.parse()?;
            path = input.parse()?;
        } else {
            path = syn::parse_quote!(::proto_vulcan::lterm::LTerm);
        }
        Ok(TypedVariable { name, path })
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct Fresh {
    or1_token: Token![|],
    variables: Punctuated<TypedVariable, Token![,]>,
    or2_token: Token![|],
    brace_token: Brace,
    body: Punctuated<Clause, Token![,]>,
}

impl Parse for Fresh {
    fn parse(input: ParseStream) -> Result<Self> {
        let or1_token: Token![|] = input.parse()?;
        let mut variables = Punctuated::new();
        loop {
            if input.peek(Token![|]) {
                break;
            }
            let var: TypedVariable = input.parse()?;
            variables.push_value(var);
            if input.peek(Token![|]) {
                break;
            }
            let punct: Token![,] = input.parse()?;
            variables.push_punct(punct);
        }
        let or2_token: Token![|] = input.parse()?;

        let content;
        Ok(Fresh {
            or1_token,
            variables,
            or2_token,
            brace_token: braced!(content in input),
            body: content.parse_terminated(Clause::parse)?,
        })
    }
}

impl ToTokens for Fresh {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let variables: Vec<Ident> = self.variables.iter().map(|x| &x.name).cloned().collect();
        let variable_types: Vec<syn::Path> =
            self.variables.iter().map(|x| &x.path).cloned().collect();
        let body: Vec<&Clause> = self.body.iter().collect();
        let output = quote! {{
            #( let #variables: #variable_types <_>= ::proto_vulcan::compound::CompoundTerm::new_var(stringify!(#variables)); )*
            ::proto_vulcan::operator::fresh::Fresh::new(vec![ #( ::proto_vulcan::Upcast::to_super(&#variables) ),* ],
                ::proto_vulcan::operator::all::All::from_array(&[ #( #body ),* ]))
        }};
        output.to_tokens(tokens);
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct Conjunction {
    bracket_token: Bracket,
    body: Punctuated<Clause, Token![,]>,
}

impl Parse for Conjunction {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Conjunction {
            bracket_token: bracketed!(content in input),
            body: content.parse_terminated(Clause::parse)?,
        })
    }
}

impl ToTokens for Conjunction {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let body: Vec<&Clause> = self.body.iter().collect();
        let output = quote! { &[ #( #body ),* ] };
        output.to_tokens(tokens)
    }
}

#[derive(Clone, Debug)]
struct UnnamedCompoundConstructorArgument {
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
struct NamedCompoundConstructorArgument {
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
struct TupleCompoundConstructorArgument {
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

#[derive(Clone, Debug)]
struct UnnamedCompoundConstructor {
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

#[derive(Clone, Debug)]
struct NamedCompoundConstructor {
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

#[derive(Clone, Debug)]
struct TupleCompoundConstructor {
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
enum CompoundConstructor {
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

#[derive(Clone)]
enum Argument {
    TreeTerm(TreeTerm),
    Compound(CompoundConstructor),
    Quoted(syn::Expr),
    Expr(syn::Expr),
}

impl Parse for Argument {
    fn parse(input: ParseStream) -> Result<Self> {
        // No parsing for quoted input
        if input.peek(Token![#]) {
            let _: Token![#] = input.parse()?;
            let expr: syn::Expr = input.parse()?;
            Ok(Argument::Quoted(expr))
        } else {
            if (input.peek(syn::token::Colon2) && input.peek2(Ident))
                || (input.peek(Ident) && (input.peek2(syn::token::Colon2) || input.peek2(Paren))
                    || input.peek(Paren))
            {
                let compound: CompoundConstructor = input.parse()?;
                Ok(Argument::Compound(compound))
            } else if let Ok(term) = input.parse() {
                // Try parsing TreeTerm
                Ok(Argument::TreeTerm(term))
            } else {
                // By parsing parenthesises away if any, we avoid unused parenthesis warnings
                if input.peek(Paren) {
                    let content;
                    let _ = parenthesized!(content in input);
                    let expr = content.parse()?;
                    Ok(Argument::Expr(expr))
                } else {
                    let expr = input.parse()?;
                    Ok(Argument::Expr(expr))
                }
            }
        }
    }
}

impl ToTokens for Argument {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Argument::TreeTerm(term) => {
                let output = quote! { #term };
                output.to_tokens(tokens);
            }
            Argument::Compound(compound_term) => {
                let output = quote! { #compound_term };
                output.to_tokens(tokens);
            }
            Argument::Quoted(expr) => {
                expr.to_tokens(tokens);
            }
            Argument::Expr(expr) => {
                let output = quote! { ::proto_vulcan::lterm::LTerm::from(#expr) };
                output.to_tokens(tokens);
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct Relation {
    name: Ident,
    paren_token: Paren,
    body: Punctuated<Argument, Token![,]>,
}

impl Parse for Relation {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Relation {
            name: input.parse()?,
            paren_token: parenthesized!(content in input),
            body: content.parse_terminated(Argument::parse)?,
        })
    }
}

impl ToTokens for Relation {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.name;
        let body: Vec<&Argument> = self.body.iter().collect();
        let output = quote! { #name ( #( #body ),* ) };
        output.to_tokens(tokens);
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct Closure {
    body: Vec<Clause>,
}

impl Closure {
    fn new(body: Vec<Clause>) -> Closure {
        Closure { body }
    }
}

impl Parse for Closure {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        if name != String::from("closure") {
            return Err(input.error("Expected \"closure\""));
        }
        let content;
        let _ = braced!(content in input);
        let mut body = vec![];
        for clause in content.parse_terminated::<Clause, Clause>(Clause::parse)? {
            body.push(clause);
        }
        Ok(Closure { body })
    }
}

impl ToTokens for Closure {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let body: Vec<&Clause> = self.body.iter().collect();
        let output = quote! {{
            ::proto_vulcan::operator::closure::Closure::new(::proto_vulcan::operator::ClosureOperatorParam {f: Box::new(move || ::proto_vulcan::operator::all::All::from_array( &[ #( #body ),* ] ) )})
        }};
        output.to_tokens(tokens);
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct Loop {
    kw: Token![loop],
    brace_token: Brace,
    body: Punctuated<ClauseInOperator, Token![,]>,
}

impl Parse for Loop {
    fn parse(input: ParseStream) -> Result<Self> {
        let kw = input.parse()?;
        let content;
        Ok(Loop {
            kw,
            brace_token: braced!(content in input),
            body: content.parse_terminated(ClauseInOperator::parse)?,
        })
    }
}

impl ToTokens for Loop {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let body: Vec<&ClauseInOperator> = self.body.iter().collect();
        let output = quote! {{
            ::proto_vulcan::operator::anyo::anyo(::proto_vulcan::operator::OperatorParam { body: &[ #( #body ),* ] })
        }};
        output.to_tokens(tokens);
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct Operator {
    name: Ident,
    brace_token: Brace,
    body: Punctuated<ClauseInOperator, Token![,]>,
}

impl Parse for Operator {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Operator {
            name: input.parse()?,
            brace_token: braced!(content in input),
            body: content.parse_terminated(ClauseInOperator::parse)?,
        })
    }
}

impl ToTokens for Operator {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.name;
        let body: Vec<&ClauseInOperator> = self.body.iter().collect();
        let output =
            quote! { #name ( ::proto_vulcan::operator::OperatorParam { body: &[ #( #body ),* ] } )};
        output.to_tokens(tokens);
    }
}

struct PatternVariableSet {
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
    fn new() -> PatternVariableSet {
        PatternVariableSet {
            idents: HashSet::new(),
            is_compound: HashSet::new(),
        }
    }

    fn set_compound(&mut self, ident: &Ident) {
        self.is_compound.insert(ident.clone());
    }

    fn is_compound(&self, ident: &Ident) -> bool {
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

#[derive(Clone, Debug)]
struct UnnamedCompoundPattern {
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

#[derive(Clone, Debug)]
struct NamedCompoundPattern {
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
enum CompoundPattern {
    Unnamed(UnnamedCompoundPattern),
    Named(NamedCompoundPattern),
    //Tuple
}

impl CompoundPattern {
    fn is_next_compound(input: ParseStream) -> bool {
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

    fn get_vars(&self, vars: &mut PatternVariableSet) {
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
enum Pattern {
    Term(TreeTerm),
    Compound(CompoundPattern),
}

impl Pattern {
    fn is_any(&self) -> bool {
        match self {
            Pattern::Term(treeterm) => treeterm.is_any(),
            _ => false,
        }
    }
}

impl Pattern {
    fn get_vars(&self, vars: &mut PatternVariableSet) {
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
struct PatternMatchOperator {
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
        let mut clauses: Vec<Punctuated<Clause, Token![,]>> = vec![];
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
                clauses.push(arm.body.clone());
            }
        }

        let output = if name.to_string() == "match" {
            quote! {
                ::proto_vulcan::operator::matche ( ::proto_vulcan::operator::PatternMatchOperatorParam {
                    arms: &[ #( &{
                        // Define alias for the `term` so that pattern-variables do not redefine it
                        // before the equality-relation with pattern is created.
                        let __term__ = #term;
                        // Define new variables found in the pattern
                        #( let #vars = ::proto_vulcan::lterm::LTerm::var(stringify!(#vars)); )*
                        #( let #compounds = ::proto_vulcan::compound::CompoundTerm::new_var(stringify!(#compounds)); )*
                        let __pattern__ = #patterns;
                        [::proto_vulcan::relation::eq(__term__, __pattern__), #clauses ]
                    } ),* ],
                })
            }
        } else {
            quote! {
                #name ( ::proto_vulcan::operator::PatternMatchOperatorParam {
                    arms: &[ #( &{
                        // Define alias for the `term` so that pattern-variables do not redefine it
                        // before the equality-relation with pattern is created.
                        let __term__ = #term;
                        // Define new variables found in the pattern
                        #( let #vars = ::proto_vulcan::lterm::LTerm::var(stringify!(#vars)); )*
                        #( let #compounds = ::proto_vulcan::compound::CompoundTerm::new_var(stringify!(#compounds)); )*
                        let __pattern__ = #patterns;
                        [::proto_vulcan::relation::eq(__term__, __pattern__), #clauses ]
                    } ),* ],
                })
            }
        };
        output.to_tokens(tokens);
    }
}

#[derive(Clone)]
struct For {
    for_token: Token![for],
    pattern: Ident,
    in_token: Token![in],
    coll: syn::Expr,
    brace_token: Brace,
    body: Punctuated<ClauseInOperator, Token![,]>,
}

impl Parse for For {
    fn parse(input: ParseStream) -> Result<Self> {
        let for_token: Token![for] = input.parse()?;
        let pattern = input.parse()?;
        let in_token: Token![in] = input.parse()?;
        let coll = input.call(syn::Expr::parse_without_eager_brace)?;
        let content;
        let brace_token = braced!(content in input);
        let body = content.parse_terminated(ClauseInOperator::parse)?;
        Ok(For {
            for_token,
            pattern,
            in_token,
            coll,
            brace_token,
            body,
        })
    }
}

impl ToTokens for For {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let pattern = &self.pattern;
        let coll = &self.coll;
        let body: Vec<&ClauseInOperator> = self.body.iter().collect();
        let output = quote!({
            ::proto_vulcan::operator::everyg(::proto_vulcan::operator::ForOperatorParam {
                coll: ::std::clone::Clone::clone(#coll),
                g: Box::new(|#pattern| ::proto_vulcan::operator::all::All::from_conjunctions(&[ #( #body ),* ])),
            })
        });
        output.to_tokens(tokens);
    }
}

#[derive(Clone, Debug)]
enum Value {
    Bool(syn::LitBool),
    Number(syn::LitInt),
    Char(syn::LitChar),
    String(syn::LitStr),
}

impl Parse for Value {
    fn parse(input: ParseStream) -> Result<Self> {
        let lit: syn::Lit = input.parse()?;
        match lit {
            syn::Lit::Str(s) => Ok(Value::String(s)),
            syn::Lit::Char(c) => Ok(Value::Char(c)),
            syn::Lit::Int(n) => Ok(Value::Number(n)),
            syn::Lit::Bool(b) => Ok(Value::Bool(b)),
            _ => Err(Error::new(lit.span(), "Invalid literal")),
        }
    }
}

impl ToTokens for Value {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Value::Bool(b) => b.to_tokens(tokens),
            Value::Number(n) => n.to_tokens(tokens),
            Value::Char(c) => c.to_tokens(tokens),
            Value::String(s) => s.to_tokens(tokens),
        }
    }
}

#[derive(Clone, Debug)]
struct FieldAccess {
    field: Punctuated<Ident, Token![.]>,
}

impl Parse for FieldAccess {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut field = Punctuated::new();
        loop {
            if let Ok(p) = input.parse() {
                field.push_value(p);
            } else {
                let p: Token![self] = input.parse()?;
                field.push_value(Ident::new("self", p.span))
            }

            if !input.peek(Token![.]) {
                break;
            }
            let punct: Token![.] = input.parse()?;
            field.push_punct(punct);
        }

        Ok(FieldAccess { field })
    }
}

/// TreeTerm within a TreeTerm
#[derive(Clone, Debug)]
struct InnerTreeTerm(TreeTerm);

impl InnerTreeTerm {
    fn get_vars(&self, vars: &mut PatternVariableSet) {
        self.0.get_vars(vars)
    }
}

impl Parse for InnerTreeTerm {
    fn parse(input: ParseStream) -> Result<Self> {
        let term: TreeTerm = input.parse()?;
        Ok(InnerTreeTerm(term))
    }
}

impl From<TreeTerm> for InnerTreeTerm {
    fn from(u: TreeTerm) -> InnerTreeTerm {
        InnerTreeTerm(u)
    }
}

impl ToTokens for InnerTreeTerm {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match &self.0 {
            TreeTerm::Value(value) => {
                let output = quote! { ::proto_vulcan::lterm::LTerm::from(#value) };
                output.to_tokens(tokens);
            }
            TreeTerm::Var(ident) => {
                let output = quote! { ::std::clone::Clone::clone(&#ident) };
                output.to_tokens(tokens);
            }
            TreeTerm::Field(field_access) => {
                let field = &field_access.field;
                let output = quote! { ::std::clone::Clone::clone(&#field) };
                output.to_tokens(tokens);
            }
            TreeTerm::Any(_) => {
                let output = quote! { ::proto_vulcan::lterm::LTerm::any() };
                output.to_tokens(tokens);
            }
            TreeTerm::ImproperList { items } => {
                let items: Vec<&InnerTreeTerm> = items.iter().collect();
                let output = quote! { ::proto_vulcan::lterm::LTerm::improper_from_array( &[ #(#items),* ] ) };
                output.to_tokens(tokens);
            }
            TreeTerm::ProperList { items } => {
                let items: Vec<&InnerTreeTerm> = items.iter().collect();
                let output =
                    quote! { ::proto_vulcan::lterm::LTerm::from_array( &[ #(#items),* ] ) };
                output.to_tokens(tokens);
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
enum TreeTerm {
    Value(Value),
    Var(Ident),
    Field(FieldAccess),
    Any(Token![_]),
    ImproperList { items: Vec<InnerTreeTerm> },
    ProperList { items: Vec<InnerTreeTerm> },
}

impl TreeTerm {
    fn is_any(&self) -> bool {
        match self {
            TreeTerm::Any(_) => true,
            _ => false,
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            TreeTerm::ProperList { items } => items.len() == 0,
            _ => false,
        }
    }
}

impl TreeTerm {
    fn get_vars(&self, vars: &mut PatternVariableSet) {
        match self {
            TreeTerm::Value(_) => (),
            TreeTerm::Var(ident) => {
                vars.insert(ident.clone());
            }
            TreeTerm::Field(_) => (),
            TreeTerm::Any(_) => (),
            TreeTerm::ImproperList { items } => {
                for item in items {
                    item.get_vars(vars);
                }
            }
            TreeTerm::ProperList { items } => {
                for item in items {
                    item.get_vars(vars);
                }
            }
        }
    }
}

impl Parse for TreeTerm {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![_]) {
            let us: Token![_] = input.parse()?;
            Ok(TreeTerm::Any(us))
        } else if (input.peek(Token![self]) || input.peek(Ident)) && input.peek2(Token![.]) {
            let field_access: FieldAccess = input.parse()?;
            Ok(TreeTerm::Field(field_access))
        } else if input.peek(Ident) {
            let id: Ident = input.parse()?;
            Ok(TreeTerm::Var(id))
        } else if input.peek(syn::Lit) {
            let value: Value = input.parse()?;
            Ok(TreeTerm::Value(value))
        } else if input.peek(Bracket) {
            let content;
            let _ = bracketed!(content in input);

            let mut items: Vec<InnerTreeTerm> = vec![];
            let mut is_proper = true;
            while !content.is_empty() {
                let term: InnerTreeTerm = content.parse()?;
                items.push(term);
                if content.peek(Token![,]) {
                    let _: Token![,] = content.parse()?;
                } else if content.peek(Token![|]) {
                    let _: Token![|] = content.parse()?;
                    let rest: InnerTreeTerm = content.parse()?;
                    items.push(rest);
                    is_proper = false;
                    break;
                }
            }

            if !content.is_empty() {
                return Err(content.error("Trailing characters"));
            }

            if is_proper {
                Ok(TreeTerm::ProperList { items })
            } else {
                Ok(TreeTerm::ImproperList { items })
            }
        } else {
            Err(input.error("Invalid tree-term."))
        }
    }
}

impl ToTokens for TreeTerm {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            TreeTerm::Value(value) => {
                let output = quote! { ::proto_vulcan::lterm::LTerm::from(#value) };
                output.to_tokens(tokens);
            }
            TreeTerm::Var(ident) => {
                let output = quote! { ::proto_vulcan::Upcast::to_super(&#ident) };
                output.to_tokens(tokens);
            }
            TreeTerm::Field(field_access) => {
                let field = &field_access.field;
                let output = quote! { ::proto_vulcan::Upcast::to_super(&#field) };
                output.to_tokens(tokens);
            }
            TreeTerm::Any(_) => {
                let output = quote! { ::proto_vulcan::lterm::LTerm::any() };
                output.to_tokens(tokens);
            }
            TreeTerm::ImproperList { items } => {
                let items: Vec<&InnerTreeTerm> = items.iter().collect();
                let output = quote! { ::proto_vulcan::lterm::LTerm::improper_from_array( &[ #(#items),* ] ) };
                output.to_tokens(tokens);
            }
            TreeTerm::ProperList { items } => {
                let items: Vec<&InnerTreeTerm> = items.iter().collect();
                let output;
                if items.is_empty() {
                    output = quote! { ::proto_vulcan::lterm::LTerm::empty_list() };
                } else {
                    output =
                        quote! { ::proto_vulcan::lterm::LTerm::from_array( &[ #(#items),* ] ) };
                }
                output.to_tokens(tokens);
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct Eq {
    left: Argument,
    eqeq: Token![==],
    right: Argument,
}

impl Parse for Eq {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Eq {
            left: input.parse()?,
            eqeq: input.parse()?,
            right: input.parse()?,
        })
    }
}

impl ToTokens for Eq {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let left = &self.left;
        let right = &self.right;
        let output = quote! { ::proto_vulcan::relation::eq::eq ( #left, #right ) };
        output.to_tokens(tokens)
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct Diseq {
    left: Argument,
    ne: Token![!=],
    right: Argument,
}

impl Parse for Diseq {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Diseq {
            left: input.parse()?,
            ne: input.parse()?,
            right: input.parse()?,
        })
    }
}

impl ToTokens for Diseq {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let left = &self.left;
        let right = &self.right;
        let output = quote! { ::proto_vulcan::relation::diseq::diseq ( #left, #right ) };
        output.to_tokens(tokens)
    }
}

#[derive(Clone)]
enum Clause {
    /// for x in coll { }
    For(For),
    /// project |x, y, z| { }
    Project(Project),
    // fngoal |state| { }
    FnGoal(FnGoal),
    /// |x, y, z| { }
    Fresh(Fresh),
    // x == y
    Eq(Eq),
    // x != y
    Diseq(Diseq),
    // true
    Succeed(syn::LitBool),
    // false
    Fail(syn::LitBool),
    // [ ]
    Conjunction(Conjunction),
    // $relation (param1, param2, ...)
    Relation(Relation),
    // closure { }
    Closure(Closure),
    // loop { }
    Loop(Loop),
    // $operator { }
    Operator(Operator),
    // $operator $term { pattern0 => body0, ...}
    PatternMatchOperator(PatternMatchOperator),
    // Expression that evaluates to Goal
    Expression(syn::Expr),
}

impl Parse for Clause {
    fn parse(input: ParseStream) -> Result<Self> {
        let maybe_ident = input.cursor().ident().map(|x| x.0.to_string());

        if input.peek(Ident)
            && input.peek2(Token![|])
            && maybe_ident == Some(String::from("project"))
        {
            let project: Project = input.parse()?;
            Ok(Clause::Project(project))
        } else if input.peek(Ident)
            && (input.peek2(Token![|]) || (input.peek2(Token![move]) && input.peek3(Token![|])))
            && maybe_ident == Some(String::from("fngoal"))
        {
            let fngoal: FnGoal = input.parse()?;
            Ok(Clause::FnGoal(fngoal))
        } else if input.peek(Ident)
            && input.peek2(Brace)
            && maybe_ident == Some(String::from("closure"))
        {
            let closure: Closure = input.parse()?;
            Ok(Clause::Closure(closure))
        } else if input.peek(Token![for]) {
            let for_clause: For = input.parse()?;
            Ok(Clause::For(for_clause))
        } else if input.peek(Token![loop]) && input.peek2(Brace) {
            let l: Loop = input.parse()?;
            Ok(Clause::Loop(l))
        } else if input.peek(Token![|]) {
            let fresh: Fresh = input.parse()?;
            Ok(Clause::Fresh(fresh))
        } else if let Ok(_) = Eq::parse(&input.fork()) {
            let eq: Eq = input.parse()?;
            Ok(Clause::Eq(eq))
        } else if let Ok(_) = Diseq::parse(&input.fork()) {
            let diseq: Diseq = input.parse()?;
            Ok(Clause::Diseq(diseq))
        } else if input.peek(syn::LitBool) {
            let b: syn::LitBool = input.parse()?;
            if b.value {
                Ok(Clause::Succeed(b))
            } else {
                Ok(Clause::Fail(b))
            }
        } else if input.peek(Bracket) {
            let conjunction: Conjunction = input.parse()?;
            Ok(Clause::Conjunction(conjunction))
        } else if input.peek(Ident) && input.peek2(Paren) {
            let relation: Relation = input.parse()?;
            Ok(Clause::Relation(relation))
        } else if input.peek(Ident) && input.peek2(Brace) {
            let operator: Operator = input.parse()?;
            Ok(Clause::Operator(operator))
        } else {
            if (input.peek(Ident) || input.peek(Token![match])) && input.peek3(Brace) {
                return Ok(PatternMatchOperator::parse(input)
                    .and_then(|operator| Ok(Clause::PatternMatchOperator(operator)))?);
            }
            let expr: syn::Expr = input.parse()?;
            Ok(Clause::Expression(expr))
        }
    }
}

impl ToTokens for Clause {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Clause::For(for_clause) => {
                for_clause.to_tokens(tokens);
            }
            Clause::Project(project) => {
                project.to_tokens(tokens);
            }
            Clause::FnGoal(fngoal) => {
                fngoal.to_tokens(tokens);
            }
            Clause::Fresh(fresh) => {
                fresh.to_tokens(tokens);
            }
            Clause::Eq(eq) => {
                eq.to_tokens(tokens);
            }
            Clause::Diseq(diseq) => {
                diseq.to_tokens(tokens);
            }
            Clause::Succeed(_) => {
                let output = quote! { ::proto_vulcan::relation::succeed() };
                output.to_tokens(tokens);
            }
            Clause::Fail(_) => {
                let output = quote! { ::proto_vulcan::relation::fail() };
                output.to_tokens(tokens);
            }
            Clause::Conjunction(conjunction) => {
                // When conjunction is not inside a non-conjunction an operator we can construct
                // an All-goal from it.
                let output =
                    quote! { ::proto_vulcan::operator::all::All::from_array( #conjunction ) };
                output.to_tokens(tokens);
            }
            Clause::Relation(relation) => {
                relation.to_tokens(tokens);
            }
            Clause::Closure(closure) => {
                closure.to_tokens(tokens);
            }
            Clause::Loop(l) => {
                l.to_tokens(tokens);
            }
            Clause::Operator(operator) => {
                operator.to_tokens(tokens);
            }
            Clause::PatternMatchOperator(operator) => {
                operator.to_tokens(tokens);
            }
            Clause::Expression(expr) => {
                expr.to_tokens(tokens);
            }
        }
    }
}

#[derive(Clone)]
struct ClauseInOperator(Clause);

impl Parse for ClauseInOperator {
    fn parse(input: ParseStream) -> Result<Self> {
        let clause: Clause = input.parse()?;
        Ok(ClauseInOperator(clause))
    }
}

impl ToTokens for ClauseInOperator {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match &self.0 {
            Clause::For(for_clause) => {
                let output = quote! { &[ #for_clause ] };
                output.to_tokens(tokens);
            }
            Clause::Project(project) => {
                let output = quote! { &[ #project ] };
                output.to_tokens(tokens);
            }
            Clause::FnGoal(fngoal) => {
                let output = quote! { &[ #fngoal ] };
                output.to_tokens(tokens);
            }
            Clause::Fresh(fresh) => {
                let output = quote! { &[ #fresh ] };
                output.to_tokens(tokens);
            }
            Clause::Eq(eq) => {
                let output = quote! { &[ #eq ] };
                output.to_tokens(tokens);
            }
            Clause::Diseq(diseq) => {
                let output = quote! { &[ #diseq ] };
                output.to_tokens(tokens);
            }
            Clause::Succeed(_) => {
                let output = quote! { &[ ::proto_vulcan::relation::succeed() ] };
                output.to_tokens(tokens);
            }
            Clause::Fail(_) => {
                let output = quote! { &[ ::proto_vulcan::relation::fail() ] };
                output.to_tokens(tokens);
            }
            Clause::Conjunction(conjunction) => {
                // When conjunction is inside an operator, we do not create All-goal, and instead
                // let the conjunction be represented as an array of goals.
                conjunction.to_tokens(tokens);
            }
            Clause::Relation(relation) => {
                let output = quote! { &[ #relation ] };
                output.to_tokens(tokens);
            }
            Clause::Closure(closure) => {
                let output = quote! { &[ #closure ] };
                output.to_tokens(tokens);
            }
            Clause::Loop(l) => {
                let output = quote! { &[ #l ] };
                output.to_tokens(tokens);
            }
            Clause::Operator(operator) => {
                let output = quote! { &[ #operator ] };
                output.to_tokens(tokens);
            }
            Clause::PatternMatchOperator(operator) => {
                let output = quote! { &[ #operator ] };
                output.to_tokens(tokens);
            }
            Clause::Expression(expr) => {
                let output = quote! { &[ #expr ]};
                output.to_tokens(tokens);
            }
        }
    }
}

#[proc_macro]
pub fn proto_vulcan(input: TokenStream) -> TokenStream {
    let clause = parse_macro_input!(input as Clause);

    let output = quote! {
        #clause
    };
    output.into()
}

#[proc_macro]
pub fn proto_vulcan_closure(input: TokenStream) -> TokenStream {
    let clause = parse_macro_input!(input as Clause);
    let closure = Closure::new(vec![clause]);

    let output = quote! {
        #closure
    };
    output.into()
}

#[proc_macro]
pub fn lterm(input: TokenStream) -> TokenStream {
    let term = parse_macro_input!(input as TreeTerm);

    let output = quote! {
        #term
    };
    output.into()
}

#[allow(dead_code)]
#[derive(Clone)]
struct Query {
    or1_token: Token![|],
    variables: Punctuated<TypedVariable, Token![,]>,
    or2_token: Token![|],
    brace_token: Brace,
    body: Punctuated<Clause, Token![,]>,
}

impl Parse for Query {
    fn parse(input: ParseStream) -> Result<Self> {
        let or1_token: Token![|] = input.parse()?;
        let mut variables = Punctuated::new();
        loop {
            if input.peek(Token![|]) {
                break;
            }
            let var: TypedVariable = input.parse()?;
            variables.push_value(var);
            if input.peek(Token![|]) {
                break;
            }
            let punct: Token![,] = input.parse()?;
            variables.push_punct(punct);
        }

        let or2_token: Token![|] = input.parse()?;

        let content;
        Ok(Query {
            or1_token,
            variables,
            or2_token,
            brace_token: braced!(content in input),
            body: content.parse_terminated(Clause::parse)?,
        })
    }
}

impl ToTokens for Query {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let query: Vec<Ident> = self.variables.iter().map(|x| &x.name).cloned().collect();
        let query_types: Vec<syn::Path> = self.variables.iter().map(|x| &x.path).cloned().collect();
        let body: Vec<&Clause> = self.body.iter().collect();

        let output = quote! {
            #(let #query: #query_types <_> = ::proto_vulcan::compound::CompoundTerm::new_var(stringify!(#query)); )*

            let __vars__ = vec![ #( ::proto_vulcan::Upcast::into_super(#query.clone()) ),* ];

            let goal = {
                let __query__ = ::proto_vulcan::lterm::LTerm::var("__query__");
                ::proto_vulcan::operator::fresh::Fresh::new(
                    vec![::std::clone::Clone::clone(&__query__)],
                    ::proto_vulcan::operator::all::All::from_array(&[
                        ::proto_vulcan::relation::eq::eq(
                            ::std::clone::Clone::clone(&__query__),
                            ::proto_vulcan::lterm::LTerm::from_array(&[#(::proto_vulcan::Upcast::to_super(&#query)),*]),
                     ),
                     ::proto_vulcan::operator::all::All::from_array(&[
                        #( #body ),*
                     ]),
                     ::proto_vulcan::state::reify(::std::clone::Clone::clone(&__query__)),
                    ]),
                )
            };

            use std::fmt;

            #[derive(Clone, Debug)]
            struct QResult<U: ::proto_vulcan::user::User> {
                #( #query: ::proto_vulcan::lresult::LResult<U>, )*
            }

            impl<U: ::proto_vulcan::user::User> ::proto_vulcan::query::QueryResult<U> for QResult<U> {
                fn from_vec(v: Vec<::proto_vulcan::lresult::LResult<U>>) -> QResult<U> {
                    let mut vi = v.into_iter();
                    QResult {
                        #( #query: vi.next().unwrap(), )*
                    }
                }
            }

            impl<U: User> fmt::Display for QResult<U> {
                #[allow(unused_variables, unused_assignments)]
                fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    let mut count = 0;
                    #( if count > 0 { writeln!(f, "")?; }  write!(f, "{}: {}", stringify!(#query), self.#query)?; count += 1; )*
                    write!(f, "")
                }
            }

            ::proto_vulcan::query::Query::<QResult<_>>::new(__vars__, goal)
        };

        output.to_tokens(tokens);
    }
}

#[proc_macro]
pub fn proto_vulcan_query(input: TokenStream) -> TokenStream {
    let query = parse_macro_input!(input as Query);
    let output = quote! {{
        #query
    }};
    output.into()
}

fn make_compound_modifications_to_path(path: &mut syn::Path) -> std::result::Result<(), Error> {
    match path.segments.iter_mut().last() {
        Some(last_segment) => match last_segment.arguments {
            syn::PathArguments::AngleBracketed(ref mut generic_arguments) => {
                for argument in generic_arguments.args.iter_mut() {
                    match argument {
                        syn::GenericArgument::Type(ty) => {
                            make_compound_modifications_to_type(ty)?;
                        }
                        _ => {
                            return Err(Error::new(argument.span(), "Invalid generic argument"));
                        }
                    }
                }

                return Ok(());
            }
            syn::PathArguments::None => {
                last_segment.arguments =
                    syn::PathArguments::AngleBracketed(syn::parse_quote! {<U>});
            }
            _ => {
                return Err(Error::new(
                    last_segment.arguments.span(),
                    "Invalid type argument",
                ));
            }
        },
        None => {
            return Err(Error::new(path.span(), "Invalid type argument"));
        }
    }
    Ok(())
}

fn make_compound_modifications_to_type(ty: &mut syn::Type) -> std::result::Result<(), Error> {
    match ty {
        syn::Type::Path(typepath) => {
            //let has_generic_args = has_generic_arguments(&typepath.path)?;
            make_compound_modifications_to_path(&mut typepath.path)?;
            *ty = syn::parse_quote! { #typepath };
        }
        _ => return Err(Error::new(ty.span(), "Invalid compound type")),
    }
    Ok(())
}

fn make_compound_modifications_to_itemstruct(
    itemstruct: &mut syn::ItemStruct,
) -> std::result::Result<(), Error> {
    if itemstruct.generics.params.is_empty() {
        let new_generics: syn::Generics = syn::parse_quote! {<U: ::proto_vulcan::user::User>};
        itemstruct.generics = new_generics;
    }
    for field in itemstruct.fields.iter_mut() {
        field.vis = syn::Visibility::Public(syn::VisPublic {
            pub_token: syn::parse_quote!(pub),
        });
        make_compound_modifications_to_type(&mut field.ty)?;
    }
    itemstruct.vis = syn::parse_quote!(pub);
    Ok(())
}

fn make_compound_unnamed_struct(itemstruct: syn::ItemStruct) -> TokenStream {
    let mut inner = itemstruct.clone();
    inner.ident = quote::format_ident!("_Inner{}", itemstruct.ident);

    let vis = &itemstruct.vis;
    let struct_name = itemstruct.ident.clone();
    let inner_ident = &inner.ident;
    let mod_name = quote::format_ident!("{}_compound", struct_name);
    let (impl_generics, type_generics, where_clause) = itemstruct.generics.split_for_impl();

    let field_indices: Vec<syn::Index> = itemstruct
        .fields
        .iter()
        .enumerate()
        .map(|(n, _)| syn::Index::from(n))
        .collect();

    let output = quote!(
        #[allow(non_snake_case)]
        #vis mod #mod_name {
            use super::*;
            #[derive(Clone, Eq)]
            #inner

            impl #impl_generics ::proto_vulcan::compound::CompoundObject #type_generics for #inner_ident #type_generics #where_clause {
                fn type_name(&self) -> &'static str {
                    stringify!(#struct_name)
                }

                fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn ::proto_vulcan::compound::CompoundObject<U>> + 'a> {
                    Box::new(vec![#(&self.#field_indices as &dyn ::proto_vulcan::compound::CompoundObject #type_generics),*].into_iter())
                }
            }

            impl #impl_generics ::proto_vulcan::compound::CompoundWalkStar #type_generics for #inner_ident #type_generics #where_clause {
                fn compound_walk_star(&self, smap: &::proto_vulcan::state::SMap #type_generics) -> Self {
                    #inner_ident(#(self.#field_indices.compound_walk_star(smap)),*)
                }
            }

            impl #impl_generics Into<#struct_name #type_generics> for #inner_ident #type_generics #where_clause {
                fn into(self) -> #struct_name #type_generics {
                    #struct_name {
                        inner: Into::<LTerm #type_generics>::into(self),
                    }
                }
            }

            impl #impl_generics Into<::proto_vulcan::lterm::LTerm #type_generics> for #inner_ident #type_generics #where_clause {
                fn into(self) -> ::proto_vulcan::lterm::LTerm #type_generics {
                    ::proto_vulcan::lterm::LTerm::from(::std::rc::Rc::new(self) as ::std::rc::Rc<dyn ::proto_vulcan::compound::CompoundObject #type_generics>)
                }
            }

            impl #impl_generics ::proto_vulcan::Downcast #type_generics for #inner_ident #type_generics #where_clause {
                type SubType = #struct_name #type_generics;
                fn into_sub(self) -> Self::SubType {
                    self.into()
                }
            }

            impl #impl_generics ::core::fmt::Debug for #inner_ident #type_generics #where_clause {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_tuple(f, stringify!(#struct_name));
                    #( let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &self.#field_indices); )*
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
            }

            impl #impl_generics ::std::hash::Hash for #inner_ident #type_generics #where_clause {
                fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                    #( ::std::hash::Hash::hash(&self.#field_indices, state); )*
                }
            }

            impl #impl_generics ::std::cmp::PartialEq for #inner_ident #type_generics #where_clause {
                fn eq(&self, other: &Self) -> bool {
                    #( ::std::cmp::PartialEq::eq(&self.#field_indices, &other.#field_indices) &&)* true
                }
            }
        }

        #[derive(Clone, Eq)]
        #vis struct #struct_name #impl_generics {
            inner: LTerm #type_generics,
        }

        impl #impl_generics ::std::fmt::Debug for #struct_name #type_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                self.inner.fmt(f)
            }
        }

        impl #impl_generics ::std::hash::Hash for #struct_name #type_generics #where_clause {
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                ::std::hash::Hash::hash(&self.inner, state);
            }
        }

        impl #impl_generics ::std::cmp::PartialEq for #struct_name #type_generics #where_clause {
            fn eq(&self, other: &Self) -> bool {
                ::std::cmp::PartialEq::eq(&self.inner, &other.inner)
            }
        }

        #[automatically_derived]
        impl #impl_generics ::proto_vulcan::compound::CompoundTerm #type_generics for #struct_name #type_generics #where_clause {
            fn new_var(name: &'static str) -> #struct_name #type_generics {
                #struct_name {
                    inner: LTerm::var(name),
                }
            }

            fn new_wildcard() -> #struct_name #type_generics {
                #struct_name {
                    inner: LTerm::any(),
                }
            }

            fn new_none() -> #struct_name #type_generics {
                #struct_name {
                    inner: LTerm::empty_list(),
                }
            }
        }

        impl #impl_generics ::proto_vulcan::compound::CompoundObject #type_generics for #struct_name #type_generics #where_clause {
            fn type_name(&self) -> &'static str {
                stringify!(#struct_name)
            }

            fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn ::proto_vulcan::compound::CompoundObject #type_generics> + 'a> {
                self.inner.children()
            }

            fn as_term(&self) -> Option<&LTerm<U>> {
                Some(&self.inner)
            }
        }

        impl #impl_generics ::proto_vulcan::compound::CompoundWalkStar #type_generics for #struct_name #type_generics #where_clause {
            fn compound_walk_star(&self, smap: &::proto_vulcan::state::SMap #type_generics) -> Self {
                #struct_name {
                    inner: self.inner.compound_walk_star(smap),
                }
            }
        }

        #[automatically_derived]
        impl #impl_generics Into<::proto_vulcan::lterm::LTerm #type_generics> for #struct_name #type_generics #where_clause {
            fn into(self) -> LTerm #type_generics {
                self.inner
            }
        }

        impl #impl_generics ::proto_vulcan::Upcast<U, ::proto_vulcan::lterm::LTerm #type_generics> for #struct_name #type_generics #where_clause {
            #[inline]
            fn to_super<K: ::std::borrow::Borrow<Self>>(k: &K) -> ::proto_vulcan::lterm::LTerm #type_generics {
                Into::into(::std::clone::Clone::clone(k.borrow()))
            }

            #[inline]
            fn into_super(self) -> ::proto_vulcan::lterm::LTerm #type_generics {
                Into::into(self)
            }
        }

        impl #impl_generics ::proto_vulcan::Downcast #type_generics for #struct_name #type_generics #where_clause {
            type SubType = Self;
            fn into_sub(self) -> Self::SubType {
                self.into()
            }
        }
    );
    output.into()
}

fn make_compound_named_struct(itemstruct: syn::ItemStruct) -> TokenStream {
    let mut inner = itemstruct.clone();
    inner.ident = quote::format_ident!("_Inner{}", itemstruct.ident);

    let vis = &itemstruct.vis;
    let struct_name = itemstruct.ident.clone();
    let inner_ident = &inner.ident;
    let mod_name = quote::format_ident!("{}_compound", struct_name);
    let (impl_generics, type_generics, where_clause) = itemstruct.generics.split_for_impl();

    let field_names: Vec<syn::Ident> = itemstruct
        .fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap().clone())
        .collect();

    let output = quote!(
        #[allow(non_snake_case)]
        #vis mod #mod_name {
            use super::*;
            #[derive(Clone, Eq)]
            #inner

            impl #impl_generics ::proto_vulcan::compound::CompoundObject #type_generics for #inner_ident #type_generics #where_clause {
                fn type_name(&self) -> &'static str {
                    stringify!(#struct_name)
                }

                fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn ::proto_vulcan::compound::CompoundObject<U>> + 'a> {
                    Box::new(vec![#(&self.#field_names as &dyn ::proto_vulcan::compound::CompoundObject #type_generics),*].into_iter())
                }
            }

            impl #impl_generics ::proto_vulcan::compound::CompoundWalkStar #type_generics for #inner_ident #type_generics #where_clause {
                fn compound_walk_star(&self, smap: &::proto_vulcan::state::SMap #type_generics) -> Self {
                    #inner_ident { #( #field_names: self.#field_names.compound_walk_star(smap)),* }
                }
            }

            impl #impl_generics Into<#struct_name #type_generics> for #inner_ident #type_generics #where_clause {
                fn into(self) -> #struct_name #type_generics {
                    #struct_name {
                        inner: Into::<LTerm #type_generics>::into(self),
                    }
                }
            }

            impl #impl_generics Into<::proto_vulcan::lterm::LTerm #type_generics> for #inner_ident #type_generics #where_clause {
                fn into(self) -> ::proto_vulcan::lterm::LTerm #type_generics {
                    ::proto_vulcan::lterm::LTerm::from(::std::rc::Rc::new(self) as ::std::rc::Rc<dyn ::proto_vulcan::compound::CompoundObject #type_generics>)
                }
            }

            impl #impl_generics ::proto_vulcan::Downcast #type_generics for #inner_ident #type_generics #where_clause {
                type SubType = #struct_name #type_generics;
                fn into_sub(self) -> Self::SubType {
                    self.into()
                }
            }

            impl #impl_generics ::core::fmt::Debug for #inner_ident #type_generics #where_clause {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_struct(f, stringify!(#struct_name));
                    #(
                        let _= ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            stringify!(#field_names),
                            &self.#field_names,
                        );
                    )*
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }

            impl #impl_generics ::std::hash::Hash for #inner_ident #type_generics #where_clause {
                fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                    #( ::std::hash::Hash::hash(&self.#field_names, state); )*
                }
            }

            impl #impl_generics ::std::cmp::PartialEq for #inner_ident #type_generics #where_clause {
                fn eq(&self, other: &Self) -> bool {
                    #( ::std::cmp::PartialEq::eq(&self.#field_names, &other.#field_names) &&)* true
                }
            }
        }

        #[derive(Clone, Eq)]
        #vis struct #struct_name #impl_generics {
            inner: LTerm #type_generics,
        }

        impl #impl_generics ::std::fmt::Debug for #struct_name #type_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                self.inner.fmt(f)
            }
        }

        impl #impl_generics ::std::hash::Hash for #struct_name #type_generics #where_clause {
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                ::std::hash::Hash::hash(&self.inner, state);
            }
        }

        impl #impl_generics ::std::cmp::PartialEq for #struct_name #type_generics #where_clause {
            fn eq(&self, other: &Self) -> bool {
                ::std::cmp::PartialEq::eq(&self.inner, &other.inner)
            }
        }

        #[automatically_derived]
        impl #impl_generics ::proto_vulcan::compound::CompoundTerm #type_generics for #struct_name #type_generics #where_clause {
            fn new_var(name: &'static str) -> #struct_name #type_generics {
                #struct_name {
                    inner: LTerm::var(name),
                }
            }

            fn new_wildcard() -> #struct_name #type_generics {
                #struct_name {
                    inner: LTerm::any(),
                }
            }

            fn new_none() -> #struct_name #type_generics {
                #struct_name {
                    inner: LTerm::empty_list(),
                }
            }
        }

        impl #impl_generics ::proto_vulcan::compound::CompoundObject #type_generics for #struct_name #type_generics #where_clause {
            fn type_name(&self) -> &'static str {
                stringify!(#struct_name)
            }

            fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn ::proto_vulcan::compound::CompoundObject #type_generics> + 'a> {
                self.inner.children()
            }

            fn as_term(&self) -> Option<&LTerm<U>> {
                Some(&self.inner)
            }
        }

        impl #impl_generics ::proto_vulcan::compound::CompoundWalkStar #type_generics for #struct_name #type_generics #where_clause {
            fn compound_walk_star(&self, smap: &::proto_vulcan::state::SMap #type_generics) -> Self {
                #struct_name {
                    inner: self.inner.compound_walk_star(smap),
                }
            }
        }

        #[automatically_derived]
        impl #impl_generics Into<::proto_vulcan::lterm::LTerm #type_generics> for #struct_name #type_generics #where_clause {
            fn into(self) -> LTerm #type_generics {
                self.inner
            }
        }

        impl #impl_generics ::proto_vulcan::Upcast<U, ::proto_vulcan::lterm::LTerm #type_generics> for #struct_name #type_generics #where_clause {
            #[inline]
            fn to_super<K: ::std::borrow::Borrow<Self>>(k: &K) -> ::proto_vulcan::lterm::LTerm #type_generics {
                Into::into(::std::clone::Clone::clone(k.borrow()))
            }

            #[inline]
            fn into_super(self) -> ::proto_vulcan::lterm::LTerm #type_generics {
                Into::into(self)
            }
        }

        impl #impl_generics ::proto_vulcan::Downcast #type_generics for #struct_name #type_generics #where_clause {
            type SubType = Self;
            fn into_sub(self) -> Self::SubType {
                self.into()
            }
        }
    );
    output.into()
}

fn make_compound_struct(mut itemstruct: syn::ItemStruct) -> TokenStream {
    // Add generics and where necessary
    match make_compound_modifications_to_itemstruct(&mut itemstruct) {
        Ok(()) => (),
        Err(error) => return error.to_compile_error().into(),
    }

    match itemstruct.fields {
        syn::Fields::Unnamed(_) => make_compound_unnamed_struct(itemstruct),
        syn::Fields::Named(_) => make_compound_named_struct(itemstruct),
        syn::Fields::Unit => make_compound_named_struct(itemstruct),
    }
}

#[proc_macro_attribute]
pub fn compound(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as syn::Item);

    match item {
        //syn::Item::Enum(item_enum) => return make_compound_enum(item_enum),
        syn::Item::Struct(item_struct) => return make_compound_struct(item_struct),
        _ => {
            return syn::Error::new(item.span(), "Compound attribute requires struct or enum.")
                .to_compile_error()
                .into();
        }
    }
}
