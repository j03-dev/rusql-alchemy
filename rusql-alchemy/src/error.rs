//! Concrete error type returned by rusql-alchemy operations.

/// Errors returned by rusql-alchemy operations.
///
/// The database variant is chosen at compile time from the enabled feature, so
/// callers can match on a concrete error without a boxed trait object.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(not(feature = "turso"))]
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[cfg(feature = "turso")]
    #[error(transparent)]
    Libsql(#[from] libsql::Error),

    #[error(transparent)]
    Serde(#[from] serde::de::value::Error),

    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),

    #[error("{0}")]
    Message(String),
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Self::Message(message.to_owned())
    }
}
