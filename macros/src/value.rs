use quote::ToTokens;
use syn::parse::{Parse, ParseStream, Result};
use syn::Error;

#[derive(Clone, Debug)]
pub enum Value {
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
