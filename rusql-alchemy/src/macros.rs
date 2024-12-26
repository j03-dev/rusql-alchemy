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
                args.push(Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: "=".to_string(),
                });
            )*
            args
        }
    };
    ($field:ident == $value:expr) => {
        {
            vec![
                Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: "=".to_string(),
                }
            ]
        }
    };
    ($field:ident != $value:expr) => {
        {
            vec![
                Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: "!=".to_string(),
                }
            ]
        }
    };
    ($field:ident < $value:expr) => {
        {
            vec![
                Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: "<".to_string(),
                }
            ]
        }
    };
    ($field:ident <= $value:expr) => {
        {
            vec![
                Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: "<=".to_string(),
                }
            ]
        }
    };
    ($field:ident > $value:expr) => {
        {
            vec![
                Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: ">".to_string(),
                }
            ]
        }
    };
    ($field:ident >= $value:expr) => {
        {
            vec![
                Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: ">=".to_string(),
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
    ($args: expr, $stream:expr) => {
        for (v, t) in $args {
            let v = v.replace('"', "");
            match t.as_str() {
                "i32" | "bool" => {
                    $stream = $stream.bind(v.parse::<i32>().unwrap());
                }
                "f64" => {
                    $stream = $stream.bind(v.parse::<f64>().unwrap());
                }
                _ => {
                    $stream = $stream.bind(v);
                }
            }
        }
    };
}

/// A macro to run the `migrate` function for multiple structs asynchronously.
///
/// This macro accepts a list of structs and a connection, and calls the `migrate` function
/// on each struct with the given connection.
///
/// # Arguments
///
/// * `[$($struct:ident),*]` - A list of structs to migrate.
/// * `$conn:expr` - The connection to be used for migration.
///
/// # Example
///
/// ```
/// migrate!([User, Product, Order], conn);
/// ```
///
/// This will call `User::migrate(conn).await`, `Product::migrate(conn).await`, and `Order::migrate(conn).await`.
#[macro_export]
macro_rules! migrate {
    ([$($struct:ident),*], $conn:expr) => {
        $( $struct::migrate($conn).await; )*
    };
}
