use crate::Clause;
use crate::TypedVariable;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Brace;
use syn::{braced, Ident, Token};

#[allow(dead_code)]
#[derive(Clone)]
pub struct Fresh {
    or1_token: Token![|],
    variables: Punctuated<TypedVariable, Token![,]>,
    or2_token: Token![|],
    brace_token: Brace,
    body: Punctuated<Clause, Token![,]>,
}

impl Parse for Fresh {
    fn parse(input: ParseStream) -> Result<Self> {
        let or1_token: Token![|] = input.parse()?;
        let mut variables = Punctuated::new();
        loop {
            if input.peek(Token![|]) {
                break;
            }
            let var: TypedVariable = input.parse()?;
            variables.push_value(var);
            if input.peek(Token![|]) {
                break;
            }
            let punct: Token![,] = input.parse()?;
            variables.push_punct(punct);
        }
        let or2_token: Token![|] = input.parse()?;

        let content;
        Ok(Fresh {
            or1_token,
            variables,
            or2_token,
            brace_token: braced!(content in input),
            body: content.parse_terminated(Clause::parse)?,
        })
    }
}

impl ToTokens for Fresh {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let variables: Vec<Ident> = self.variables.iter().map(|x| &x.name).cloned().collect();
        let variable_types: Vec<syn::Path> =
            self.variables.iter().map(|x| &x.path).cloned().collect();
        let body: Vec<&Clause> = self.body.iter().collect();
        let output = quote! {{
            #( let #variables: #variable_types <_, _> = ::proto_vulcan::compound::CompoundTerm::new_var(stringify!(#variables)); )*
            ::proto_vulcan::operator::fresh::Fresh::new(vec![ #( ::proto_vulcan::Upcast::to_super(&#variables) ),* ],
                ::proto_vulcan::GoalCast::cast_into(
                    ::proto_vulcan::operator::conj::InferredConj::from_array(&[ #( ::proto_vulcan::GoalCast::cast_into( #body ) ),* ]))
                )
        }};
        output.to_tokens(tokens);
    }
}
