use crate::attributes::ControllerAttributes;
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Error, Expr, ExprLit, Item, Lit, Result};

//noinspection DuplicatedCode
pub fn generate_controller(item: &Item, attributes: &ControllerAttributes) -> Result<TokenStream> {
    if let Item::Impl(item) = item {
        let ty = &item.self_ty;
        let path = if let Some(path) = &attributes.path {
            quote! {
                fn path(&self) -> Option<String> {
                    Some(#path.to_string())
                }
            }
        } else {
            quote!()
        };
        let server_names = attributes.server_names.as_ref().map(|server_names| {
            let server_names = server_names
                .elems
                .iter()
                .filter_map(|elem| {
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(string),
                        ..
                    }) = elem
                    {
                        Some(string.value())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            quote! {
                fn server_names(&self) -> Option<springtime_web_axum::controller::ServerNameSet> {
                    Some([#(#server_names.to_string()),*].into_iter().collect())
                }
            }
        }).unwrap_or_else(|| quote!());

        Ok(quote! {
            #[automatically_derived]
            #[springtime_di::component_alias]
            impl springtime_web_axum::controller::Controller for #ty {
                #path
                #server_names

                fn configure_router(&self, router: springtime_web_axum::controller::Router) -> springtime_web_axum::controller::Router {
                    router
                }
            }
        })
    } else {
        Err(Error::new(
            item.span(),
            "Only impl blocks can be marked as a controller!",
        ))
    }
}
