mod attributes;
mod controller;

use crate::attributes::ControllerAttributes;
use crate::controller::generate_controller;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Error, Item};

#[proc_macro_attribute]
pub fn controller(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as ControllerAttributes);
    let item = parse_macro_input!(input as Item);
    let controller = generate_controller(item, &args).unwrap_or_else(Error::into_compile_error);

    (quote! {
        #controller
    })
    .into()
}
