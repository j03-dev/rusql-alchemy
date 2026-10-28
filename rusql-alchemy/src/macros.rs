/// A macro to create a `Vec<Kwargs>` of conditions for query methods like
/// `get`, `filter` and `set`.
///
/// Supported forms:
///
/// * `field = value`, `field == value`, `field != value`, `field < value`,
///   `field <= value`, `field > value`, `field >= value` — a column compared to a value.
/// * `table.column = other_table.other_column` — a column compared to another
///   column, for joins and `where` clauses.
///
/// Combine conditions with the `And` / `Or` traits: `kwargs!(a = 1).and(kwargs!(b == 2))`.
///
/// # Example
///
/// ```ignore
/// use rusql_alchemy::kwargs;
///
/// let conditions = kwargs!(
///     field1 = value1,
///     field2 == value2,
///     field3 != value3,
///     field4 < value4,
///     field5 <= value5,
///     field6 > value6,
///     field7 >= value7,
/// );
///
/// // Column to column, e.g. for a join
/// let on = kwargs!(User.id == Profile.user_id);
/// ```
#[macro_export]
macro_rules! kwargs {
    ($($field:ident = $value:expr),* $(,)?) => {
        {
            let mut args = Vec::new();
            $(
                args.push($crate::db::query::condition::Kwargs::Condition {
                    field: stringify!($field).to_string(),
                    value: $crate::utils::to_string($value.clone()),
                    value_type: $crate::utils::get_type_name($value.clone()).into(),
                    comparison_operator: "=".to_string(),
                });
            )*
            args
        }
    };

    ($table:ident.$column:ident $op:tt $v_table:ident.$v_column:ident) => {
        {
            vec![
                $crate::db::query::condition::Kwargs::Condition {
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
                $crate::db::query::condition::Kwargs::Condition {
                    field: format!("{}.{}", stringify!($table), stringify!($column)),
                    value: $crate::utils::to_string($value.clone()),
                    value_type: $crate::utils::get_type_name($value.clone()).into(),
                    comparison_operator: stringify!($op).to_string(),
                }
            ]
        }
    };

    ($field:ident $op:tt $value:expr) => {
        {
            vec![
                $crate::db::query::condition::Kwargs::Condition {
                    field: stringify!($field).to_string(),
                    value: $crate::utils::to_string($value.clone()),
                    value_type: $crate::utils::get_type_name($value.clone()).into(),
                    comparison_operator: stringify!($op).to_string(),
                }
            ]
        }
    };

}

macro_rules! binds {
    ($args:expr, $stream:expr) => {{
        for arg in $args {
            let value = arg.value.replace('"', "");
            let ty = arg.ty.replace('"', "");
            $stream = match ty.as_str() {
                "i32" | "bool" => $stream.bind(value.parse::<i32>()?),
                "f64" => $stream.bind(value.parse::<f64>()?),
                _ if ty.contains("Option") && value == "null" => {
                    $stream.bind(Option::<String>::None)
                }
                _ => $stream.bind(value),
            };
        }
    }};

    ($args:expr) => {{
        use libsql::Value;
        let mut params = Vec::new();
        for arg in $args {
            let value = arg.value.replace('"', "");
            let ty = arg.ty.replace('"', "");
            match ty.as_str() {
                "i32" | "bool" => params.push(Value::Integer(value.parse::<i64>()?)),
                "f64" => params.push(Value::Real(value.parse::<f64>()?)),
                _ if ty.contains("Option") && value == "null" => params.push(Value::Null),
                _ => params.push(Value::Text(value)),
            };
        }
        libsql::params_from_iter(params)
    }};
}

/// Starts a [`SelectBuilder`](crate::db::query::statement::SelectBuilder).
///
/// * `select!(User)` — a single table; you can chain `.r#where(...)`.
/// * `select!(User, Profile)` — a join query selecting `User.*` and `Profile.*`;
///   chain `.inner_join::<User, Profile>(kwargs!(User.id == Profile.user_id))`
///   (or `left_join` / `right_join`), then run it with `fetch_all`,
///   `fetch_one` or `fetch_optional` to get models or typed tuples.
///
/// Joined columns are decoded by position, so the joined models may have
/// columns with the same name (such as `id`).
///
/// # Example
///
/// ```ignore
/// use rusql_alchemy::prelude::*;
///
/// let rows: Vec<(User, Profile)> = select!(User, Profile)
///     .inner_join::<User, Profile>(kwargs!(User.id == Profile.user_id))
///     .fetch_all(&conn)
///     .await?;
///
/// // Left join: the optional side is wrapped in Option.
/// let rows: Vec<(User, Option<Profile>)> = select!(User, Profile)
///     .left_join::<User, Profile>(kwargs!(User.id == Profile.user_id))
///     .fetch_all(&conn)
///     .await?;
/// ```
#[macro_export]
macro_rules! select {
    ($table: ty) => {
        $crate::db::query::statement::SelectBuilder::new(Some(<$table>::NAME))
    };

    ($($table:ty),+) => {
        $crate::db::query::statement::SelectBuilder::new(None)
    };
}
