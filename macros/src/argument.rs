use crate::{CompoundConstructor, TreeTerm};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::token::{Brace, Paren};
use syn::{braced, Ident};

#[derive(Clone)]
pub enum Argument {
    // [x, y, z]
    TreeTerm(TreeTerm),
    Compound(CompoundConstructor),
    // {...}
    Expr { expr: syn::Expr, cast: bool },
}

impl Parse for Argument {
    fn parse(input: ParseStream) -> Result<Self> {
        // No parsing for quoted input
        if input.peek(Brace) {
            let content;
            let _ = braced!(content in input);
            let expr = content.parse()?;
            Ok(Argument::Expr { expr, cast: true })
        } else if (input.peek(syn::token::Colon2) && input.peek2(Ident))
            || (input.peek(Ident) && (input.peek2(syn::token::Colon2) || input.peek2(Paren))
                || input.peek(Paren))
        {
            let compound: CompoundConstructor = input.parse()?;
            Ok(Argument::Compound(compound))
        } else if let Ok(term) = input.parse() {
            // Try parsing TreeTerm
            Ok(Argument::TreeTerm(term))
        } else {
            // If not a treeterm, fall back to expression
            let expr = input.parse()?;
            Ok(Argument::Expr { expr, cast: false })
        }
    }
}

impl ToTokens for Argument {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Argument::TreeTerm(term) => {
                let output = quote! { #term };
                output.to_tokens(tokens);
            }
            Argument::Compound(compound_term) => {
                let output = quote! { #compound_term };
                output.to_tokens(tokens);
            }
            Argument::Expr { expr, cast } => {
                let output = if *cast {
                    quote! { ::std::convert::Into::into(#expr) }
                } else {
                    quote! { #expr }
                };
                output.to_tokens(tokens);
            }
        }
    }
}
