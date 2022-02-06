use crate::Argument;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::Token;

#[allow(dead_code)]
#[derive(Clone)]
pub struct Diseq {
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
