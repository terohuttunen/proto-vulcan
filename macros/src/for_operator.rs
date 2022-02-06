use crate::ClauseInOperator;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Brace;
use syn::{braced, Ident, Token};

#[allow(dead_code)]
#[derive(Clone)]
pub struct For {
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
            ::proto_vulcan::operator::everyg(::proto_vulcan::operator::ForOperatorParam::new(
                ::std::clone::Clone::clone(#coll),
                Box::new(|#pattern| ::proto_vulcan::GoalCast::cast_into(::proto_vulcan::operator::conj::InferredConj::from_conjunctions(&[ #( #body ),* ]))),
            ))
        });
        output.to_tokens(tokens);
    }
}
