mod controller;

use crate::controller::generate_controller;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Error, Item};

#[proc_macro_attribute]
pub fn controller(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    let controller = generate_controller(&item).unwrap_or_else(Error::into_compile_error);

    (quote! {
        #item
        #controller
    })
    .into()
}
