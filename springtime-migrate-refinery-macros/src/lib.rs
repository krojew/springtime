mod migration;

use crate::migration::generate_migrations;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Error, LitStr};

#[proc_macro]
pub fn embed_migrations(input: TokenStream) -> TokenStream {
    let migrations = if input.is_empty() {
        generate_migrations("migrations", Span::call_site())
    } else {
        let path = parse_macro_input!(input as LitStr);
        generate_migrations(&path.value(), path.span())
    };

    let migrations = migrations.unwrap_or_else(Error::into_compile_error);
    quote!(#migrations).into()
}
