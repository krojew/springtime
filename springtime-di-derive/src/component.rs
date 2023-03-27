use crate::attributes::{
    ComponentAliasAttributes, ComponentAttributes, DefaultDefinition, FieldAttributes,
};
use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::ops::Deref;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DataStruct, DeriveInput, Error, Expr, ExprArray, ExprLit, Field, Fields,
    FieldsNamed, FieldsUnnamed, GenericArgument, Item, Lit, PathArguments, Result, Type, TypePath,
    TypeTraitObject,
};

const COMPONENT_ATTR: &str = "component";

fn ungroup(mut ty: &Type) -> &Type {
    while let Type::Group(group) = ty {
        ty = &group.elem;
    }

    ty
}

fn get_injected_option_type(ty: &Type) -> Option<TokenStream> {
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

    if seg.ident != "Option" || args.len() != 1 {
        return None;
    }

    if let GenericArgument::Type(Type::Path(TypePath { path, .. })) = &args[0] {
        if let Some(last_segment) = path.segments.last() {
            if last_segment.ident == "ComponentInstancePtr" {
                if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    if let Some(GenericArgument::Type(Type::Path(path))) = args.args.first() {
                        return Some(quote!(#path));
                    }
                }
            }
        }
    }

    None
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

fn get_single_instance(ty: &Type) -> TokenStream {
    let (getter, ty) = get_injected_option_type(ty)
        .map(|ty| (quote!(primary_instance_option), ty))
        .unwrap_or_else(|| (quote!(primary_instance_typed), get_injected_type(ty)));

    quote! {
        instance_provider.#getter::<#ty>()?
    }
}

fn generate_construction(field: &Field) -> Result<TokenStream> {
    for attr in &field.attrs {
        if attr.path().is_ident(COMPONENT_ATTR) {
            let attributes = FieldAttributes::try_from(attr)?;
            match &attributes.default {
                Some(DefaultDefinition::Expr(path)) => return Ok(quote!(#path())),
                Some(DefaultDefinition::Default) => {
                    return Ok(quote!(std::default::Default::default()))
                }
                _ => {}
            }
        }
    }

    Ok(get_single_instance(&field.ty))
}

fn make_named_struct(fields: &FieldsNamed) -> Result<TokenStream> {
    let fields: Vec<_> = fields
        .named
        .iter()
        .map(|field| -> Result<TokenStream> {
            let ident = field.ident.as_ref().unwrap();
            let instance = generate_construction(field)?;
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
            let instance = generate_construction(field)?;
            Ok(quote! {
                #instance
            })
        })
        .try_collect()?;

    Ok(quote! {
        Self(#(#fields),*)
    })
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

pub fn expand_component(input: &DeriveInput) -> Result<TokenStream> {
    if let Data::Struct(DataStruct { fields, .. }) = &input.data {
        let ident = &input.ident;
        let generation = match fields {
            Fields::Named(fields) => make_named_struct(fields)?,
            Fields::Unnamed(fields) => make_unnamed_struct(fields)?,
            Fields::Unit => quote! { Self },
        };
        let names = {
            if let Some(ComponentAttributes { names }) = extract_component_attributes(&input.attrs)?
            {
                names
            } else {
                None
            }
        };
        let names = generate_names(names, &input.ident);

        Ok(quote! {
            #[automatically_derived]
            impl springtime_di::component::Injectable for #ident {}

            #[automatically_derived]
            impl springtime_di::component::ComponentDowncast for #ident {
                fn downcast(
                    source: springtime_di::component::ComponentInstanceAnyPtr,
                ) -> Result<springtime_di::component::ComponentInstancePtr<Self>, springtime_di::component::ComponentInstanceAnyPtr> {
                    source.downcast()
                }
            }

            #[automatically_derived]
            impl springtime_di::component::Component for #ident {
                fn create(instance_provider: &dyn springtime_di::component::ComponentInstanceProvider) -> Result<Self, springtime_di::error::ComponentInstanceProviderError> {
                    use springtime_di::component::TypedComponentInstanceProvider;
                    use std::ops::Deref;
                    Ok(#generation)
                }
            }

            const _: () = {
                fn constructor(instance_provider: &dyn springtime_di::component::ComponentInstanceProvider) -> Result<springtime_di::component::ComponentInstanceAnyPtr, springtime_di::error::ComponentInstanceProviderError> {
                    use springtime_di::component::Component;
                    #ident::create(instance_provider).map(|p| springtime_di::component::ComponentInstancePtr::new(p) as springtime_di::component::ComponentInstanceAnyPtr)
                }

                fn register() -> springtime_di::component_registry::internal::TypedComponentDefinition {
                    use std::any::TypeId;
                    springtime_di::component_registry::internal::TypedComponentDefinition {
                        target: TypeId::of::<#ident>(),
                        metadata: springtime_di::component_registry::ComponentMetadata {
                            names: vec![#(#names.to_string()),*],
                            constructor,
                        },
                    }
                }

                springtime_di::component_registry::internal::submit! {
                    springtime_di::component_registry::internal::ComponentDefinitionRegisterer {
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

        #[cfg(feature = "threadsafe")]
        let trait_bounds = quote!( + Sync + Send);
        #[cfg(not(feature = "threadsafe"))]
        let trait_bounds = quote!();

        Ok(quote! {
            #[automatically_derived]
            impl springtime_di::component::Injectable for dyn #trait_type #trait_bounds {}

            #[automatically_derived]
            impl springtime_di::component::ComponentDowncast for dyn #trait_type #trait_bounds {
                fn downcast(
                    source: springtime_di::component::ComponentInstanceAnyPtr,
                ) -> Result<springtime_di::component::ComponentInstancePtr<Self>, springtime_di::component::ComponentInstanceAnyPtr> {
                    source.downcast::<#target_type>().map(|p| p as springtime_di::component::ComponentInstancePtr<Self>)
                }
            }

            const _: () = {
                fn register() -> springtime_di::component_registry::internal::TraitComponentDefinition {
                    use std::any::TypeId;
                    springtime_di::component_registry::internal::TraitComponentDefinition {
                        trait_type: TypeId::of::<dyn #trait_type #trait_bounds>(),
                        target_type: TypeId::of::<#target_type>(),
                        metadata: springtime_di::component_registry::ComponentAliasMetadata {
                            is_primary: #is_primary
                        }
                    }
                }

                springtime_di::component_registry::internal::submit! {
                    springtime_di::component_registry::internal::TraitComponentRegisterer {
                        register
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
