use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DataStruct, DeriveInput, Error, Fields, FieldsNamed, FieldsUnnamed, Result, Type};

fn get_single_instance(ty: &Type) -> TokenStream {
    quote! {
        instance_provider.primary_instance::<<#ty as Deref>::Target>()?
    }
}

fn make_named_struct(fields: &FieldsNamed) -> TokenStream {
    let fields = fields
        .named
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            let instance = get_single_instance(&field.ty);
            quote! {
                #ident: #instance
            }
        })
        .collect_vec();

    quote! {
        Self {
            #(#fields)*
        }
    }
}

fn make_unnamed_struct(fields: &FieldsUnnamed) -> TokenStream {
    let fields = fields
        .unnamed
        .iter()
        .map(|field| {
            let instance = get_single_instance(&field.ty);
            quote! {
                #instance
            }
        })
        .collect_vec();

    quote! {
        Self(#(#fields)*)
    }
}

pub fn expand_component(input: &DeriveInput) -> Result<TokenStream> {
    if let Data::Struct(DataStruct { fields, .. }) = &input.data {
        let ident = &input.ident;
        let generation = match fields {
            Fields::Named(fields) => make_named_struct(fields),
            Fields::Unnamed(fields) => make_unnamed_struct(fields),
            Fields::Unit => quote! { Self },
        };

        Ok(quote! {
            #[automatically_derived]
            impl springtime_di::component::Component for #ident {
                fn create<CIP: springtime_di::component::ComponentInstanceProvider>(instance_provider: &CIP) -> Result<springtime_di::component::ComponentInstancePtr<Self>, springtime_di::Error> {
                    use std::ops::Deref;
                    Ok(springtime_di::component::ComponentInstancePtr::new(#generation))
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
