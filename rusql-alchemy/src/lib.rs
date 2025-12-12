/// This module contains the macros used in the crate.
#[macro_use]
mod macros;

/// This module contains the database-related functionality.
pub mod db;

/// This module contains the prelude for the crate.
pub mod prelude;

/// This module contains the custom types used in the crate.
pub mod types;

pub mod utils;

use std::{future::Future, pin::Pin};

pub use async_trait;
pub use chrono;
pub use inventory;
pub use rusql_alchemy_derive as derive;

#[cfg(feature = "turso")]
pub use libsql;
#[cfg(not(feature = "turso"))]
pub use sqlx;

#[cfg(not(feature = "turso"))]
/// A type alias for the database connection pool.
///
/// When the `turso` feature is not enabled, this is a `sqlx::Pool<sqlx::Any>`.
pub type Connection = sqlx::Pool<sqlx::Any>;

#[cfg(feature = "turso")]
/// A type alias for the database connection.
///
/// When the `turso` feature is enabled, this is a `libsql::Connection`.
pub type Connection = libsql::Connection;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

type FutRes<'fut, T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'fut>>;

type MigrateFn = for<'m> fn(&'m Connection) -> FutRes<'m, (), Error>;

pub struct MigrationRegistrar {
    pub migrate_fn: MigrateFn,
}

inventory::collect!(MigrationRegistrar);

/// Represents a database connection and provides methods for interacting with it.
///
/// The `Database` struct is the primary entry point for interacting with the database.
/// It holds the connection pool and provides methods for creating new connections,
/// running migrations, and performing other database-level operations.
pub struct Database {
    pub conn: Connection,
}

impl Database {
    /// Creates a new database connection.
    ///
    /// This method is only available when the `turso` feature is **not** enabled.
    /// It uses `sqlx` to connect to the database specified by the `database_url`.
    ///
    /// # Arguments
    ///
    /// * `database_url` - The connection string for the database.
    ///
    /// # Returns
    ///
    /// A `Result` containing a new `Database` instance on success, or an `Error` on failure.
    #[cfg(not(feature = "turso"))]
    pub async fn new(database_url: &str) -> Result<Self, Error> {
        sqlx::any::install_default_drivers();
        let conn = sqlx::any::AnyPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Ok(Self { conn })
    }

    /// Creates a new local database connection using Turso.
    ///
    /// This method is only available when the `turso` feature is enabled.
    /// It creates a local database file at the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path for the local database.
    ///
    /// # Returns
    ///
    /// A `Result` containing a new `Database` instance on success, or an `Error` on failure.
    #[cfg(feature = "turso")]
    pub async fn new_local(path: &str) -> Result<Self, Error> {
        let db = libsql::Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        Ok(Self { conn })
    }

    /// Creates a new remote replica database connection using Turso.
    ///
    /// This method is only available when the `turso` feature is enabled.
    /// It connects to a remote Turso database as a replica, using a local file for caching.
    ///
    /// # Arguments
    ///
    /// * `path` - The local file path for the replica database.
    /// * `database_url` - The URL of the remote Turso database.
    /// * `auth_token` - The authentication token for the remote database.
    ///
    /// # Returns
    ///
    /// A `Result` containing a new `Database` instance on success, or an `Error` on failure.
    #[cfg(feature = "turso")]
    pub async fn new_remote_replica(
        path: &str,
        database_url: &str,
        auth_token: &str,
    ) -> Result<Self, Error> {
        let db = libsql::Builder::new_remote_replica(
            path,
            database_url.to_string(),
            auth_token.to_string(),
        )
        .build()
        .await?;
        let conn = db.connect()?;
        Ok(Self { conn })
    }

    /// Creates a new remote database connection using Turso.
    ///
    /// This method is only available when the `turso` feature is enabled.
    /// It connects directly to a remote Turso database.
    ///
    /// # Arguments
    ///
    /// * `database_url` - The URL of the remote Turso database.
    /// * `auth_token` - The authentication token for the remote database.
    ///
    /// # Returns
    ///
    /// A `Result` containing a new `Database` instance on success, or an `Error` on failure.
    #[cfg(feature = "turso")]
    pub async fn new_remote(database_url: &str, auth_token: &str) -> Result<Self, Error> {
        let db = libsql::Builder::new_remote(database_url.to_string(), auth_token.to_string())
            .build()
            .await?;
        let conn = db.connect()?;
        Ok(Self { conn })
    }

    /// Runs database migrations.
    ///
    /// This method iterates over all registered models and applies their migrations
    /// to the database. For migrations to be discovered, the models must be
    /// imported into the binary where this method is called.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // In your main.rs
    /// use rusql_alchemy::prelude::*;
    /// use rusql_alchemy::Error;
    ///
    /// // Import your models so they can be discovered for migration.
    /// #[allow(unused_imports)]
    /// use crate::models::*;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let database = Database::new_local("local.db").await?;
    ///     database.migrate().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn migrate(&self) -> Result<(), Error> {
        for model in inventory::iter::<MigrationRegistrar> {
            (model.migrate_fn)(&self.conn).await?;
        }
        Ok(())
    }
}
