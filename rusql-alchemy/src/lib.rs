/// This module contains the macros used in the crate.
#[macro_use]
mod macros;

/// This module contains the database-related functionality.
pub mod db;

/// This module contains the prelude for the crate.
pub mod prelude;

/// This module contains the custom types used in the crate.
pub mod types;

/// The placeholder for the database query.
pub use db::models::PLACEHOLDER;
pub use utils::*;

use anyhow::Result;

mod utils;

/// Alias for the database connection pool.
pub type Connection = sqlx::Pool<sqlx::Any>;

use sqlx::any::{install_default_drivers, AnyPoolOptions};

async fn establish_connection(url: String) -> Result<Connection> {
    install_default_drivers();
    let conn = AnyPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;
    Ok(conn)
}

/// Represents a database.
pub struct Database {
    /// The connection pool for the database.
    pub conn: Connection,
}

impl Database {
    /// Creates a new instance of `Database`.
    ///
    /// # Returns
    ///
    /// Returns a new `Database` instance.
    ///
    /// # Example
    /// ```rust
    /// use rusql_alchemy::Database;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let db = Database::new().await;
    /// }
    /// ```
    pub async fn new() -> Result<Self> {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")?;
        let conn = establish_connection(database_url).await?;
        Ok(Self { conn })
    }

    pub async fn migrate(&self) -> Result<()> {
        for model in inventory::iter::<MigrationRegistrar> {
            (model.migrate_fn)(self.conn.clone()).await?;
        }
        Ok(())
    }
}

pub struct MigrationRegistrar {
    pub migrate_fn: fn(
        Connection,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), sqlx::Error>> + Send + 'static>,
    >,
}
inventory::collect!(MigrationRegistrar);
