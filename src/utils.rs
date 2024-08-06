use std::{any::type_name, io::Error};

/// Returns the name of the type `T` as a string.
///
/// # Arguments
///
/// * `value` - A value of type `T`.
///
/// # Returns
///
/// * A string slice that represents the name of the type `T`.
///
/// # Example
///
/// ```
/// let type_name = get_type_name(42);
/// assert_eq!(type_name, "i32");
/// ```
pub fn get_type_name<T: Sized>(_: T) -> &'static str {
    type_name::<T>()
}

/// Retrieves a placeholder for SQL queries based on the `DATABASE_URL` environment variable.
///
/// # Returns
///
/// * `Ok("?")` if the database is SQLite or MySQL.
/// * `Ok("$")` if the database is PostgreSQL.
/// * `Err` if the `DATABASE_URL` environment variable is not set or if the database type is unsupported.
///
/// # Errors
///
/// Returns an `std::io::Error` if the `DATABASE_URL` is not found or if the database type is unsupported.
///
/// # Example
///
/// ```
/// std::env::set_var("DATABASE_URL", "sqlite://database.db");
/// let placeholder = get_placeholder().unwrap();
/// assert_eq!(placeholder, "?");
/// ```
pub fn get_placeholder() -> std::io::Result<&'static str> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "DATABASE_URL is not found")
    })?;
    if database_url.starts_with("sqlite") || database_url.starts_with("mysql") {
        Ok("?")
    } else if database_url.starts_with("postgres") {
        Ok("$")
    } else {
        Err(Error::new(
            std::io::ErrorKind::InvalidData,
            "Unsupported database type",
        ))
    }
}

/// Converts a value into a JSON string.
///
/// # Arguments
///
/// * `value` - A value that can be converted into `serde_json::Value`.
///
/// # Returns
///
/// * A `String` representation of the JSON value.
/// * If the value is a boolean, it converts `true` to `1` and `false` to `0`.
///
/// # Example
///
/// ```
/// let json_string = to_string(true);
/// assert_eq!(json_string, "1");
///
/// let json_string = to_string("Hello");
/// assert_eq!(json_string, "\"Hello\"");
/// ```
pub fn to_string(value: impl Into<serde_json::Value>) -> String {
    let json_value = value.into();
    match json_value {
        serde_json::Value::Bool(true) => serde_json::json!(1),
        serde_json::Value::Bool(false) => serde_json::json!(0),
        _ => json_value,
    }
    .to_string()
}
