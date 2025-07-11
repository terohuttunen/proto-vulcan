use crate::Clause;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Bracket;
use syn::{bracketed, Token};

#[allow(dead_code)]
#[derive(Clone)]
pub struct Conjunction {
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
        let output = quote! { &[ #( ::proto_vulcan::GoalCast::cast_into(#body) ),* ] };
        output.to_tokens(tokens)
    }
}
