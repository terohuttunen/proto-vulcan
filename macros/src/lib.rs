extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;
use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Error};

mod fngoal;
use fngoal::FnGoal;

mod project;
use project::Project;

mod typed_variable;
use typed_variable::TypedVariable;

mod fresh;
use fresh::Fresh;

mod conjunction;
use conjunction::Conjunction;

mod compound;
use compound::{CompoundConstructor, CompoundPattern};

mod pattern;
use pattern::{Pattern, PatternMatchOperator, PatternVariableSet};

mod treeterm;
use treeterm::TreeTerm;

mod value;
use value::Value;

mod argument;
use argument::Argument;

mod relation;
use relation::Relation;

mod closure;
use closure::Closure;

mod operator;
use operator::Operator;

mod loop_operator;
use loop_operator::Loop;

mod for_operator;
use for_operator::For;

mod eq;
use eq::Eq;

mod diseq;
use diseq::Diseq;

mod clause;
use clause::{Clause, ClauseInOperator};

mod query;
use query::Query;

#[proc_macro]
pub fn proto_vulcan(input: TokenStream) -> TokenStream {
    let clause = parse_macro_input!(input as Clause);

    let output = quote! {
        ::proto_vulcan::GoalCast::cast_into(#clause)
    };
    output.into()
}

#[proc_macro]
pub fn proto_vulcan_closure(input: TokenStream) -> TokenStream {
    let clause = parse_macro_input!(input as Clause);
    let closure = Closure::new(vec![clause]);

    let output = quote! {
        ::proto_vulcan::GoalCast::cast_into(#closure)
    };
    output.into()
}

#[proc_macro]
pub fn lterm(input: TokenStream) -> TokenStream {
    let term = parse_macro_input!(input as TreeTerm);

    let output = quote! {
        #term
    };
    output.into()
}

#[proc_macro]
pub fn proto_vulcan_query(input: TokenStream) -> TokenStream {
    let query = parse_macro_input!(input as Query);
    let output = quote! {{
        #query
    }};
    output.into()
}

fn make_compound_modifications_to_path(path: &mut syn::Path) -> std::result::Result<(), Error> {
    match path.segments.iter_mut().last() {
        Some(last_segment) => match last_segment.arguments {
            syn::PathArguments::AngleBracketed(ref mut generic_arguments) => {
                for argument in generic_arguments.args.iter_mut() {
                    match argument {
                        syn::GenericArgument::Type(ty) => {
                            make_compound_modifications_to_type(ty)?;
                        }
                        _ => {
                            return Err(Error::new(argument.span(), "Invalid generic argument"));
                        }
                    }
                }

                return Ok(());
            }
            syn::PathArguments::None => {
                last_segment.arguments =
                    syn::PathArguments::AngleBracketed(syn::parse_quote! {<U, E>});
            }
            _ => {
                return Err(Error::new(
                    last_segment.arguments.span(),
                    "Invalid type argument",
                ));
            }
        },
        None => {
            return Err(Error::new(path.span(), "Invalid type argument"));
        }
    }
    Ok(())
}

fn make_compound_modifications_to_type(ty: &mut syn::Type) -> std::result::Result<(), Error> {
    match ty {
        syn::Type::Path(typepath) => {
            //let has_generic_args = has_generic_arguments(&typepath.path)?;
            make_compound_modifications_to_path(&mut typepath.path)?;
            *ty = syn::parse_quote! { #typepath };
        }
        _ => return Err(Error::new(ty.span(), "Invalid compound type")),
    }
    Ok(())
}

fn make_compound_modifications_to_itemstruct(
    itemstruct: &mut syn::ItemStruct,
) -> std::result::Result<(), Error> {
    if itemstruct.generics.params.is_empty() {
        let new_generics: syn::Generics = syn::parse_quote! {<U: ::proto_vulcan::user::User, E: ::proto_vulcan::engine::Engine<U>>};
        itemstruct.generics = new_generics;
    }
    for field in itemstruct.fields.iter_mut() {
        field.vis = syn::Visibility::Public(syn::VisPublic {
            pub_token: syn::parse_quote!(pub),
        });
        make_compound_modifications_to_type(&mut field.ty)?;
    }
    itemstruct.vis = syn::parse_quote!(pub);
    Ok(())
}

fn make_compound_unnamed_struct(itemstruct: syn::ItemStruct) -> TokenStream {
    let mut inner = itemstruct.clone();
    inner.ident = quote::format_ident!("_Inner{}", itemstruct.ident);

    let vis = &itemstruct.vis;
    let struct_name = itemstruct.ident.clone();
    let inner_ident = &inner.ident;
    let mod_name = quote::format_ident!("{}_compound", struct_name);
    let (impl_generics, type_generics, where_clause) = itemstruct.generics.split_for_impl();

    let field_indices: Vec<syn::Index> = itemstruct
        .fields
        .iter()
        .enumerate()
        .map(|(n, _)| syn::Index::from(n))
        .collect();

    let output = quote!(
        #[allow(non_snake_case)]
        #vis mod #mod_name {
            use super::*;
            #[derive(Eq)]
            #inner

            impl #impl_generics ::std::clone::Clone for #inner_ident #type_generics #where_clause {
                fn clone(&self) -> #inner_ident #type_generics {
                    #inner_ident(#( ::std::clone::Clone::clone(&self.#field_indices) ),* )
                }
            }

            impl #impl_generics ::proto_vulcan::compound::CompoundObject #type_generics for #inner_ident #type_generics #where_clause {
                fn type_name(&self) -> &'static str {
                    stringify!(#struct_name)
                }

                fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn ::proto_vulcan::compound::CompoundObject #type_generics> + 'a> {
                    Box::new(vec![#(&self.#field_indices as &dyn ::proto_vulcan::compound::CompoundObject #type_generics),*].into_iter())
                }
            }

            impl #impl_generics ::proto_vulcan::compound::CompoundWalkStar #type_generics for #inner_ident #type_generics #where_clause {
                fn compound_walk_star(&self, smap: &::proto_vulcan::state::SMap #type_generics) -> Self {
                    #inner_ident(#(self.#field_indices.compound_walk_star(smap)),*)
                }
            }

            impl #impl_generics Into<#struct_name #type_generics> for #inner_ident #type_generics #where_clause {
                fn into(self) -> #struct_name #type_generics {
                    #struct_name {
                        inner: Into::<LTerm #type_generics>::into(self),
                    }
                }
            }

            impl #impl_generics Into<::proto_vulcan::lterm::LTerm #type_generics> for #inner_ident #type_generics #where_clause {
                fn into(self) -> ::proto_vulcan::lterm::LTerm #type_generics {
                    ::proto_vulcan::lterm::LTerm::from(::std::rc::Rc::new(self) as ::std::rc::Rc<dyn ::proto_vulcan::compound::CompoundObject #type_generics>)
                }
            }

            impl #impl_generics ::proto_vulcan::Downcast #type_generics for #inner_ident #type_generics #where_clause {
                type SubType = #struct_name #type_generics;
                fn into_sub(self) -> Self::SubType {
                    self.into()
                }
            }

            impl #impl_generics ::core::fmt::Debug for #inner_ident #type_generics #where_clause {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_tuple(f, stringify!(#struct_name));
                    #( let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &self.#field_indices); )*
                    ::core::fmt::DebugTuple::finish(debug_trait_builder)
                }
            }

            impl #impl_generics ::std::hash::Hash for #inner_ident #type_generics #where_clause {
                fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                    #( ::std::hash::Hash::hash(&self.#field_indices, state); )*
                }
            }

            impl #impl_generics ::std::cmp::PartialEq for #inner_ident #type_generics #where_clause {
                fn eq(&self, other: &Self) -> bool {
                    #( ::std::cmp::PartialEq::eq(&self.#field_indices, &other.#field_indices) &&)* true
                }
            }
        }

        #[derive(Eq)]
        #vis struct #struct_name #impl_generics {
            inner: LTerm #type_generics,
        }

        impl #impl_generics ::std::clone::Clone for #struct_name #type_generics #where_clause {
            fn clone(&self) -> #struct_name #type_generics {
                #struct_name {
                    inner: ::std::clone::Clone::clone(&self.inner),
                }
            }
        }

        impl #impl_generics ::std::fmt::Debug for #struct_name #type_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                self.inner.fmt(f)
            }
        }

        impl #impl_generics ::std::hash::Hash for #struct_name #type_generics #where_clause {
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                ::std::hash::Hash::hash(&self.inner, state);
            }
        }

        impl #impl_generics ::std::cmp::PartialEq for #struct_name #type_generics #where_clause {
            fn eq(&self, other: &Self) -> bool {
                ::std::cmp::PartialEq::eq(&self.inner, &other.inner)
            }
        }

        #[automatically_derived]
        impl #impl_generics ::proto_vulcan::compound::CompoundTerm #type_generics for #struct_name #type_generics #where_clause {
            fn new_var(name: &'static str) -> #struct_name #type_generics {
                #struct_name {
                    inner: LTerm::var(name),
                }
            }

            fn new_wildcard() -> #struct_name #type_generics {
                #struct_name {
                    inner: LTerm::any(),
                }
            }

            fn new_none() -> #struct_name #type_generics {
                #struct_name {
                    inner: LTerm::empty_list(),
                }
            }
        }

        impl #impl_generics ::proto_vulcan::compound::CompoundObject #type_generics for #struct_name #type_generics #where_clause {
            fn type_name(&self) -> &'static str {
                stringify!(#struct_name)
            }

            fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn ::proto_vulcan::compound::CompoundObject #type_generics> + 'a> {
                self.inner.children()
            }

            fn as_term(&self) -> Option<&LTerm<U, E>> {
                Some(&self.inner)
            }
        }

        impl #impl_generics ::proto_vulcan::compound::CompoundWalkStar #type_generics for #struct_name #type_generics #where_clause {
            fn compound_walk_star(&self, smap: &::proto_vulcan::state::SMap #type_generics) -> Self {
                #struct_name {
                    inner: self.inner.compound_walk_star(smap),
                }
            }
        }

        #[automatically_derived]
        impl #impl_generics Into<::proto_vulcan::lterm::LTerm #type_generics> for #struct_name #type_generics #where_clause {
            fn into(self) -> LTerm #type_generics {
                self.inner
            }
        }

        impl #impl_generics ::proto_vulcan::Upcast<U, E, ::proto_vulcan::lterm::LTerm #type_generics> for #struct_name #type_generics #where_clause {
            #[inline]
            fn to_super<K: ::std::borrow::Borrow<Self>>(k: &K) -> ::proto_vulcan::lterm::LTerm #type_generics {
                Into::into(::std::clone::Clone::clone(k.borrow()))
            }

            #[inline]
            fn into_super(self) -> ::proto_vulcan::lterm::LTerm #type_generics {
                Into::into(self)
            }
        }

        impl #impl_generics ::proto_vulcan::Downcast #type_generics for #struct_name #type_generics #where_clause {
            type SubType = Self;
            fn into_sub(self) -> Self::SubType {
                self.into()
            }
        }
    );
    output.into()
}

fn make_compound_named_struct(itemstruct: syn::ItemStruct) -> TokenStream {
    let mut inner = itemstruct.clone();
    inner.ident = quote::format_ident!("_Inner{}", itemstruct.ident);

    let vis = &itemstruct.vis;
    let struct_name = itemstruct.ident.clone();
    let inner_ident = &inner.ident;
    let mod_name = quote::format_ident!("{}_compound", struct_name);
    let (impl_generics, type_generics, where_clause) = itemstruct.generics.split_for_impl();

    let field_names: Vec<syn::Ident> = itemstruct
        .fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap().clone())
        .collect();

    let output = quote!(
        #[allow(non_snake_case)]
        #vis mod #mod_name {
            use super::*;
            #[derive(Eq)]
            #inner

            impl #impl_generics ::std::clone::Clone for #inner_ident #type_generics #where_clause {
                fn clone(&self) -> #inner_ident #type_generics {
                    #inner_ident {
                        #( #field_names: ::std::clone::Clone::clone(&self.#field_names) ),*
                    }
                }
            }

            impl #impl_generics ::proto_vulcan::compound::CompoundObject #type_generics for #inner_ident #type_generics #where_clause {
                fn type_name(&self) -> &'static str {
                    stringify!(#struct_name)
                }

                fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn ::proto_vulcan::compound::CompoundObject #type_generics> + 'a> {
                    Box::new(vec![#(&self.#field_names as &dyn ::proto_vulcan::compound::CompoundObject #type_generics),*].into_iter())
                }
            }

            impl #impl_generics ::proto_vulcan::compound::CompoundWalkStar #type_generics for #inner_ident #type_generics #where_clause {
                fn compound_walk_star(&self, smap: &::proto_vulcan::state::SMap #type_generics) -> Self {
                    #inner_ident { #( #field_names: self.#field_names.compound_walk_star(smap)),* }
                }
            }

            impl #impl_generics Into<#struct_name #type_generics> for #inner_ident #type_generics #where_clause {
                fn into(self) -> #struct_name #type_generics {
                    #struct_name {
                        inner: Into::<LTerm #type_generics>::into(self),
                    }
                }
            }

            impl #impl_generics Into<::proto_vulcan::lterm::LTerm #type_generics> for #inner_ident #type_generics #where_clause {
                fn into(self) -> ::proto_vulcan::lterm::LTerm #type_generics {
                    ::proto_vulcan::lterm::LTerm::from(::std::rc::Rc::new(self) as ::std::rc::Rc<dyn ::proto_vulcan::compound::CompoundObject #type_generics>)
                }
            }

            impl #impl_generics ::proto_vulcan::Downcast #type_generics for #inner_ident #type_generics #where_clause {
                type SubType = #struct_name #type_generics;
                fn into_sub(self) -> Self::SubType {
                    self.into()
                }
            }

            impl #impl_generics ::core::fmt::Debug for #inner_ident #type_generics #where_clause {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_struct(f, stringify!(#struct_name));
                    #(
                        let _= ::core::fmt::DebugStruct::field(
                            debug_trait_builder,
                            stringify!(#field_names),
                            &self.#field_names,
                        );
                    )*
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }

            impl #impl_generics ::std::hash::Hash for #inner_ident #type_generics #where_clause {
                fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                    #( ::std::hash::Hash::hash(&self.#field_names, state); )*
                }
            }

            impl #impl_generics ::std::cmp::PartialEq for #inner_ident #type_generics #where_clause {
                fn eq(&self, other: &Self) -> bool {
                    #( ::std::cmp::PartialEq::eq(&self.#field_names, &other.#field_names) &&)* true
                }
            }
        }

        #[derive(Eq)]
        #vis struct #struct_name #impl_generics {
            inner: LTerm #type_generics,
        }

        impl #impl_generics ::std::clone::Clone for #struct_name #type_generics #where_clause {
            fn clone(&self) -> #struct_name #type_generics {
                #struct_name {
                    inner: ::std::clone::Clone::clone(&self.inner),
                }
            }
        }

        impl #impl_generics ::std::fmt::Debug for #struct_name #type_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                self.inner.fmt(f)
            }
        }

        impl #impl_generics ::std::hash::Hash for #struct_name #type_generics #where_clause {
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                ::std::hash::Hash::hash(&self.inner, state);
            }
        }

        impl #impl_generics ::std::cmp::PartialEq for #struct_name #type_generics #where_clause {
            fn eq(&self, other: &Self) -> bool {
                ::std::cmp::PartialEq::eq(&self.inner, &other.inner)
            }
        }

        #[automatically_derived]
        impl #impl_generics ::proto_vulcan::compound::CompoundTerm #type_generics for #struct_name #type_generics #where_clause {
            fn new_var(name: &'static str) -> #struct_name #type_generics {
                #struct_name {
                    inner: LTerm::var(name),
                }
            }

            fn new_wildcard() -> #struct_name #type_generics {
                #struct_name {
                    inner: LTerm::any(),
                }
            }

            fn new_none() -> #struct_name #type_generics {
                #struct_name {
                    inner: LTerm::empty_list(),
                }
            }
        }

        impl #impl_generics ::proto_vulcan::compound::CompoundObject #type_generics for #struct_name #type_generics #where_clause {
            fn type_name(&self) -> &'static str {
                stringify!(#struct_name)
            }

            fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn ::proto_vulcan::compound::CompoundObject #type_generics> + 'a> {
                self.inner.children()
            }

            fn as_term(&self) -> Option<&LTerm #type_generics> {
                Some(&self.inner)
            }
        }

        impl #impl_generics ::proto_vulcan::compound::CompoundWalkStar #type_generics for #struct_name #type_generics #where_clause {
            fn compound_walk_star(&self, smap: &::proto_vulcan::state::SMap #type_generics) -> Self {
                #struct_name {
                    inner: self.inner.compound_walk_star(smap),
                }
            }
        }

        #[automatically_derived]
        impl #impl_generics Into<::proto_vulcan::lterm::LTerm #type_generics> for #struct_name #type_generics #where_clause {
            fn into(self) -> LTerm #type_generics {
                self.inner
            }
        }

        impl #impl_generics ::proto_vulcan::Upcast<U, E, ::proto_vulcan::lterm::LTerm #type_generics> for #struct_name #type_generics #where_clause {
            #[inline]
            fn to_super<K: ::std::borrow::Borrow<Self>>(k: &K) -> ::proto_vulcan::lterm::LTerm #type_generics {
                Into::into(::std::clone::Clone::clone(k.borrow()))
            }

            #[inline]
            fn into_super(self) -> ::proto_vulcan::lterm::LTerm #type_generics {
                Into::into(self)
            }
        }

        impl #impl_generics ::proto_vulcan::Downcast #type_generics for #struct_name #type_generics #where_clause {
            type SubType = Self;
            fn into_sub(self) -> Self::SubType {
                self.into()
            }
        }
    );
    output.into()
}

fn make_compound_struct(mut itemstruct: syn::ItemStruct) -> TokenStream {
    // Add generics and where necessary
    match make_compound_modifications_to_itemstruct(&mut itemstruct) {
        Ok(()) => (),
        Err(error) => return error.to_compile_error().into(),
    }

    match itemstruct.fields {
        syn::Fields::Unnamed(_) => make_compound_unnamed_struct(itemstruct),
        syn::Fields::Named(_) => make_compound_named_struct(itemstruct),
        syn::Fields::Unit => make_compound_named_struct(itemstruct),
    }
}

#[proc_macro_attribute]
pub fn compound(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as syn::Item);

    match item {
        //syn::Item::Enum(item_enum) => return make_compound_enum(item_enum),
        syn::Item::Struct(item_struct) => return make_compound_struct(item_struct),
        _ => {
            return syn::Error::new(item.span(), "Compound attribute requires struct or enum.")
                .to_compile_error()
                .into();
        }
    }
}
