use crate::attributes::{ComponentAttributes, DefaultDefinition, FieldAttributes};
use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DataStruct, DeriveInput, Error, Field, Fields, FieldsNamed, FieldsUnnamed,
    LitStr, Result, Type,
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

fn generate_name(attribute_name: &Option<LitStr>, ident: &Ident) -> String {
    attribute_name
        .as_ref()
        .map(|name| name.value())
        .unwrap_or_else(|| ident.to_string().to_case(Case::Snake))
}

pub fn expand_component(input: &DeriveInput) -> Result<TokenStream> {
    if let Data::Struct(DataStruct { fields, .. }) = &input.data {
        let ident = &input.ident;
        let generation = match fields {
            Fields::Named(fields) => make_named_struct(fields)?,
            Fields::Unnamed(fields) => make_unnamed_struct(fields)?,
            Fields::Unit => quote! { Self },
        };
        let (name, is_primary) = {
            if let Some(ComponentAttributes { name, is_primary }) =
                extract_component_attributes(&input.attrs)?
            {
                (name, is_primary)
            } else {
                (None, false)
            }
        };
        let name = generate_name(&name, &input.ident);

        Ok(quote! {
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
                        definition: springtime_di::component_registry::ComponentDefinition {
                            name: #name.to_string(),
                            is_primary: #is_primary
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
