pub mod model;
pub mod query;
pub mod types;

#[cfg(not(feature = "postgres"))]
pub const PLACEHOLDER: &str = "?";

#[cfg(feature = "postgres")]
pub const PLACEHOLDER: &str = "$";

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
