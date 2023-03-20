use crate::attributes::{DefaultDefinition, FieldAttributes};
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{
    Data, DataStruct, DeriveInput, Error, Field, Fields, FieldsNamed, FieldsUnnamed, Result, Type,
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

pub fn expand_component(input: &DeriveInput) -> Result<TokenStream> {
    if let Data::Struct(DataStruct { fields, .. }) = &input.data {
        let ident = &input.ident;
        let generation = match fields {
            Fields::Named(fields) => make_named_struct(fields)?,
            Fields::Unnamed(fields) => make_unnamed_struct(fields)?,
            Fields::Unit => quote! { Self },
        };

        Ok(quote! {
            #[automatically_derived]
            impl springtime_di::component::Component for #ident {
                fn create<CIP: springtime_di::component::ComponentInstanceProvider>(instance_provider: &CIP) -> Result<Self, springtime_di::Error> {
                    use std::ops::Deref;
                    Ok(#generation)
                }
            }
        })
    } else {
        Err(Error::new(
            input.span(),
            "Can only derive Component on structs!",
        ))
    }
}
