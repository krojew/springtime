use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use refinery_core::{find_migration_files, MigrationType};
use std::path::Path;
use syn::{Error, Result};

fn generate_migration(path: &Path, item_span: Span) -> Result<TokenStream> {
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

    Ok(quote! {
        Migration::unapplied(#filename, include_str!(#path))
            .map_err(|error| std::sync::Arc::new(error) as ErrorPtr)?
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
        .map(|path| {
            generate_migration(&path, item_span).map(|migration| {
                quote! {
                    #migration
                }
            })
        })
        .try_collect()
        .map(|migrations: Vec<_>| {
            quote! {
                #[automatically_derived]
                mod migrations {
                    use springtime::future::{BoxFuture, FutureExt};
                    use springtime::runner::ErrorPtr;
                    use springtime_di::{component_alias, Component};
                    use springtime_migrate_refinery::migration::MigrationSource;
                    use springtime_migrate_refinery::refinery::Migration;

                    #[derive(Component)]
                    struct GenratedMigrationSource;

                    #[component_alias]
                    impl MigrationSource for GenratedMigrationSource {
                        fn migrations(&self) -> Result<Vec<Migration>, ErrorPtr> {
                            Ok(vec![#(#migrations),*])
                        }
                    }
                }
            }
        })
}
