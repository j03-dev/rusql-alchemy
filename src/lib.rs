#[macro_use]
mod macros;
pub mod db;
pub mod prelude;
pub mod types;
mod utils;

pub use db::models::PLACEHOLDER;
pub use utils::*;

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

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub async fn new() -> Self {
        dotenv::dotenv().ok();
        let database_url =
            std::env::var("DATABASE_URL").expect("-> Pls set the DATABASE_ULR in `.env`");
        Self {
            conn: establish_connection(database_url).await,
        }
    }
}
