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

mod utils;

/// Alias for the database connection pool.
pub type Connection = sqlx::Pool<sqlx::Any>;

use sqlx::any::{install_default_drivers, AnyPoolOptions};

async fn establish_connection(url: String) -> Connection {
    install_default_drivers();
    AnyPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .unwrap()
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
    pub async fn new() -> Self {
        dotenv::dotenv().ok();
        let database_url =
            std::env::var("DATABASE_URL").expect("-> Pls set the DATABASE_ULR in `.env`");
        Self {
            conn: establish_connection(database_url).await,
        }
    }
}
