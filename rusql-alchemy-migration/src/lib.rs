//! Schema migrations.
//!
//! [`make_migration`] compares the schemas registered by the models with
//! the last saved snapshot and writes a new migration directory.
//! [`migrate`] applies, in order, every migration not yet applied.

mod diff;
mod executor;
mod schema;
mod store;

use rusql_alchemy::{Error, prelude::*};

use self::{
    schema::Schema,
    store::{MigrationDir, Version},
};

type Result<T> = std::result::Result<T, Error>;

/// Generates a migration in `directory` if the models changed since the
/// last one. Does nothing when the schema is unchanged.
pub fn make_migration(directory: &str) -> Result<()> {
    let migrations = MigrationDir::new(directory);
    migrations.ensure_exists()?;

    let existing = migrations.scan()?;
    let latest = existing.last();

    let current = current_schema();
    let previous = migrations.previous_schema(latest)?;

    if previous.trim() == current.trim() {
        return Ok(());
    }

    let changes = diff::between(&Schema::parse(&previous)?, &Schema::parse(&current)?);
    let version = latest.map_or(Version::FIRST, |latest| latest.version.next());

    migrations.write(version, &changes, &current)
}

/// Applies every pending migration found in `directory`.
///
/// Fails if a migration that was already applied has been edited since.
pub async fn migrate(database: &Database, directory: &str) -> Result<()> {
    let conn = &database.conn;

    executor::ensure_history_table(conn).await?;
    let applied = executor::applied_checksums(conn).await?;

    for migration in MigrationDir::new(directory).scan()? {
        let up = migration.read_up()?;
        let checksum = store::checksum(&up, &migration.read_down()?);

        match applied.get(&migration.version) {
            Some(stored) if *stored == checksum => {}
            Some(_) => {
                return Err(format!(
                    "migration {} was modified after being applied",
                    migration.name
                )
                .into());
            }
            None => executor::apply(conn, &migration, &up, checksum).await?,
        }
    }

    Ok(())
}

/// The schema declared by every registered model.
///
/// Sorted so that the registration order cannot cause a false migration.
fn current_schema() -> String {
    let mut schemas: Vec<&str> = inventory::iter::<Schemas>()
        .map(|model| model.schema)
        .collect();

    schemas.sort_unstable();
    schemas.join("\n")
}
