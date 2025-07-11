use quote::ToTokens;
use syn::parse::{Parse, ParseStream, Result};
use syn::Error;

#[derive(Clone, Debug)]
pub enum Literal {
    Bool(syn::LitBool),
    Number(syn::LitInt),
    Char(syn::LitChar),
    String(syn::LitStr),
}

impl Parse for Literal {
    fn parse(input: ParseStream) -> Result<Self> {
        let lit: syn::Lit = input.parse()?;
        match lit {
            syn::Lit::Str(s) => Ok(Literal::String(s)),
            syn::Lit::Char(c) => Ok(Literal::Char(c)),
            syn::Lit::Int(n) => Ok(Literal::Number(n)),
            syn::Lit::Bool(b) => Ok(Literal::Bool(b)),
            _ => Err(Error::new(lit.span(), "Invalid literal")),
        }
    }
}

impl ToTokens for Literal {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Literal::Bool(b) => b.to_tokens(tokens),
            Literal::Number(n) => n.to_tokens(tokens),
            Literal::Char(c) => c.to_tokens(tokens),
            Literal::String(s) => s.to_tokens(tokens),
        }
    }
}
