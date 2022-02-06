use crate::Clause;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Brace;
use syn::{braced, Error, Ident, Token};

#[allow(dead_code)]
#[derive(Clone)]
pub struct Project {
    project: Ident,
    or1_token: Token![|],
    variables: Punctuated<Ident, Token![,]>,
    or2_token: Token![|],
    brace_token: Brace,
    body: Punctuated<Clause, Token![,]>,
}

impl Parse for Project {
    fn parse(input: ParseStream) -> Result<Self> {
        let project: Ident = input.parse()?;
        if project.to_string().as_str() != "project" {
            return Err(Error::new(
                project.span(),
                "Identifier \"project\" expected",
            ));
        }

        let or1_token: Token![|] = input.parse()?;
        let mut variables = Punctuated::new();
        loop {
            if input.peek(Token![|]) {
                break;
            }
            let var: Ident = input.parse()?;
            variables.push_value(var);
            if input.peek(Token![|]) {
                break;
            }
            let punct: Token![,] = input.parse()?;
            variables.push_punct(punct);
        }
        let or2_token: Token![|] = input.parse()?;

        let content;
        Ok(Project {
            project,
            or1_token,
            variables,
            or2_token,
            brace_token: braced!(content in input),
            body: content.parse_terminated(Clause::parse)?,
        })
    }
}

impl ToTokens for Project {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let variables: Vec<&Ident> = self.variables.iter().collect();
        let body: Vec<&Clause> = self.body.iter().collect();
        let output = quote! {{
            #( let #variables = ::proto_vulcan::lterm::LTerm::projection(::std::clone::Clone::clone(&#variables)); )*
            ::proto_vulcan::operator::project::Project::new(
                vec![ #( ::std::clone::Clone::clone(&#variables) ),* ],
                ::proto_vulcan::GoalCast::cast_into(
                    ::proto_vulcan::operator::conj::InferredConj::from_conjunctions(&[ #( &[ ::proto_vulcan::GoalCast::cast_into( #body ) ] ),* ])
                )
            )
        }};
        output.to_tokens(tokens);
    }
}
