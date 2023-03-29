use crate::attributes::ComponentAliasAttributes;
use crate::component::{expand_component, generate_injectable, register_component_alias};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Error, Item};

mod attributes;
mod component;

#[proc_macro_derive(Component, attributes(component))]
pub fn generate_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_component(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn injectable(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    let injectable = generate_injectable(&item).unwrap_or_else(Error::into_compile_error);

    (quote! {
        #item
        #injectable
    })
    .into()
}

#[proc_macro_attribute]
pub fn component_alias(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as ComponentAliasAttributes);
    let item = parse_macro_input!(input as Item);
    let registration =
        register_component_alias(&item, &args).unwrap_or_else(Error::into_compile_error);

    (quote! {
        #item
        #registration
    })
    .into()
}
