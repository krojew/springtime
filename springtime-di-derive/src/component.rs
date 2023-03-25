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
    FieldsNamed, FieldsUnnamed, Item, Lit, Result, Type,
};

const COMPONENT: &str = "component";

fn get_single_instance(ty: &Type) -> TokenStream {
    quote! {
        instance_provider.primary_instance::<<#ty as Deref>::Target>()?
    }
}

fn generate_construction(field: &Field) -> Result<TokenStream> {
    for attr in &field.attrs {
        if attr.path().is_ident(COMPONENT) {
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
            if attribute.path().is_ident(COMPONENT) {
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
                fn create<CIP: springtime_di::component::ComponentInstanceProvider>(instance_provider: &CIP) -> Result<Self, springtime_di::error::ComponentInstanceProviderError> {
                    use std::ops::Deref;
                    Ok(#generation)
                }
            }

            const _: () = {
                fn register() -> springtime_di::component_registry::internal::TypedComponentDefinition {
                    use std::any::TypeId;
                    springtime_di::component_registry::internal::TypedComponentDefinition {
                        target: TypeId::of::<#ident>(),
                        metadata: springtime_di::component_registry::ComponentMetadata {
                            names: vec![#(#names.to_string()),*],
                        }
                    }
                }

                springtime_di::component_registry::internal::submit! {
                    springtime_di::component_registry::internal::ComponentDefinitionRegisterer {
                        register
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

        Ok(quote! {
            #[automatically_derived]
            impl springtime_di::component::Injectable for dyn #trait_type {}

            #[automatically_derived]
            impl springtime_di::component::ComponentDowncast for dyn #trait_type {
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
                        trait_type: TypeId::of::<dyn #trait_type>(),
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
