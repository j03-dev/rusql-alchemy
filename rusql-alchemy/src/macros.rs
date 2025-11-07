/// A macro to create a vector of `Condition::FieldCondition` for different comparison operators.
///
/// This macro supports generating conditions for field-value pairs using various comparison operators:
/// `=`, `==`, `!=`, `<`, `<=`, `>`, `>=`.
///
/// # Example
///
/// ```
/// let conditions = kwargs!(
///     field1 = value1,
///     field2 == value2,
///     field3 != value3,
///     field4 < value4,
///     field5 <= value5,
///     field6 > value6,
///     field7 >= value7,
/// );
/// ```
///
/// # Variants
///
/// - `$field:ident = $value:expr`
/// - `$field:ident == $value:expr`
/// - `$field:ident != $value:expr`
/// - `$field:ident < $value:expr`
/// - `$field:ident <= $value:expr`
/// - `$field:ident > $value:expr`
/// - `$field:ident >= $value:expr`
#[macro_export]
macro_rules! kwargs {
    // Support for direct field-value pairs with custom comparison operators
    ($($field:ident = $value:expr),* $(,)?) => {
        {
            let mut args = Vec::new();
            $(
                args.push($crate::Kwargs::Condition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: "=".to_string(),
                });
            )*
            args
        }
    };
    
    ($table:ident.$column:ident $op:tt $v_table:ident.$v_column:ident) => {
        {
            vec![
                $crate::Kwargs::Condition {
                    field: format!("{}.{}", stringify!($table), stringify!($column)),
                    value: format!("{}.{}", stringify!($v_table), stringify!($v_column)),
                    value_type: "column".into(),
                    comparison_operator: stringify!($op).to_string(),
                }
            ]
        }
    };

    ($table:ident.$column:ident $op:tt $value:expr) => {
        {
            vec![
                $crate::Kwargs::Condition {
                    field: format!("{}.{}", stringify!($table), stringify!($column)),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: stringify!($op).to_string(),
                }
            ]
        }
    };
    
    ($field:ident $op:tt $value:expr) => {
        {
            vec![
                $crate::Kwargs::Condition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: stringify!($op).to_string(),
                }
            ]
        }
    };

}

/// A macro to bind arguments to a stream based on their type.
///
/// This macro iterates over a list of `(value, type)` pairs and binds each value to the stream
/// according to its type. Supported types are `i32`, `bool`, and `f64`. All other types are bound as strings.
///
/// # Arguments
///
/// * `$args:expr` - A list of `(value, type)` pairs.
/// * `$stream:expr` - The stream to which the values will be bound.
///
/// # Example
///
/// ```
/// let args = vec![
///     ("42".to_string(), "i32".to_string()),
///     ("3.14".to_string(), "f64".to_string()),
///     ("true".to_string(), "bool".to_string()),
/// ];
/// let stream = some_stream();
/// binds!(args, stream);
/// ```
macro_rules! binds {
    ($args:expr, $stream:expr) => {{
        for arg in $args {
            let value = arg.value.replace('"', "");
            let ty = arg.ty.replace('"', "");
            if ty == "i32" || ty == "bool" {
                $stream = $stream.bind(value.parse::<i32>().unwrap());
            } else if ty == "f64" {
                $stream = $stream.bind(value.parse::<f64>().unwrap());
            } else if ty.contains("Option") && value == "null" {
                $stream = $stream.bind(Option::<String>::None);
            } else {
                $stream = $stream.bind(value);
            }
        }
    }};

    ($args:expr) => {{
        use libsql::Value;
        let mut params = Vec::new();
        for arg in $args {
            let value = arg.value.replace('"', "");
            let ty = arg.ty.replace('"', "");
            if ty == "i32" || ty == "bool" {
                params.push(Value::Integer(value.parse::<i64>().unwrap()));
            } else if ty == "f64" {
                params.push(Value::Real(value.parse::<f64>().unwrap()));
            } else if ty.contains("Option") && value == "null" {
                params.push(Value::Null);
            } else {
                params.push(Value::Text(value));
            }
        }
        libsql::params_from_iter(params)
    }};
}

#[macro_export]
macro_rules! select {
    ($($table:ty),*) => {
        $crate::db::Statement(format!("SELECT {}", { let table_names = [$(format!("{}.*", stringify!($table))),*]; table_names.join(", ") }))
    };
}
