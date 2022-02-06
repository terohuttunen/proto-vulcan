use crate::Clause;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::{braced, Ident};

#[allow(dead_code)]
#[derive(Clone)]
pub struct Closure {
    body: Vec<Clause>,
}

impl Closure {
    pub fn new(body: Vec<Clause>) -> Closure {
        Closure { body }
    }
}

impl Parse for Closure {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        if name != String::from("closure") {
            return Err(input.error("Expected \"closure\""));
        }
        let content;
        let _ = braced!(content in input);
        let mut body = vec![];
        for clause in content.parse_terminated::<Clause, Clause>(Clause::parse)? {
            body.push(clause);
        }
        Ok(Closure { body })
    }
}

impl ToTokens for Closure {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let body: Vec<&Clause> = self.body.iter().collect();
        let output = quote! {{
            ::proto_vulcan::operator::closure::Closure::new(
                ::proto_vulcan::operator::ClosureOperatorParam::new(
                    Box::new(move || ::proto_vulcan::GoalCast::cast_into(::proto_vulcan::operator::conj::InferredConj::from_array( &[ #( ::proto_vulcan::GoalCast::cast_into( #body ) ),* ] ) ))
                )
            )
        }};
        output.to_tokens(tokens);
    }
}
