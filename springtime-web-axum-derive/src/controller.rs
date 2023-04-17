use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Error, Item, Result};

pub fn generate_controller(item: &Item) -> Result<TokenStream> {
    if let Item::Impl(item) = item {
        let ty = &item.self_ty;

        Ok(quote! {
            #[automatically_derived]
            #[springtime_di::component_alias]
            impl springtime_web_axum::controller::Controller for #ty {
            }
        })
    } else {
        Err(Error::new(
            item.span(),
            "Only impl blocks can be marked as a controller!",
        ))
    }
}
