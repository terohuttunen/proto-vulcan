use crate::{PatternVariableSet, Value};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Bracket;
use syn::{bracketed, Ident, Token};

#[derive(Clone, Debug)]
pub struct FieldAccess {
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
pub struct InnerTreeTerm(TreeTerm);

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
pub enum TreeTerm {
    Value(Value),
    Var(Ident),
    Field(FieldAccess),
    Any(Token![_]),
    ImproperList { items: Vec<InnerTreeTerm> },
    ProperList { items: Vec<InnerTreeTerm> },
}

impl TreeTerm {
    pub fn is_empty(&self) -> bool {
        match self {
            TreeTerm::ProperList { items } => items.len() == 0,
            _ => false,
        }
    }
}

impl TreeTerm {
    pub fn get_vars(&self, vars: &mut PatternVariableSet) {
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
