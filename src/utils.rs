use std::{any::type_name, io::Error};

pub fn get_type_name<T: Sized>(_: T) -> &'static str {
    type_name::<T>()
}

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

pub fn to_value(value: impl Into<serde_json::Value>) -> serde_json::Value {
    let json_value = value.into();
    match json_value {
        serde_json::Value::Bool(true) => serde_json::json!(1),
        serde_json::Value::Bool(false) => serde_json::json!(0),
        _ => json_value,
    }
}
