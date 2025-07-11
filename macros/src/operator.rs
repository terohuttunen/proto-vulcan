use crate::ClauseInOperator;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Brace;
use syn::{braced, Ident, Token};

#[allow(dead_code)]
#[derive(Clone)]
pub struct Operator {
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
            quote! { #name ( ::proto_vulcan::operator::OperatorParam::new( &[ #( #body ),* ] ) )};
        output.to_tokens(tokens);
    }
}
