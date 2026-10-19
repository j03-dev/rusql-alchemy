//! Schema migrations.
//!
//! [`make_migration`] compares the schemas registered by the models with
//! the last saved snapshot and writes a new migration directory.
//! [`migrate`] applies, in order, every migration not yet applied.
//! [`cli`] exposes both as a small command line.

mod command;
mod diff;
mod executor;
mod schema;
mod store;

use rusql_alchemy::{Error, prelude::*};

use self::{
    command::Command,
    schema::Schema,
    store::{MigrationDir, Version},
};

type Result<T> = std::result::Result<T, Error>;

/// Runs the migration command line: `make-migration [name]`, `migrate`
/// or `help`.
///
/// `args` are the arguments after the program name:
///
/// ```ignore
/// migration::cli(&database, "migrations", std::env::args().skip(1)).await?;
/// ```
pub async fn cli(
    database: &Database,
    directory: &str,
    args: impl IntoIterator<Item = String>,
) -> Result<()> {
    match Command::parse(args)? {
        Command::MakeMigration { name } => match make_migration(directory, name.as_deref())? {
            Some(created) => println!("Created migration {created}"),
            None => println!("No changes detected"),
        },
        Command::Migrate => {
            migrate(database, directory).await?;
            println!("Migrations applied");
        }
        Command::Help => println!("{}", command::USAGE),
    }

    Ok(())
}

/// Generates a migration in `directory` if the models changed since the
/// last one, and returns its directory name.
///
/// `name` is free text such as `"first migration"`; it is normalized
/// (`first_migration`) and defaults to `schema_change`. Returns `None`
/// when the schema is unchanged and nothing was written.
pub fn make_migration(directory: &str, name: Option<&str>) -> Result<Option<String>> {
    let migrations = MigrationDir::new(directory);
    migrations.ensure_exists()?;

    let existing = migrations.scan()?;
    let latest = existing.last();

    let current = current_schema();
    let previous = migrations.previous_schema(latest)?;

    if previous.trim() == current.trim() {
        return Ok(None);
    }

    let changes = diff::between(&Schema::parse(&previous)?, &Schema::parse(&current)?);
    let version = latest.map_or(Version::FIRST, |latest| latest.version.next());

    migrations
        .write(version, name.unwrap_or_default(), &changes, &current)
        .map(Some)
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
