use crate::attributes::ControllerAttributes;
use itertools::{Either, Itertools};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{
    Attribute, Error, Expr, ExprLit, FnArg, Ident, ImplItem, Item, ItemImpl, Lit, LitStr, Result,
};

macro_rules! impl_handlers {
    ($ident:expr, $path:expr, $inner_code:expr, $($m:tt)+) => {
        $(if $ident == stringify!($m) {
            let inner_code = $inner_code;
            let path = $path;
            Some(ControllerMethod::Configuration(quote!(let router = router.route(#path, $m(#inner_code));)))
        } else)+ {
            None
        }
    }
}

#[derive(Clone)]
enum ControllerMethod {
    Configuration(TokenStream),
    Source(TokenStream),
    PostConfigure(TokenStream),
}

fn generate_method_configuration(
    attr: &Attribute,
    inner_code: &TokenStream,
    method_prefix: &TokenStream,
    method_name: &Ident,
) -> Result<Option<ControllerMethod>> {
    attr.meta
        .path()
        .get_ident()
        .and_then(|ident| {
            if ident == "fallback" {
                return Some(Ok(ControllerMethod::Configuration(quote!(let router = router.fallback(#inner_code);))));
            }

            if ident == "router_source" {
                return Some(Ok(ControllerMethod::Source(quote!(#method_prefix::#method_name(self)))));
            }

            if ident == "router_post_configure" {
                return Some(Ok(ControllerMethod::PostConfigure(quote!(#method_prefix::#method_name(self, router)))));
            }

            attr.parse_args::<LitStr>().map(|path| {
                impl_handlers!(ident, path, inner_code, delete get head options patch post put trace)
            }).transpose()
        })
        .transpose()
}

struct RouterConfiguration {
    methods: TokenStream,
    router_source: Option<TokenStream>,
    post_configure_router: Option<TokenStream>,
}

fn extract_router_configuration(item: &mut ItemImpl) -> Result<RouterConfiguration> {
    let mut method_configs = vec![];
    let mut router_source = None;
    let mut post_configure_router = None;

    let self_ty = item.self_ty.as_ref();
    let method_prefix = item
        .trait_
        .as_ref()
        .map(|(_, path, ..)| quote!(<#self_ty as #path>))
        .unwrap_or_else(|| quote!(#self_ty));

    for item in &mut item.items {
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
                    move |#(#args),*| async move { #method_prefix::#name(self_instance_ptr.as_ref(), #(#args),*).await }
                }
            };

            let (normal_attrs, controller_attrs): (Vec<_>, Vec<_>) =
                item.attrs.iter().partition_map(|attr| {
                    match generate_method_configuration(attr, &function_call, &method_prefix, name)
                    {
                        Ok(Some(controller_attr)) => Either::Right(Ok(controller_attr)),
                        Ok(None) => Either::Left(attr.clone()),
                        Err(error) => Either::Right(Err(error)),
                    }
                });

            if let Some(error) = controller_attrs.iter().find_map(|attr| attr.clone().err()) {
                return Err(error);
            }

            item.attrs = normal_attrs;
            method_configs.extend(controller_attrs.into_iter().filter_map(|attr| match attr {
                Ok(ControllerMethod::Configuration(tokens)) => Some(tokens),
                Ok(ControllerMethod::Source(tokens)) => {
                    router_source = Some(tokens);
                    None
                }
                Ok(ControllerMethod::PostConfigure(tokens)) => {
                    post_configure_router = Some(tokens);
                    None
                }
                Err(_) => None,
            }));
        }
    }

    Ok(RouterConfiguration {
        methods: quote!(#(#method_configs)*),
        router_source,
        post_configure_router,
    })
}

//noinspection DuplicatedCode
pub fn generate_controller(item: Item, attributes: &ControllerAttributes) -> Result<TokenStream> {
    if let Item::Impl(mut item) = item {
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

        let RouterConfiguration {
            methods: router_config,
            router_source,
            post_configure_router,
        } = extract_router_configuration(&mut item)?;

        let ty = &item.self_ty;

        let router_source = router_source
            .map(|router_source| quote!(#router_source))
            .unwrap_or_else(|| quote!(Ok(springtime_web_axum::axum::Router::new())));

        let create_router = quote! {
            fn create_router(&self) -> Result<springtime_web_axum::axum::Router, springtime_di::instance_provider::ErrorPtr> {
                #router_source
            }
        };

        let post_configure_router = post_configure_router
            .map(|post_configure_router| quote!(#post_configure_router))
            .unwrap_or_else(|| quote!(Ok(router)));

        let post_configure_router = quote! {
            fn post_configure_router(&self, router: springtime_web_axum::axum::Router) -> Result<springtime_web_axum::axum::Router, springtime_di::instance_provider::ErrorPtr> {
                #post_configure_router
            }
        };

        Ok(quote! {
            #[automatically_derived]
            #[springtime_di::component_alias]
            impl springtime_web_axum::controller::Controller for #ty {
                #path
                #server_names

                fn configure_router(
                    &self,
                    router: springtime_web_axum::axum::Router,
                    self_instance_ptr: springtime_di::instance_provider::ComponentInstancePtr<dyn springtime_web_axum::controller::Controller + Send + Sync>,
                ) -> Result<springtime_web_axum::axum::Router, springtime_di::instance_provider::ErrorPtr> {
                    use springtime_di::instance_provider::ErrorPtr;
                    use springtime_web_axum::axum::routing::*;
                    use std::sync::Arc;

                    let self_instance_ptr = self_instance_ptr
                        .downcast_arc::<#ty>()
                        .map_err(|error| Arc::new(error) as ErrorPtr)?;

                    #router_config

                    Ok(router)
                }

                #create_router
                #post_configure_router
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
