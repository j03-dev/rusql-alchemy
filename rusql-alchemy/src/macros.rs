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
            let value = $crate::db::query::BindValue::parse(&arg.value, &arg.ty)?;
            $stream = match value {
                $crate::db::query::BindValue::Int32(v) => $stream.bind(v),
                $crate::db::query::BindValue::Int64(v) => $stream.bind(v),
                $crate::db::query::BindValue::Real(v) => $stream.bind(v),
                $crate::db::query::BindValue::Text(v) => $stream.bind(v),
            };
        }
    }};

    ($args:expr) => {{
        use libsql::Value;
        let mut params = Vec::new();
        for arg in $args {
            let value = $crate::db::query::BindValue::parse(&arg.value, &arg.ty)?;
            params.push(match value {
                $crate::db::query::BindValue::Int32(v) => {
                    v.map(|n| Value::Integer(n as i64)).unwrap_or(Value::Null)
                }
                $crate::db::query::BindValue::Int64(v) => {
                    v.map(Value::Integer).unwrap_or(Value::Null)
                }
                $crate::db::query::BindValue::Real(v) => v.map(Value::Real).unwrap_or(Value::Null),
                $crate::db::query::BindValue::Text(v) => v.map(Value::Text).unwrap_or(Value::Null),
            });
        }
        libsql::params_from_iter(params)
    }};
}

/// Starts a [`SelectBuilder`](crate::db::query::statement::SelectBuilder).
///
/// * `select!(User)` — a single table; you can chain `.r#where(...)`.
/// * `select!(User, Profile)` — prepares a join query;
///   chain `.inner_join::<User, Profile>(kwargs!(User.id == Profile.user_id))`
///   (or `left_join` / `right_join`), then run it with `fetch_all` or
///   `fetch_one`. The output type selects `User`, `Profile`, or both as a
///   typed tuple. `fetch_all` returns an empty `Vec` when nothing matches;
///   `fetch_one` returns `Option<T>`.
///
/// Joined columns are decoded by position, so the joined models may have
/// columns with the same name (such as `id`).
///
/// # Example
///
/// ```ignore
/// use rusql_alchemy::prelude::*;
///
/// // Both joined models.
/// let rows: Vec<(User, Profile)> = select!(User, Profile)
///     .inner_join::<User, Profile>(kwargs!(User.id == Profile.user_id))
///     .fetch_all(&conn)
///     .await?;
///
/// // Only one side: the output type chooses the selected model.
/// let users: Vec<User> = select!(User, Profile)
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
