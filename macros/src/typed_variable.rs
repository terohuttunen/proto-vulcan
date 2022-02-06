use syn::parse::{Parse, ParseStream, Result};
use syn::{Ident, Token};

#[derive(Clone)]
pub struct TypedVariable {
    pub name: Ident,
    pub path: syn::Path,
}

impl Parse for TypedVariable {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let path;
        if input.peek(Token![:]) {
            let _: Token![:] = input.parse()?;
            path = input.parse()?;
        } else {
            path = syn::parse_quote!(::proto_vulcan::lterm::LTerm);
        }
        Ok(TypedVariable { name, path })
    }
}
