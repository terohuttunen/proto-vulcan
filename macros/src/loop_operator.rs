use crate::ClauseInOperator;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Brace;
use syn::{braced, Token};

#[allow(dead_code)]
#[derive(Clone)]
pub struct Loop {
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
            ::proto_vulcan::operator::anyo::anyo(::proto_vulcan::operator::OperatorParam::new( &[ #( #body ),* ] ))
        }};
        output.to_tokens(tokens);
    }
}
