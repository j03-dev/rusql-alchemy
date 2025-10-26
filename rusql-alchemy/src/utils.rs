use std::any::type_name;

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
