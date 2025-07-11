use crate::{Clause, TypedVariable};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::Brace;
use syn::{braced, Ident, Token};

#[allow(dead_code)]
#[derive(Clone)]
pub struct Query {
    or1_token: Token![|],
    variables: Punctuated<TypedVariable, Token![,]>,
    or2_token: Token![|],
    brace_token: Brace,
    body: Punctuated<Clause, Token![,]>,
}

impl Parse for Query {
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
        Ok(Query {
            or1_token,
            variables,
            or2_token,
            brace_token: braced!(content in input),
            body: content.parse_terminated(Clause::parse)?,
        })
    }
}

impl ToTokens for Query {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let query: Vec<Ident> = self.variables.iter().map(|x| &x.name).cloned().collect();
        let query_types: Vec<syn::Path> = self.variables.iter().map(|x| &x.path).cloned().collect();
        let body: Vec<&Clause> = self.body.iter().collect();

        let output = quote! {
            #(let #query: #query_types <_, _> = ::proto_vulcan::compound::CompoundTerm::new_var(stringify!(#query)); )*

            let __vars__ = vec![ #( ::proto_vulcan::Upcast::into_super(#query.clone()) ),* ];

            let goal = {
                let __query__ = ::proto_vulcan::lterm::LTerm::var("__query__");
                ::proto_vulcan::GoalCast::cast_into(
                    ::proto_vulcan::operator::fresh::Fresh::new(
                        vec![::std::clone::Clone::clone(&__query__)],
                        ::proto_vulcan::GoalCast::cast_into(
                            ::proto_vulcan::operator::conj::InferredConj::from_array(&[
                                ::proto_vulcan::GoalCast::cast_into(
                                    ::proto_vulcan::relation::eq::eq(
                                        ::std::clone::Clone::clone(&__query__),
                                        ::proto_vulcan::lterm::LTerm::from_array(&[#(::proto_vulcan::Upcast::to_super(&#query)),*]),

                                    )
                                ),
                                ::proto_vulcan::operator::conj::Conj::from_array(&[
                                    #( ::proto_vulcan::GoalCast::cast_into( #body ) ),*
                                ]),
                                ::proto_vulcan::state::reify(::std::clone::Clone::clone(&__query__)),
                            ]),
                        )
                    )
                )
            };

            use std::fmt;

            #[derive(Clone, Debug)]
            struct QResult<U: ::proto_vulcan::user::User, E: ::proto_vulcan::engine::Engine<U>> {
                #( #query: ::proto_vulcan::lresult::LResult<U, E>, )*
            }

            impl<U: ::proto_vulcan::user::User, E: ::proto_vulcan::engine::Engine<U>> ::proto_vulcan::query::QueryResult<U, E> for QResult<U, E> {
                fn from_vec(v: Vec<::proto_vulcan::lresult::LResult<U, E>>) -> QResult<U, E> {
                    let mut vi = v.into_iter();
                    QResult {
                        #( #query: vi.next().unwrap(), )*
                    }
                }
            }

            impl<U: ::proto_vulcan::user::User, E: ::proto_vulcan::engine::Engine<U>> fmt::Display for QResult<U, E> {
                #[allow(unused_variables, unused_assignments)]
                fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    let mut count = 0;
                    #( if count > 0 { writeln!(f, "")?; }  write!(f, "{}: {}", stringify!(#query), self.#query)?; count += 1; )*
                    write!(f, "")
                }
            }

            ::proto_vulcan::query::Query::<QResult<_, _>>::new(__vars__, goal)
        };

        output.to_tokens(tokens);
    }
}
