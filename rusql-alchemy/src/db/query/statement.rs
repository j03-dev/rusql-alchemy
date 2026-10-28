//! Fluent `SELECT` builder used by the `select!` macro.

use super::{Arg, builder, condition::Kwargs};
use crate::{
    Error,
    db::{Connection, model::Model},
};

pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

impl std::fmt::Display for JoinType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let join_type_name = match self {
            JoinType::Inner => "INNER",
            JoinType::Left => "LEFT",
            JoinType::Right => "RIGHT",
            JoinType::Full => "FULL",
        };
        std::write!(f, "{}", join_type_name)
    }
}

pub struct SelectBuilder {
    from_table: Option<&'static str>,
    joins: Vec<JoinClause>,
    where_conditions: Option<Vec<Kwargs>>,
}

struct JoinClause {
    join_type: JoinType,
    table: &'static str,
    on_conditions: Vec<Kwargs>,
}

/// A joined result type: a model or a tuple such as `(User, Product)`.
///
/// It knows the tables and fields to select and how to decode each model from
/// the resulting row. Models implement it through [`Model`].
#[cfg(not(feature = "turso"))]
pub trait JoinQuery: Sized {
    /// The selected `(table, fields)` pairs, in output/decoding order.
    fn tables() -> Vec<(&'static str, &'static [&'static str])>;

    /// Decodes the joined result from `row`.
    fn from_join_row(row: &sqlx::any::AnyRow, offset: &mut usize) -> Result<Self, Error>;
}

/// A joined result type: a model or a tuple such as `(User, Product)`.
///
/// It knows the tables and fields to select and how to decode each model from
/// the resulting row. Models implement it through [`Model`].
#[cfg(feature = "turso")]
pub trait JoinQuery: Sized {
    /// The selected `(table, fields)` pairs, in output/decoding order.
    fn tables() -> Vec<(&'static str, &'static [&'static str])>;

    /// Decodes the joined result from `row`.
    fn from_join_row(row: &libsql::Row, offset: &mut usize) -> Result<Self, Error>;
}

#[cfg(not(feature = "turso"))]
impl<T: Model> JoinQuery for T {
    fn tables() -> Vec<(&'static str, &'static [&'static str])> {
        vec![(<T as Model>::NAME, <T as Model>::FIELDS)]
    }

    fn from_join_row(row: &sqlx::any::AnyRow, offset: &mut usize) -> Result<Self, Error> {
        <T as Model>::from_join_row(row, offset)
    }
}

#[cfg(feature = "turso")]
impl<T: Model> JoinQuery for T {
    fn tables() -> Vec<(&'static str, &'static [&'static str])> {
        vec![(<T as Model>::NAME, <T as Model>::FIELDS)]
    }

    fn from_join_row(row: &libsql::Row, offset: &mut usize) -> Result<Self, Error> {
        <T as Model>::from_join_row(row, offset)
    }
}

#[cfg(not(feature = "turso"))]
impl<T: Model> JoinQuery for Option<T> {
    fn tables() -> Vec<(&'static str, &'static [&'static str])> {
        vec![(<T as Model>::NAME, <T as Model>::FIELDS)]
    }

    fn from_join_row(row: &sqlx::any::AnyRow, offset: &mut usize) -> Result<Self, Error> {
        if <T as Model>::is_null_join_row(row, *offset)? {
            *offset += <T as Model>::FIELDS.len();
            Ok(None)
        } else {
            Ok(Some(<T as Model>::from_join_row(row, offset)?))
        }
    }
}

#[cfg(feature = "turso")]
impl<T: Model> JoinQuery for Option<T> {
    fn tables() -> Vec<(&'static str, &'static [&'static str])> {
        vec![(<T as Model>::NAME, <T as Model>::FIELDS)]
    }

    fn from_join_row(row: &libsql::Row, offset: &mut usize) -> Result<Self, Error> {
        if <T as Model>::is_null_join_row(row, *offset)? {
            *offset += <T as Model>::FIELDS.len();
            Ok(None)
        } else {
            Ok(Some(<T as Model>::from_join_row(row, offset)?))
        }
    }
}

macro_rules! impl_join_query_for_tuple {
    ($($name:ident),+) => {
        #[cfg(not(feature = "turso"))]
        impl<$($name: JoinQuery),+> JoinQuery for ($($name,)+) {
            fn tables() -> Vec<(&'static str, &'static [&'static str])> {
                let mut tables = Vec::new();
                $(tables.extend(<$name as JoinQuery>::tables());)+
                tables
            }

            fn from_join_row(
                row: &sqlx::any::AnyRow,
                offset: &mut usize,
            ) -> Result<Self, Error> {
                Ok(($(<$name as JoinQuery>::from_join_row(row, offset)?,)+))
            }
        }

        #[cfg(feature = "turso")]
        impl<$($name: JoinQuery),+> JoinQuery for ($($name,)+) {
            fn tables() -> Vec<(&'static str, &'static [&'static str])> {
                let mut tables = Vec::new();
                $(tables.extend(<$name as JoinQuery>::tables());)+
                tables
            }

            fn from_join_row(
                row: &libsql::Row,
                offset: &mut usize,
            ) -> Result<Self, Error> {
                Ok(($(<$name as JoinQuery>::from_join_row(row, offset)?,)+))
            }
        }
    };
}

impl_join_query_for_tuple!(A, B);
impl_join_query_for_tuple!(A, B, C);
impl_join_query_for_tuple!(A, B, C, D);

impl SelectBuilder {
    pub fn new(from_table: Option<&'static str>) -> Self {
        Self {
            from_table,
            joins: Vec::new(),
            where_conditions: None,
        }
    }

    pub fn inner_join<Base: Model, Join: Model>(mut self, on: Vec<Kwargs>) -> Self {
        if self.from_table.is_none() {
            self.from_table = Some(Base::NAME);
        }

        self.joins.push(JoinClause {
            join_type: JoinType::Inner,
            table: Join::NAME,
            on_conditions: on,
        });

        self
    }

    pub fn left_join<Base: Model, Join: Model>(mut self, on: Vec<Kwargs>) -> Self {
        if self.from_table.is_none() {
            self.from_table = Some(Base::NAME);
        }

        self.joins.push(JoinClause {
            join_type: JoinType::Left,
            table: Join::NAME,
            on_conditions: on,
        });

        self
    }

    pub fn right_join<Base: Model, Join: Model>(mut self, on: Vec<Kwargs>) -> Self {
        if self.from_table.is_none() {
            self.from_table = Some(Base::NAME);
        }

        self.joins.push(JoinClause {
            join_type: JoinType::Right,
            table: Join::NAME,
            on_conditions: on,
        });

        self
    }

    pub fn r#where(mut self, conditions: Vec<Kwargs>) -> Self {
        self.where_conditions = Some(conditions);
        self
    }

    fn build_join_query<Output: JoinQuery>(&self) -> (String, Vec<Arg>) {
        let tables = Output::tables();
        let select_clause = tables
            .iter()
            .flat_map(|(table, fields)| {
                fields
                    .iter()
                    .map(move |field| format!("{table}.{field} AS {table}__{field}"))
            })
            .collect::<Vec<_>>()
            .join(", ");

        let mut query = format!("SELECT {select_clause}");
        let mut args = Vec::new();

        let from_table = self
            .from_table
            .or_else(|| tables.first().map(|(table, _)| *table));

        if let Some(from_table) = from_table {
            query.push_str(&format!(" FROM {from_table}"));
        }

        for join in &self.joins {
            let select_query = builder::to_select_query(join.on_conditions.clone());
            query.push_str(&format!(
                " {} JOIN {} ON {}",
                join.join_type, join.table, select_query.placeholders
            ));
            args.extend(select_query.args);
        }

        if let Some(conditions) = &self.where_conditions {
            let select_query = builder::to_select_query(conditions.clone());
            query.push_str(&format!(" WHERE {}", select_query.placeholders));
            args.extend(select_query.args);
        }

        query.push(';');

        (query, args)
    }

    #[cfg(not(feature = "turso"))]
    /// Fetches every row as a model or typed tuple, e.g. `Vec<(User, Product)>`.
    ///
    /// The output type determines which joined models are selected. Columns are
    /// selected with table-prefixed aliases and decoded by position, so joined
    /// tables may have columns with the same name.
    pub async fn fetch_all<Output>(self, conn: &Connection) -> Result<Vec<Output>, Error>
    where
        Output: JoinQuery,
    {
        let (query, args) = self.build_join_query::<Output>();

        let mut stream = sqlx::query(sqlx::AssertSqlSafe(query));
        binds!(args, stream);

        let rows = stream.fetch_all(conn).await?;
        rows.iter()
            .map(|row| {
                let mut offset = 0;
                Output::from_join_row(row, &mut offset)
            })
            .collect()
    }

    #[cfg(feature = "turso")]
    /// Fetches every row as a model or typed tuple, e.g. `Vec<(User, Product)>`.
    ///
    /// The output type determines which joined models are selected. Columns are
    /// selected with table-prefixed aliases and decoded by position, so joined
    /// tables may have columns with the same name.
    pub async fn fetch_all<Output>(self, conn: &Connection) -> Result<Vec<Output>, Error>
    where
        Output: JoinQuery,
    {
        let (query, args) = self.build_join_query::<Output>();
        let params = binds!(args.iter());

        let mut rows = conn.query(&query, params).await?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let mut offset = 0;
            results.push(Output::from_join_row(&row, &mut offset)?);
        }
        Ok(results)
    }

    #[cfg(not(feature = "turso"))]
    /// Fetches the first matching row as a model or typed tuple.
    ///
    /// Returns `None` when the query matches no rows. The output type
    /// determines which joined models are selected.
    pub async fn fetch_one<Output>(self, conn: &Connection) -> Result<Option<Output>, Error>
    where
        Output: JoinQuery,
    {
        let (query, args) = self.build_join_query::<Output>();

        let mut stream = sqlx::query(sqlx::AssertSqlSafe(query));
        binds!(args, stream);

        match stream.fetch_optional(conn).await? {
            Some(row) => {
                let mut offset = 0;
                Ok(Some(Output::from_join_row(&row, &mut offset)?))
            }
            None => Ok(None),
        }
    }

    #[cfg(feature = "turso")]
    /// Fetches the first matching row as a model or typed tuple.
    ///
    /// Returns `None` when the query matches no rows. The output type
    /// determines which joined models are selected.
    pub async fn fetch_one<Output>(self, conn: &Connection) -> Result<Option<Output>, Error>
    where
        Output: JoinQuery,
    {
        let (query, args) = self.build_join_query::<Output>();
        let params = binds!(args.iter());

        let mut rows = conn.query(&query, params).await?;
        match rows.next().await? {
            Some(row) => {
                let mut offset = 0;
                Ok(Some(Output::from_join_row(&row, &mut offset)?))
            }
            None => Ok(None),
        }
    }
}
