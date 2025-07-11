use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::{Error, Ident, Token};

#[allow(dead_code)]
#[derive(Clone)]
pub struct FnGoal {
    fngoal: Ident,
    m: Option<Token![move]>,
    or1_token: Token![|],
    engine: Ident,
    state: Ident,
    or2_token: Token![|],
    body: syn::Block,
}

impl Parse for FnGoal {
    fn parse(input: ParseStream) -> Result<Self> {
        let fngoal: Ident = input.parse()?;
        if fngoal.to_string().as_str() != "fngoal" {
            return Err(Error::new(fngoal.span(), "Identifier \"fngoal\" expected"));
        }

        let m = if input.peek(Token![move]) {
            Some(input.parse()?)
        } else {
            None
        };

        let or1_token: Token![|] = input.parse()?;
        let engine: Ident = input.parse()?;
        let _: Token![,] = input.parse()?;
        let state: Ident = input.parse()?;
        let or2_token: Token![|] = input.parse()?;

        Ok(FnGoal {
            fngoal,
            m,
            or1_token,
            engine,
            state,
            or2_token,
            body: input.parse()?,
        })
    }
}

impl ToTokens for FnGoal {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let m = &self.m;
        let engine = &self.engine;
        let state = &self.state;
        let body: &syn::Block = &self.body;
        let output = quote! {{
            ::proto_vulcan::operator::fngoal::FnGoal::new(Box::new(#m |#engine, #state| { #body } ))
        }};
        output.to_tokens(tokens);
    }
}
