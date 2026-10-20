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
        Command::Rollback => match rollback(database, directory).await? {
            Some(canceled) => println!("Revert migration {canceled}"),
            None => println!("Nothing changed"),
        },
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

    let changes = diff::between(
        &Schema::parse(&previous)?.without(executor::HISTORY_TABLE),
        &Schema::parse(&current)?.without(executor::HISTORY_TABLE),
    );
    if changes.is_empty() {
        return Ok(None);
    }
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
    let applied = executor::applied(conn).await?;

    for migration in MigrationDir::new(directory).scan()? {
        let up = migration.read_up()?;
        let down = migration.read_down()?;
        let checksum = store::checksum(&up, &down);

        match applied.get(&migration.version) {
            Some(stored) if stored.checksum == checksum => {}
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

/// Reverts the latest applied migration and returns its name, or `None`
/// when nothing is applied.
///
/// Runs its `down.sql` and removes its history row in one transaction.
/// Fails if its directory is missing or its files were edited since.
pub async fn rollback(database: &Database, directory: &str) -> Result<Option<String>> {
    let conn = &database.conn;

    executor::get_last_migration(conn).await?;
    let applied = executor::applied(conn).await?;

    let Some((version, last)) = applied.last_key_value() else {
        return Ok(None);
    };

    let migration = MigrationDir::new(directory)
        .scan()?
        .into_iter()
        .find(|migration| migration.version == *version)
        .ok_or_else(|| format!("migration {version} is applied but its directory missing"))?;

    let down = migration.read_down()?;
    let up = migration.read_up()?;

    if store::checksum(&up, &down) != last.checksum {
        return Err(format!(
            "migration {} was modified after being applied",
            migration.name
        )
        .into());
    }

    executor::revert(conn, last, &down).await?;

    Ok(Some(migration.name))
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
