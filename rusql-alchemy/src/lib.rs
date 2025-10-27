/// This module contains the macros used in the crate.
#[macro_use]
mod macros;

/// This module contains the database-related functionality.
pub mod db;

/// This module contains the prelude for the crate.
pub mod prelude;

/// This module contains the custom types used in the crate.
pub mod types;

mod utils;

pub use db::models::PLACEHOLDER;
pub use utils::*;

use std::{future::Future, pin::Pin};

#[cfg(not(feature = "turso"))]
pub type Connection = sqlx::Pool<sqlx::Any>;

#[cfg(feature = "turso")]
pub type Connection = libsql::Connection;

#[cfg(feature = "turso")]
pub use libsql::params;

/// Represents a database.
pub struct Database {
    pub conn: Connection,
}

impl Database {
    #[cfg(not(feature = "turso"))]
    pub async fn new(database_url: &str) -> Result<Self, Error> {
        sqlx::any::install_default_drivers();
        let conn = sqlx::any::AnyPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Ok(Self { conn })
    }

    #[cfg(feature = "turso")]
    pub async fn new_local(path: &str) -> Result<Self, Error> {
        let db = libsql::Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        Ok(Self { conn })
    }

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

    #[cfg(feature = "turso")]
    pub async fn new_remote(database_url: &str, auth_token: &str) -> Result<Self, Error> {
        let db = libsql::Builder::new_remote(database_url.to_string(), auth_token.to_string())
            .build()
            .await?;
        let conn = db.connect()?;
        Ok(Self { conn })
    }

    pub async fn migrate(&self) -> Result<(), Error> {
        for model in inventory::iter::<MigrationRegistrar> {
            (model.migrate_fn)(&self.conn).await?;
        }
        Ok(())
    }
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;

type FutRes<'fut, T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'fut>>;

type MigrateFn = for<'m> fn(&'m Connection) -> FutRes<'m, (), Error>;

pub struct MigrationRegistrar {
    pub migrate_fn: MigrateFn,
}

inventory::collect!(MigrationRegistrar);
