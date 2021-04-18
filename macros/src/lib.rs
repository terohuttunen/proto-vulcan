extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
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
            #( let #variables = crate::lterm::LTerm::projection(::std::clone::Clone::clone(&#variables)); )*
            crate::operator::project::project(crate::operator::ProjectOperatorParam {
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
            crate::operator::fngoal::FnGoal::new(Box::new(#m |#engine, #state| { #body } ))
        }};
        output.to_tokens(tokens);
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct Fresh {
    or1_token: Token![|],
    variables: Punctuated<Ident, Token![,]>,
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
        let variables: Vec<&Ident> = self.variables.iter().collect();
        let body: Vec<&Clause> = self.body.iter().collect();
        let output = quote! {{
            #( let #variables = crate::lterm::LTerm::var(stringify!(#variables)); )*
            crate::operator::fresh::Fresh::new(vec![ #( ::std::clone::Clone::clone(&#variables) ),* ],
                crate::operator::all::All::from_array(&[ #( #body ),* ]))
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

#[derive(Clone)]
enum Argument {
    TreeTerm(TreeTerm),
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
            // Try parsing TreeTerm
            if let Ok(term) = input.parse() {
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
            Argument::Quoted(expr) => {
                expr.to_tokens(tokens);
            }
            Argument::Expr(expr) => {
                let output = quote! { crate::lterm::LTerm::from(#expr) };
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
            crate::operator::closure::Closure::new(crate::operator::ClosureOperatorParam {f: Box::new(move || crate::operator::all::All::from_array( &[ #( #body ),* ] ) )})
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
            crate::operator::anyo::anyo(crate::operator::OperatorParam { body: &[ #( #body ),* ] })
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
        let output = quote! { #name ( crate::operator::OperatorParam { body: &[ #( #body ),* ] } )};
        output.to_tokens(tokens);
    }
}

#[derive(Clone)]
struct PatternArm {
    patterns: Vec<TreeTerm>,
    arrow: Token![=>],
    brace_token: Option<Brace>,
    body: Punctuated<Clause, Token![,]>,
}

impl Parse for PatternArm {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut patterns = vec![];
        loop {
            let pattern: TreeTerm = input.parse()?;
            patterns.push(pattern);

            if input.peek(Token![|]) {
                let _: Token![|] = input.parse()?;
            } else if input.peek(Token![=>]) {
                break;
            }
        }

        for pattern in patterns.iter() {
            for var_ident in pattern.get_vars().iter() {
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

        let mut patterns: Vec<TreeTerm> = vec![];
        let mut vars: Vec<Vec<Ident>> = vec![];
        let mut clauses: Vec<Punctuated<Clause, Token![,]>> = vec![];
        for arm in self.arms.iter() {
            // Repeat |-expression patterns with multiple single pattern entries
            for pattern in arm.patterns.iter() {
                patterns.push(pattern.clone());
                vars.push(pattern.get_vars());
                clauses.push(arm.body.clone());
            }
        }

        let output = if name.to_string() == "match" {
            quote! {
                crate::operator::matche ( crate::operator::PatternMatchOperatorParam {
                    arms: &[ #( &{
                        // Define alias for the `term` so that pattern-variables do not redefine it
                        // before the equality-relation with pattern is created.
                        let __term__ = #term;
                        // Define new variables found in the pattern
                        #( let #vars = LTerm::var(stringify!(#vars)); )*
                        [crate::relation::eq(__term__, #patterns), #clauses ]
                    } ),* ],
                })
            }
        } else {
            quote! {
                #name ( crate::operator::PatternMatchOperatorParam {
                    arms: &[ #( &{
                        // Define alias for the `term` so that pattern-variables do not redefine it
                        // before the equality-relation with pattern is created.
                        let __term__ = #term;
                        // Define new variables found in the pattern
                        #( let #vars = LTerm::var(stringify!(#vars)); )*
                        [crate::relation::eq(__term__, #patterns), #clauses ]
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
            crate::operator::everyg(crate::operator::ForOperatorParam {
                coll: ::std::clone::Clone::clone(#coll),
                g: Box::new(|#pattern| crate::operator::all::All::from_conjunctions(&[ #( #body ),* ])),
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

/// TreeTerm within a TreeTerm
#[derive(Clone, Debug)]
struct InnerTreeTerm(TreeTerm);

impl InnerTreeTerm {
    fn get_vars(&self) -> Vec<Ident> {
        self.0.get_vars()
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
                let output = quote! { crate::lterm::LTerm::from(#value) };
                output.to_tokens(tokens);
            }
            TreeTerm::Var(ident) => {
                // For InnerTreeTerms any references to variables must be cloned
                let output = quote! { ::std::clone::Clone::clone(&#ident) };
                output.to_tokens(tokens);
            }
            TreeTerm::Any(_) => {
                let output = quote! { crate::lterm::LTerm::any() };
                output.to_tokens(tokens);
            }
            TreeTerm::ImproperList { items } => {
                let items: Vec<&InnerTreeTerm> = items.iter().collect();
                let output =
                    quote! { crate::lterm::LTerm::improper_from_array( &[ #(#items),* ] ) };
                output.to_tokens(tokens);
            }
            TreeTerm::ProperList { items } => {
                let items: Vec<&InnerTreeTerm> = items.iter().collect();
                let output = quote! { crate::lterm::LTerm::from_array( &[ #(#items),* ] ) };
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
    Any(Token![_]),
    ImproperList { items: Vec<InnerTreeTerm> },
    ProperList { items: Vec<InnerTreeTerm> },
}

impl TreeTerm {
    fn get_vars(&self) -> Vec<Ident> {
        match self {
            TreeTerm::Value(_) => vec![],
            TreeTerm::Var(ident) => vec![ident.clone()],
            TreeTerm::Any(_) => vec![],
            TreeTerm::ImproperList { items } => {
                let mut variables = vec![];
                for item in items {
                    variables.append(&mut item.get_vars());
                }
                variables.sort();
                variables.dedup();
                variables
            }
            TreeTerm::ProperList { items } => {
                let mut variables = vec![];
                for item in items {
                    variables.append(&mut item.get_vars());
                }
                variables.sort();
                variables.dedup();
                variables
            }
        }
    }
}

impl Parse for TreeTerm {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![_]) {
            let us: Token![_] = input.parse()?;
            Ok(TreeTerm::Any(us))
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
                let output = quote! { crate::lterm::LTerm::from(#value) };
                output.to_tokens(tokens);
            }
            TreeTerm::Var(ident) => {
                let output = quote! { ::std::clone::Clone::clone(&#ident) };
                output.to_tokens(tokens);
            }
            TreeTerm::Any(_) => {
                let output = quote! { crate::lterm::LTerm::any() };
                output.to_tokens(tokens);
            }
            TreeTerm::ImproperList { items } => {
                let items: Vec<&InnerTreeTerm> = items.iter().collect();
                let output =
                    quote! { crate::lterm::LTerm::improper_from_array( &[ #(#items),* ] ) };
                output.to_tokens(tokens);
            }
            TreeTerm::ProperList { items } => {
                let items: Vec<&InnerTreeTerm> = items.iter().collect();
                let output = quote! { crate::lterm::LTerm::from_array( &[ #(#items),* ] ) };
                output.to_tokens(tokens);
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct Eq {
    left: TreeTerm,
    eqeq: Token![==],
    right: TreeTerm,
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
        let output = quote! { crate::relation::eq::eq ( #left, #right ) };
        output.to_tokens(tokens)
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct Diseq {
    left: TreeTerm,
    ne: Token![!=],
    right: TreeTerm,
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
        let output = quote! { crate::relation::diseq::diseq ( #left, #right ) };
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
        } else if input.peek2(Token![==]) {
            let eq: Eq = input.parse()?;
            Ok(Clause::Eq(eq))
        } else if input.peek2(Token![!=]) {
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
                let output = quote! { crate::relation::succeed() };
                output.to_tokens(tokens);
            }
            Clause::Fail(_) => {
                let output = quote! { crate::relation::fail() };
                output.to_tokens(tokens);
            }
            Clause::Conjunction(conjunction) => {
                // When conjunction is not inside a non-conjunction an operator we can construct
                // an All-goal from it.
                let output = quote! { crate::operator::all::All::from_array( #conjunction ) };
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
                let output = quote! { &[ crate::relation::succeed() ] };
                output.to_tokens(tokens);
            }
            Clause::Fail(_) => {
                let output = quote! { &[ crate::relation::fail() ] };
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
    variables: Punctuated<Ident, Token![,]>,
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
        let query: Vec<&Ident> = self.variables.iter().collect();
        let body: Vec<&Clause> = self.body.iter().collect();

        let output = quote! {
            #(let #query = LTerm::var(stringify!(#query));)*

            let __vars__ = vec![ #( #query.clone() ),* ];

            let goal = {
                let __query__ = crate::lterm::LTerm::var("__query__");
                crate::operator::fresh::Fresh::new(
                    vec![::std::clone::Clone::clone(&__query__)],
                    crate::operator::all::All::from_array(&[
                        crate::relation::eq::eq(
                            ::std::clone::Clone::clone(&__query__),
                            crate::lterm::LTerm::from_array(&[#(::std::clone::Clone::clone(&#query)),*]),
                     ),
                     crate::operator::all::All::from_array(&[
                        #( #body ),*
                     ]),
                     crate::state::reify(::std::clone::Clone::clone(&__query__)),
                    ]),
                )
            };

            use crate::user::User;
            use std::fmt;
            use crate::lresult::LResult;
            use crate::lterm::LTerm;
            use crate::query::QueryResult;

            #[derive(Clone, Debug)]
            struct QResult<U: User> {
                #( #query: LResult<U>, )*
            }

            impl<U: User> QueryResult<U> for QResult<U> {
                fn from_vec(v: Vec<LResult<U>>) -> QResult<U> {
                    let mut vi = v.into_iter();
                    QResult {
                        #( #query: vi.next().unwrap(), )*
                    }
                }
            }

            impl<U: User> fmt::Display for QResult<U> {
                #[allow(unused_variables, unused_assignments)]
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    let mut count = 0;
                    #( if count > 0 { writeln!(f, "")?; }  write!(f, "{}: {}", stringify!(#query), self.#query)?; count += 1; )*
                    write!(f, "")
                }
            }

            crate::query::Query::<QResult<_>>::new(__vars__, goal)
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
