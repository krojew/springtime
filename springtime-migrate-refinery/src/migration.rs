//! Bridge between *Springtime* and `refinery` migrations.

#[cfg(test)]
use mockall::automock;
use refinery_core::Migration;
use springtime::runner::ErrorPtr;
use springtime_di::injectable;

/// Embed migrations from a given path (`migrations` by default). Path is inspected for `*.sql`
/// files, which are converted into [MigrationSources](MigrationSource).
///
/// ```ignore
/// use springtime_migrate_refinery::migration::embed_migrations;
/// embed_migrations!("path/to/migrations");
/// ```
pub use springtime_migrate_refinery_macros::embed_migrations;

/// A source for [Migrations](Migration).
#[injectable]
#[cfg_attr(test, automock)]
pub trait MigrationSource {
    /// Provides a migration from this source.
    fn migrations(&self) -> Result<Vec<Migration>, ErrorPtr>;
}
