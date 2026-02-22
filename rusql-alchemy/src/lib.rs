#[macro_use]
mod macros;
pub mod db;
pub mod prelude;
pub mod utils;

pub use async_trait;
pub use chrono;
pub use inventory;
#[cfg(feature = "turso")]
pub use libsql;
pub use rusql_alchemy_derive as derive;
#[cfg(not(feature = "turso"))]
pub use sqlx;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

type FutureResult<'fut, T, E = Error> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'fut>>;

pub struct MigrationRegistrar {
    pub up_fn: for<'m> fn(&'m db::Connection) -> FutureResult<'m, ()>,
    pub down_fn: for<'m> fn(&'m db::Connection) -> FutureResult<'m, ()>,
}

inventory::collect!(MigrationRegistrar);

/// Represents a database connection and provides methods for interacting with it.
///
/// The `Database` struct is the primary entry point for interacting with the database.
/// It holds the connection pool and provides methods for creating new connections,
/// running migrations, and performing other database-level operations.
pub struct Database {
    pub conn: db::Connection,
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

    pub async fn up(&self) -> Result<(), Error> {
        for model in inventory::iter::<MigrationRegistrar> {
            (model.up_fn)(&self.conn).await?;
        }
        Ok(())
    }

    pub async fn down(&self) -> Result<(), Error> {
        for model in inventory::iter::<MigrationRegistrar> {
            (model.down_fn)(&self.conn).await?;
        }
        Ok(())
    }
}
