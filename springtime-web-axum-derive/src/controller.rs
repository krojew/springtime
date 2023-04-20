use crate::attributes::ControllerAttributes;
use itertools::{Either, Itertools};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, Error, Expr, ExprLit, FnArg, Ident, ImplItem, Item, Lit, LitStr, Result};

macro_rules! impl_handlers {
    ($ident:expr, $path:expr, $inner_code:expr, $($m:tt)+) => {
        $(if $ident == stringify!($m) {
            let inner_code = $inner_code;
            let path = $path;
            quote!(let router = router.route(#path, $m(#inner_code));)
        } else)+ {
            quote!()
        }
    }
}

fn generate_method_configuration(
    attr: &Attribute,
    inner_code: &TokenStream,
) -> Result<Option<TokenStream>> {
    attr.meta
        .path()
        .get_ident()
        .map(|ident| {
            if ident == "fallback" {
                return Ok(quote!(let router = router.fallback(#inner_code);));
            }

            attr.parse_args::<LitStr>().map(|path| {
            impl_handlers!(ident, path, inner_code, delete get head options patch post put trace)
        })
        })
        .transpose()
}

fn extract_router_configuration(items: &mut Vec<ImplItem>) -> Result<TokenStream> {
    let mut method_configs = vec![];

    for item in items {
        if let ImplItem::Fn(item) = item {
            let name = &item.sig.ident;
            let args = item
                .sig
                .inputs
                .iter()
                .filter(|input| !matches!(input, FnArg::Receiver(_)))
                .enumerate()
                .map(|(index, _)| Ident::new(&format!("a{index}"), Span::call_site()))
                .collect_vec();

            let function_call = quote! {
                {
                    let self_instance_ptr = self_instance_ptr.clone();
                    move |#(#args),*| async move { self_instance_ptr.#name(#(#args),*).await }
                }
            };

            let (normal_attrs, controller_attrs): (Vec<_>, Vec<_>) =
                item.attrs.iter().partition_map(|attr| {
                    match generate_method_configuration(attr, &function_call) {
                        Ok(Some(controller_attr)) => Either::Right(Ok(controller_attr)),
                        Ok(None) => Either::Left(attr.clone()),
                        Err(error) => Either::Right(Err(error)),
                    }
                });

            if let Some(error) = controller_attrs
                .iter()
                .find_map(|attr| {
                    if let Err(error) = attr {
                        Some(error)
                    } else {
                        None
                    }
                })
                .cloned()
            {
                return Err(error);
            }

            item.attrs = normal_attrs;
            method_configs.extend(controller_attrs.into_iter().filter_map(|attr| {
                if let Ok(tokens) = attr {
                    Some(tokens)
                } else {
                    None
                }
            }));
        }
    }

    Ok(quote! {
        #(#method_configs)*
    })
}

//noinspection DuplicatedCode
pub fn generate_controller(item: Item, attributes: &ControllerAttributes) -> Result<TokenStream> {
    if let Item::Impl(mut item) = item {
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
        let router_config = extract_router_configuration(&mut item.items)?;

        Ok(quote! {
            #[automatically_derived]
            #[springtime_di::component_alias]
            impl springtime_web_axum::controller::Controller for #ty {
                #path
                #server_names

                fn configure_router(
                    &self,
                    self_instance_ptr: springtime_di::instance_provider::ComponentInstancePtr<dyn springtime_web_axum::controller::Controller + Send + Sync>,
                ) -> Result<springtime_web_axum::axum::Router, springtime_web_axum::controller::RouterError> {
                    use springtime_web_axum::controller::RouterError;
                    use springtime_web_axum::axum::routing::*;

                    let router = springtime_web_axum::axum::Router::new();
                    let self_instance_ptr = self_instance_ptr
                        .downcast_arc::<#ty>()
                        .map_err(|error| RouterError::RouterConfigurationError(format!("Invalid controller instance: {}", error)))?;

                    #router_config

                    Ok(router)
                }
            }

            #item
        })
    } else {
        Err(Error::new(
            item.span(),
            "Only impl blocks can be marked as a controller!",
        ))
    }
}
