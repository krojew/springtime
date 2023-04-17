use crate::attributes::ControllerAttributes;
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Error, Item, Result};

pub fn generate_controller(item: &Item, args: &ControllerAttributes) -> Result<TokenStream> {
    if let Item::Impl(item) = item {
        let ty = &item.self_ty;
        let path = if let Some(path) = &args.path {
            quote! {
                fn path(&self) -> Option<String> {
                    Some(#path.to_string())
                }
            }
        } else {
            quote!()
        };

        Ok(quote! {
            #[automatically_derived]
            #[springtime_di::component_alias]
            impl springtime_web_axum::controller::Controller for #ty {
                #path
            }
        })
    } else {
        Err(Error::new(
            item.span(),
            "Only impl blocks can be marked as a controller!",
        ))
    }
}
