use crate::component::expand_component;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};

mod component;

#[proc_macro_derive(Component)]
pub fn generate_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_component(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
