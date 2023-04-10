use crate::attributes::{
    ComponentAliasAttributes, ComponentAttributes, ConstructorParameter, DefaultDefinition,
    FieldAttributes,
};
use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::ops::Deref;
use syn::spanned::Spanned;
use syn::{
    parse_str, Attribute, Data, DataStruct, DeriveInput, Error, Expr, ExprArray, ExprLit, ExprPath,
    Field, Fields, FieldsNamed, FieldsUnnamed, GenericArgument, Item, Lit, LitStr, PathArguments,
    Result, Type, TypePath, TypeTraitObject,
};

const COMPONENT_ATTR: &str = "component";

fn ungroup(mut ty: &Type) -> &Type {
    while let Type::Group(group) = ty {
        ty = &group.elem;
    }

    ty
}

fn get_wrapped_type(ty: &Type, expected_wrapper: &str, require_ptr: bool) -> Option<TokenStream> {
    let path = match ungroup(ty) {
        Type::Path(ty) => &ty.path,
        _ => {
            return None;
        }
    };

    let seg = match path.segments.last() {
        Some(seg) => seg,
        None => {
            return None;
        }
    };

    let args = match &seg.arguments {
        PathArguments::AngleBracketed(bracketed) => &bracketed.args,
        _ => {
            return None;
        }
    };

    if seg.ident != expected_wrapper || args.len() != 1 {
        return None;
    }

    if !require_ptr {
        let ty = &args[0];
        return Some(quote!(#ty));
    }

    if let GenericArgument::Type(Type::Path(TypePath { path, .. })) = &args[0] {
        if let Some(last_segment) = path.segments.last() {
            if last_segment.ident == "ComponentInstancePtr" {
                if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    if let Some(GenericArgument::Type(ty)) = args.args.first() {
                        return Some(quote!(#ty));
                    }
                }
            }
        }
    }

    None
}

fn get_injected_option_type(ty: &Type) -> Option<TokenStream> {
    get_wrapped_type(ty, "Option", true)
}

fn get_constructor_option_type(ty: &Type) -> Option<TokenStream> {
    get_wrapped_type(ty, "Option", false)
}

fn get_injected_type(ty: &Type) -> TokenStream {
    // let's try to extract "dyn Trait" from the inner type
    if let Type::Path(TypePath { path, .. }) = ungroup(ty) {
        if let Some(last_segment) = path.segments.last() {
            if last_segment.ident == "ComponentInstancePtr" {
                if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    if let Some(GenericArgument::Type(Type::TraitObject(TypeTraitObject {
                        dyn_token,
                        bounds,
                    }))) = args.args.first()
                    {
                        // and we're done
                        return quote!(#dyn_token #bounds);
                    }
                }
            }
        }
    }

    quote!(<#ty as Deref>::Target)
}

fn get_injected_vec_type(ty: &Type) -> Option<TokenStream> {
    get_wrapped_type(ty, "Vec", true)
}

fn get_constructor_vec_type(ty: &Type) -> Option<TokenStream> {
    get_wrapped_type(ty, "Vec", false)
}

fn get_unnamed_instance(ty: &Type) -> TokenStream {
    let (getter, ty) = get_injected_option_type(ty)
        .map(|ty| (quote!(primary_instance_option), ty))
        .or_else(|| get_injected_vec_type(ty).map(|ty| (quote!(instances_typed), ty)))
        .unwrap_or_else(|| (quote!(primary_instance_typed), get_injected_type(ty)));

    #[cfg(not(feature = "async"))]
    quote! {
        instance_provider.#getter::<#ty>()?
    }

    #[cfg(feature = "async")]
    quote! {
        instance_provider.#getter::<#ty>().await?
    }
}

fn get_named_instance(ty: &Type, name: &LitStr) -> TokenStream {
    let (getter, ty) = get_injected_option_type(ty)
        .map(|ty| (quote!(instance_by_name_option), ty))
        .or_else(|| get_injected_vec_type(ty).map(|ty| (quote!(instances_typed), ty)))
        .unwrap_or_else(|| (quote!(instance_by_name_typed), get_injected_type(ty)));

    #[cfg(not(feature = "async"))]
    quote! {
        instance_provider.#getter::<#ty>(#name)?
    }

    #[cfg(feature = "async")]
    quote! {
        instance_provider.#getter::<#ty>(#name).await?
    }
}

fn get_instance(ty: &Type, name: Option<&LitStr>) -> TokenStream {
    name.map(|name| get_named_instance(ty, name))
        .unwrap_or_else(|| get_unnamed_instance(ty))
}

fn generate_field_construction(field: &Field) -> Result<TokenStream> {
    for attr in &field.attrs {
        if attr.path().is_ident(COMPONENT_ATTR) {
            let attributes = FieldAttributes::try_from(attr)?;
            return match &attributes.default {
                Some(DefaultDefinition::Expr(path)) => Ok(quote!(#path())),
                Some(DefaultDefinition::Default) => Ok(quote!(std::default::Default::default())),
                _ => Ok(get_instance(&field.ty, attributes.name.as_ref())),
            };
        }
    }

    Ok(get_instance(&field.ty, None))
}

fn make_named_struct(fields: &FieldsNamed) -> Result<TokenStream> {
    let fields: Vec<_> = fields
        .named
        .iter()
        .map(|field| -> Result<TokenStream> {
            let ident = field.ident.as_ref().unwrap();
            let instance = generate_field_construction(field)?;
            Ok(quote! {
                #ident: #instance
            })
        })
        .try_collect()?;

    Ok(quote! {
        Self {
            #(#fields),*
        }
    })
}

fn make_unnamed_struct(fields: &FieldsUnnamed) -> Result<TokenStream> {
    let fields: Vec<_> = fields
        .unnamed
        .iter()
        .map(|field| -> Result<TokenStream> {
            let instance = generate_field_construction(field)?;
            Ok(quote! {
                #instance
            })
        })
        .try_collect()?;

    Ok(quote! {
        Self(#(#fields),*)
    })
}

fn generate_constructor_call_arguments<'a>(
    fields: impl Iterator<Item = &'a Field>,
    constructor_parameters: &[ConstructorParameter],
) -> Result<TokenStream> {
    let fields: Vec<_> = fields
        .map(|field| {
            for attr in &field.attrs {
                if attr.path().is_ident(COMPONENT_ATTR) {
                    let attributes = FieldAttributes::try_from(attr)?;
                    return Ok((attributes.ignore, field));
                }
            }

            Ok((false, field))
        })
        .filter_map_ok(|(ignore, field)| (!ignore).then_some(field))
        .map(|field| field.and_then(generate_field_construction))
        .try_collect()?;

    let constructor_parameters = generate_constructor_parameters(constructor_parameters)?;

    Ok(if fields.is_empty() {
        quote!(#constructor_parameters)
    } else {
        quote! {
            #(#fields),*, #constructor_parameters
        }
    })
}

fn generate_constructor_parameters(
    constructor_parameters: &[ConstructorParameter],
) -> Result<TokenStream> {
    constructor_parameters
        .iter()
        .map(|param| {
            parse_str::<Type>(&param.component_type).map(|component_type| {
                param
                    .name
                    .as_ref()
                    .map(|name| {
                        get_constructor_option_type(&component_type)
                            .map(|component_type| {
                                #[cfg(not(feature = "async"))]
                                quote! {
                                    instance_provider.instance_by_name_option::<#component_type>(#name)?
                                }
                                #[cfg(feature = "async")]
                                quote! {
                                    instance_provider.instance_by_name_option::<#component_type>(#name).await?
                                }
                            })
                            .unwrap_or_else(|| {
                                #[cfg(not(feature = "async"))]
                                quote! {
                                    instance_provider.instance_by_name_typed::<#component_type>(#name)?
                                }
                                #[cfg(feature = "async")]
                                quote! {
                                    instance_provider.instance_by_name_typed::<#component_type>(#name).await?
                                }
                            })
                    })
                    .unwrap_or_else(|| {
                        get_constructor_vec_type(&component_type)
                            .map(|component_type| {
                                #[cfg(not(feature = "async"))]
                                quote! {
                                    instance_provider.instances_typed::<#component_type>()?
                                }
                                #[cfg(feature = "async")]
                                quote! {
                                    instance_provider.instances_typed::<#component_type>().await?
                                }
                            })
                            .or_else(|| get_constructor_option_type(&component_type)
                                .map(|component_type| {
                                    #[cfg(not(feature = "async"))]
                                    quote! {
                                        instance_provider.primary_instance_option::<#component_type>()?
                                    }
                                    #[cfg(feature = "async")]
                                    quote! {
                                        instance_provider.primary_instance_option::<#component_type>().await?
                                    }
                                }))
                            .unwrap_or_else(|| {
                                #[cfg(not(feature = "async"))]
                                quote! {
                                    instance_provider.primary_instance_typed::<#component_type>()?
                                }
                                #[cfg(feature = "async")]
                                quote! {
                                    instance_provider.primary_instance_typed::<#component_type>().await?
                                }
                            })
                    })
            })
        })
        .fold_ok(quote!(), |tokens, param| quote!(#tokens #param,))
}

fn make_constructor_call(
    fields: &Fields,
    constructor: &ExprPath,
    constructor_parameters: &[ConstructorParameter],
) -> Result<TokenStream> {
    let fields = match fields {
        Fields::Named(FieldsNamed { named, .. }) => {
            generate_constructor_call_arguments(named.iter(), constructor_parameters)?
        }
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
            generate_constructor_call_arguments(unnamed.iter(), constructor_parameters)?
        }
        Fields::Unit => generate_constructor_parameters(constructor_parameters)?,
    };

    #[cfg(not(feature = "async"))]
    let call = quote! {
        #constructor(#fields)
            .map_err(|error| ComponentInstanceProviderError::ConstructorError(error))
    };
    #[cfg(feature = "async")]
    let call = quote! {
        #constructor(#fields)
            .await
            .map_err(|error| ComponentInstanceProviderError::ConstructorError(error))
    };

    Ok(call)
}

fn extract_component_attributes(attributes: &[Attribute]) -> Result<Option<ComponentAttributes>> {
    attributes
        .iter()
        .filter_map(|attribute| {
            if attribute.path().is_ident(COMPONENT_ATTR) {
                Some(ComponentAttributes::try_from(attribute))
            } else {
                None
            }
        })
        .next()
        .transpose()
}

fn generate_names(attribute_names: Option<ExprArray>, ident: &Ident) -> Vec<String> {
    attribute_names
        .map(|names| {
            names
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
                .collect()
        })
        .unwrap_or_else(|| vec![ident.to_string().to_case(Case::Snake)])
}

pub fn generate_injectable(item: &Item) -> Result<TokenStream> {
    if let Item::Trait(item_trait) = item {
        let ident = &item_trait.ident;

        #[cfg(feature = "threadsafe")]
        let trait_bounds = quote!( + Sync + Send);
        #[cfg(not(feature = "threadsafe"))]
        let trait_bounds = quote!();

        Ok(quote! {
            #[automatically_derived]
            impl springtime_di::component::Injectable for dyn #ident #trait_bounds {}
        })
    } else if let Item::Struct(item_struct) = item {
        let ident = &item_struct.ident;

        Ok(quote! {
            #[automatically_derived]
            impl springtime_di::component::Injectable for #ident {}
        })
    } else {
        Err(Error::new(
            item.span(),
            "Only traits or structs can be marked as injectable!",
        ))
    }
}

pub fn expand_component(input: &DeriveInput) -> Result<TokenStream> {
    if let Data::Struct(DataStruct { fields, .. }) = &input.data {
        let ident = &input.ident;
        let attributes = extract_component_attributes(&input.attrs)?;
        let generation = if let Some(ComponentAttributes {
            constructor: Some(constructor),
            constructor_parameters,
            ..
        }) = &attributes
        {
            make_constructor_call(fields, constructor, constructor_parameters)?
        } else {
            match fields {
                Fields::Named(fields) => {
                    let component = make_named_struct(fields)?;
                    quote!(Ok(#component))
                }
                Fields::Unnamed(fields) => {
                    let component = make_unnamed_struct(fields)?;
                    quote!(Ok(#component))
                }
                Fields::Unit => quote! { Ok(Self) },
            }
        };
        let names = attributes
            .as_ref()
            .and_then(|attributes| attributes.names.clone());
        let names = generate_names(names, &input.ident);
        let condition = attributes
            .as_ref()
            .and_then(|attributes| attributes.condition.clone())
            .map(|condition| quote!(Some(#condition)))
            .unwrap_or_else(|| quote!(None));
        let priority = attributes
            .as_ref()
            .map(|attributes| attributes.priority)
            .unwrap_or(0);
        let scope = attributes
            .as_ref()
            .and_then(|attributes| attributes.scope.clone())
            .map(|scope| quote!(#scope))
            .unwrap_or_else(|| quote!(springtime_di::scope::SINGLETON));

        #[cfg(not(feature = "async"))]
        let constructor = quote! {
            fn constructor(
                instance_provider: &mut dyn ComponentInstanceProvider,
            ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError> {
                #ident::create(instance_provider).map(|p| ComponentInstancePtr::new(p) as ComponentInstanceAnyPtr)
            }
        };

        #[cfg(feature = "async")]
        let constructor = quote! {
            fn constructor(
                instance_provider: &mut (dyn ComponentInstanceProvider + Sync + Send),
            ) -> springtime_di::future::BoxFuture<Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError>> {
                use springtime_di::future::FutureExt;
                async move {
                    #ident::create(instance_provider).await.map(|p| ComponentInstancePtr::new(p) as ComponentInstanceAnyPtr)
                }.boxed()
            }
        };

        #[cfg(not(feature = "async"))]
        let create = quote! {
            fn create(
                instance_provider: &mut dyn springtime_di::instance_provider::ComponentInstanceProvider,
            ) -> Result<Self, springtime_di::instance_provider::ComponentInstanceProviderError> {
                use springtime_di::instance_provider::{ComponentInstanceProviderError, TypedComponentInstanceProvider};
                use std::ops::Deref;
                #generation
            }
        };

        #[cfg(feature = "async")]
        let create = quote! {
            fn create(
                instance_provider: &mut (dyn springtime_di::instance_provider::ComponentInstanceProvider + Sync + Send),
            ) -> springtime_di::future::BoxFuture<Result<Self, springtime_di::instance_provider::ComponentInstanceProviderError>> {
                use springtime_di::future::FutureExt;
                use springtime_di::instance_provider::{ComponentInstanceProviderError, TypedComponentInstanceProvider};
                use std::ops::Deref;
                async move { #generation }.boxed()
            }
        };

        Ok(quote! {
            #[automatically_derived]
            impl springtime_di::component::Injectable for #ident {}

            #[automatically_derived]
            impl springtime_di::component::ComponentDowncast<#ident> for #ident {
                fn downcast(
                    source: springtime_di::instance_provider::ComponentInstanceAnyPtr,
                ) -> Result<springtime_di::instance_provider::ComponentInstancePtr<Self>, springtime_di::instance_provider::ComponentInstanceAnyPtr> {
                    source.downcast()
                }
            }

            #[automatically_derived]
            impl springtime_di::component::Component for #ident {
                #create
            }

            const _: () = {
                use springtime_di::component::{Component, ComponentDowncast};
                use springtime_di::component_registry::ComponentMetadata;
                use springtime_di::component_registry::internal::{ComponentDefinitionRegisterer, submit, TypedComponentDefinition};
                use springtime_di::instance_provider::{ComponentInstanceAnyPtr, ComponentInstanceProvider, ComponentInstanceProviderError, ComponentInstancePtr};
                use std::any::{Any, TypeId, type_name};

                #constructor

                fn cast(instance: ComponentInstanceAnyPtr) -> Result<Box<dyn Any>, ComponentInstanceAnyPtr> {
                    #ident::downcast(instance).map(|p| Box::new(p) as Box<dyn Any>)
                }

                fn register() -> TypedComponentDefinition {
                    TypedComponentDefinition {
                        target: TypeId::of::<#ident>(),
                        target_name: type_name::<#ident>(),
                        condition: #condition,
                        priority: #priority,
                        metadata: ComponentMetadata {
                            names: [#(#names.to_string()),*].into_iter().collect(),
                            scope: #scope.to_string(),
                            constructor,
                            cast,
                        },
                    }
                }

                submit! {
                    ComponentDefinitionRegisterer {
                        register,
                    }
                };
            };
        })
    } else {
        Err(Error::new(
            input.span(),
            "Can only derive Component on structs!",
        ))
    }
}

pub fn register_component_alias(
    item: &Item,
    args: &ComponentAliasAttributes,
) -> Result<TokenStream> {
    if let Item::Impl(item_impl) = item {
        let trait_type = item_impl
            .trait_
            .as_ref()
            .map(|(_, path, ..)| path)
            .ok_or_else(|| Error::new(item.span(), "Missing trait identifier!"))?;

        let target_type = if let Type::Path(path) = item_impl.self_ty.deref() {
            &path.path
        } else {
            return Err(Error::new(
                item.span(),
                "Registering traits is only available for Components!",
            ));
        };

        let is_primary = args.is_primary;
        let condition = args
            .condition
            .as_ref()
            .map(|condition| quote!(Some(#condition)))
            .unwrap_or_else(|| quote!(None));
        let priority = args.priority;
        let scope = args
            .scope
            .as_ref()
            .map(|scope| quote!(Some(#scope.to_string())))
            .unwrap_or_else(|| quote!(None));

        #[cfg(feature = "threadsafe")]
        let trait_bounds = quote!( + Sync + Send);
        #[cfg(not(feature = "threadsafe"))]
        let trait_bounds = quote!();

        Ok(quote! {
            #[automatically_derived]
            impl springtime_di::component::ComponentDowncast<#target_type> for dyn #trait_type #trait_bounds {
                fn downcast(
                    source: springtime_di::instance_provider::ComponentInstanceAnyPtr,
                ) -> Result<springtime_di::instance_provider::ComponentInstancePtr<Self>, springtime_di::instance_provider::ComponentInstanceAnyPtr> {
                    source.downcast::<#target_type>().map(|p| p as springtime_di::instance_provider::ComponentInstancePtr<Self>)
                }
            }

            const _: () = {
                use springtime_di::component::ComponentDowncast;
                use springtime_di::component_registry::ComponentAliasMetadata;
                use springtime_di::component_registry::internal::{ComponentAliasDefinition, ComponentAliasRegisterer, submit};
                use springtime_di::instance_provider::ComponentInstanceAnyPtr;
                use std::any::{Any, TypeId, type_name};

                fn cast(instance: ComponentInstanceAnyPtr) -> Result<Box<dyn Any>, ComponentInstanceAnyPtr> {
                    <dyn #trait_type #trait_bounds as ComponentDowncast<#target_type>>::downcast(instance)
                        .map(|p| Box::new(p) as Box<dyn Any>)
                }

                fn register() -> ComponentAliasDefinition {
                    ComponentAliasDefinition {
                        alias_type: TypeId::of::<dyn #trait_type #trait_bounds>(),
                        target_type: TypeId::of::<#target_type>(),
                        alias_name: type_name::<dyn #trait_type #trait_bounds>(),
                        target_name: type_name::<#target_type>(),
                        condition: #condition,
                        priority: #priority,
                        metadata: ComponentAliasMetadata {
                            is_primary: #is_primary,
                            scope: #scope,
                            cast,
                        }
                    }
                }

                submit! {
                    ComponentAliasRegisterer {
                        register,
                    }
                };
            };
        })
    } else {
        Err(Error::new(
            item.span(),
            "Registering traits for components is possible only on trait implementations!",
        ))
    }
}
