//! Runs migration SQL and keeps the migration history table.
//!
//! This is the only place that knows about database backends.
//!
//! # Atomicity
//!
//! A migration's SQL runs first and its history row is saved afterwards, so
//! the two steps are not one transaction on any backend. A failure in
//! between can leave a migration applied but not recorded.

use std::collections::BTreeMap;

use rusql_alchemy::prelude::*;

use super::{
    Error, Result,
    store::{MigrationEntry, Version},
};

pub const HISTORY_TABLE: &str = RusqlAlchemyMigration::NAME;

// The struct name determines the table name, so it must not be renamed
// without a data migration for existing databases.
#[cfg(not(feature = "turso"))]
#[derive(Model, Clone, FromRow)]
struct RusqlAlchemyMigration {
    #[field(primary_key = true)]
    version: String,
    name: String,
    checksum: String,
}

#[cfg(feature = "turso")]
#[derive(Model, serde::Deserialize, Clone)]
struct RusqlAlchemyMigration {
    #[field(primary_key = true)]
    version: String,
    name: String,
    checksum: String,
}

/// Creates the history table if it does not exist yet.
pub async fn ensure_history_table(conn: &Connection) -> Result<()> {
    execute(RusqlAlchemyMigration::SCHEMA, conn).await
}

/// Checksums of the migrations that were already applied, by version.
pub async fn applied_checksums(conn: &Connection) -> Result<BTreeMap<Version, String>> {
    RusqlAlchemyMigration::all(conn)
        .await?
        .into_iter()
        .map(|record| {
            let version = record.version.parse::<Version>().map_err(|err| {
                format!(
                    "invalid version `{}` in migration history: {err}",
                    record.version
                )
            })?;

            Ok::<_, Error>((version, record.checksum))
        })
        .collect()
}

/// Runs a migration's `up` SQL and records it as applied.
pub async fn apply(
    conn: &Connection,
    migration: &MigrationEntry,
    up: &str,
    checksum: String,
) -> Result<()> {
    let record = RusqlAlchemyMigration {
        version: migration.version.to_string(),
        name: migration.name.clone(),
        checksum,
    };

    #[cfg(feature = "turso")]
    {
        let tx = conn.transaction().await?;
        tx.execute_batch(up).await?;
        record.save(conn).await?;
        tx.commit().await?;
    }

    #[cfg(not(feature = "turso"))]
    {
        let mut tx = conn.begin().await?;
        sqlx::raw_sql(up).execute(&mut *tx).await?;
        record.save(&mut *tx).await?;
        tx.commit().await?;
    }

    Ok(())
}

/// Executes a raw SQL script. Migration files are arbitrary DDL, so this
/// is the one place that cannot go through the ORM.
async fn execute(sql: &str, conn: &Connection) -> Result<()> {
    #[cfg(feature = "turso")]
    conn.execute_batch(sql).await?;

    #[cfg(not(feature = "turso"))]
    sqlx::raw_sql(sql).execute(conn).await?;

    Ok(())
}
