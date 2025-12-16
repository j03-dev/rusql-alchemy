pub fn get_type_name<T: Sized>(_: T) -> &'static str {
    std::any::type_name::<T>()
}

pub fn to_string(value: impl Into<serde_json::Value>) -> String {
    let json_value = value.into();
    match json_value {
        serde_json::Value::Bool(true) => serde_json::json!(1),
        serde_json::Value::Bool(false) => serde_json::json!(0),
        _ => json_value,
    }
    .to_string()
}

#[cfg(feature = "turso")]
pub async fn libsql_from_row<T>(mut rows: libsql::Rows) -> Result<Vec<T>, crate::Error>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let mut results = Vec::new();
    while let Some(row) = rows.next().await? {
        let s = libsql::de::from_row::<T>(&row)?;
        results.push(s);
    }
    Ok(results)
}
