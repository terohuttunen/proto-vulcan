use crate::Argument;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Paren;
use syn::{parenthesized, Ident, Token};

#[allow(dead_code)]
#[derive(Clone)]
pub struct Relation {
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
