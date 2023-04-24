use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use refinery_core::{find_migration_files, MigrationType};
use std::path::Path;
use syn::{Error, Result};

fn generate_migration(index: usize, path: &Path, item_span: Span) -> Result<TokenStream> {
    let filename = path
        .file_stem()
        .and_then(|file| file.to_os_string().into_string().ok())
        .ok_or_else(|| {
            Error::new(
                item_span,
                format!("Cannot extract migration name: {}", path.display()),
            )
        })?;

    let path = path.display().to_string();
    let struct_name = format!("Migration{index}");
    let ctor_name = format!("{struct_name}::new");
    let struct_ident = Ident::new(&struct_name, Span::call_site());

    Ok(quote! {
        #[derive(Component)]
        #[component(constructor = #ctor_name)]
        struct #struct_ident {
            #[component(ignore)]
            name: &'static str,
            #[component(ignore)]
            sql: &'static str,
        }

        impl #struct_ident {
            fn new() -> BoxFuture<'static, Result<Self, ErrorPtr>> {
                async {
                    Ok(Self {
                        name: #filename,
                        sql: include_str!(#path),
                    })
                }.boxed()
            }
        }

        #[component_alias]
        impl MigrationSource for #struct_ident {
            fn migration(&self) -> Result<Migration, ErrorPtr> {
                Migration::unapplied(self.name, self.sql)
                    .map_err(|error| std::sync::Arc::new(error) as ErrorPtr)
            }
        }
    })
}

pub fn generate_migrations(path: &str, item_span: Span) -> Result<TokenStream> {
    let files = find_migration_files(path, MigrationType::Sql).map_err(|error| {
        Error::new(
            item_span,
            format!("Error looking for migrations in {path}: {error}"),
        )
    })?;

    files
        .enumerate()
        .try_fold(quote!(), |migrations, (index, path)| {
            generate_migration(index, &path, item_span).map(|migration| {
                quote! {
                    #migrations
                    #migration
                }
            })
        })
        .map(|migrations| {
            quote! {
                mod migrations {
                    use refinery_core::Migration;
                    use springtime::future::{BoxFuture, FutureExt};
                    use springtime::runner::ErrorPtr;
                    use springtime_di::{component_alias, Component};
                    use springtime_migrate_refinery::migration::MigrationSource;

                    #migrations
                }
            }
        })
}
