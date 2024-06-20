use std::any::type_name;

pub fn get_type_name<T: Sized>(_: T) -> &'static str {
    type_name::<T>()
}

pub fn get_placeholder() -> &'static str {
    let database_url = std::env::var("DATABASE_URL").unwrap();
    if database_url.starts_with("sqlite") || database_url.starts_with("mysql") {
        "?"
    } else if database_url.starts_with("postgres") {
        "$"
    } else {
        panic!("Unsupported database type");
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