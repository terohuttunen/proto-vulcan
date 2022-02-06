use crate::Argument;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::Token;

#[allow(dead_code)]
#[derive(Clone)]
pub struct Eq {
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
