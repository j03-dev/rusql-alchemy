//! The `db` module provides functionality for interacting with the database.
//!
//! This module contains submodules and traits that define the structure and behavior
//! of database models, as well as functions for performing common database operations.
use crate::Connection;

/// The `models` module defines the traits and structures for database models.
///
/// This module includes the `Model` trait, which provides a common interface for
/// database models, and various implementations of this trait for different
/// entities in the application.
pub mod models;

#[cfg(not(feature = "postgres"))]
pub const PLACEHOLDER: &str = "?";

#[cfg(feature = "postgres")]
pub const PLACEHOLDER: &str = "$";

use crate::Error;

#[derive(Debug)]
pub enum Kwargs {
    Condition {
        field: String,
        value: String,
        value_type: String,
        comparison_operator: String,
    },
    LogicalOperator {
        operator: String,
    },
}

pub trait Or {
    fn or(self, kwargs: Vec<Kwargs>) -> Vec<Kwargs>;
}

pub trait And {
    fn and(self, kwargs: Vec<Kwargs>) -> Vec<Kwargs>;
}

impl Or for Vec<Kwargs> {
    fn or(mut self, kwargs: Vec<Kwargs>) -> Vec<Kwargs> {
        self.push(Kwargs::LogicalOperator {
            operator: "or".to_string(),
        });
        self.extend(kwargs);
        self
    }
}

impl And for Vec<Kwargs> {
    fn and(mut self, kwargs: Vec<Kwargs>) -> Vec<Kwargs> {
        self.push(Kwargs::LogicalOperator {
            operator: "and".to_string(),
        });
        self.extend(kwargs);
        self
    }
}

struct Arg {
    value: String,
    ty: String,
}

#[derive(Default)]
struct Query {
    placeholders: String,
    fields: String,
    args: Vec<Arg>,
}

fn to_update_query(kw: Vec<Kwargs>) -> Query {
    let mut args = Vec::new();
    let mut placeholders = Vec::new();
    let mut index = 0;
    for condition in kw {
        if let Kwargs::Condition {
            field,
            value,
            value_type,
            ..
        } = condition
        {
            index += 1;
            args.push(Arg {
                value: value.to_owned(),
                ty: value_type.clone(),
            });
            placeholders.push(format!("{field}={PLACEHOLDER}{index}",));
        }
    }

    Query {
        placeholders: placeholders.join(", "),
        args,
        ..Default::default()
    }
}

fn to_select_query(kw: Vec<Kwargs>) -> Query {
    let mut args = Vec::new();
    let mut placeholders = Vec::new();
    let mut index = 0;
    for condition in kw {
        match condition {
            Kwargs::Condition {
                field,
                value,
                value_type,
                comparison_operator,
            } => {
                if value_type == "column" {
                    placeholders.push(format!("{field}{comparison_operator}{value}"));
                } else {
                    index += 1;
                    args.push(Arg {
                        value: value.to_owned(),
                        ty: value_type.clone(),
                    });
                    placeholders.push(format!("{field}{comparison_operator}{PLACEHOLDER}{index}",));
                }
            }
            Kwargs::LogicalOperator { operator } => {
                placeholders.push(operator.to_owned());
            }
        }
    }

    Query {
        placeholders: placeholders.join(" "),
        args,
        ..Default::default()
    }
}

fn to_insert_query(kw: Vec<Kwargs>) -> Query {
    let mut args = Vec::new();
    let mut fields = Vec::new();
    let mut placeholders = Vec::new();
    let mut index = 0;
    for condition in kw {
        if let Kwargs::Condition {
            field,
            value,
            value_type,
            ..
        } = condition
        {
            index += 1;
            args.push(Arg {
                value: value.to_owned(),
                ty: value_type.clone(),
            });
            fields.push(field.clone());
            placeholders.push(format!("{PLACEHOLDER}{index}"));
        }
    }

    Query {
        placeholders: placeholders.join(", "),
        fields: fields.join(", "),
        args,
    }
}

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

pub struct Statement(pub String);

impl Statement {
    #[cfg(not(feature = "turso"))]
    pub async fn join<
        T: Unpin + Send + Sync + for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> + models::Model,
    >(
        &self,
        join_type: JoinType,
        table: &str,
        kw: Vec<Kwargs>,
        conn: &Connection,
    ) -> Result<Vec<T>, Error> {
        let Query {
            placeholders, args, ..
        } = to_select_query(kw);
        let query = format!(
            "{select} FROM {base_table} {join_type} join {table} on {placeholders};",
            select = self.0,
            base_table = T::NAME,
            join_type = join_type,
        );

        let mut stream = sqlx::query_as::<_, T>(&query);
        binds!(args, stream);
        Ok(stream.fetch_all(conn).await?)
    }

    #[cfg(feature = "turso")]
    pub async fn join<T: Unpin + Send + Sync + for<'de> serde::Deserialize<'de> + models::Model>(
        &self,
        join_type: JoinType,
        table: &str,
        kw: Vec<Kwargs>,
        conn: &Connection,
    ) -> Result<Vec<T>, Error> {
        let Query {
            placeholders, args, ..
        } = to_select_query(kw);
        let query = format!(
            "{select} FROM {base_table} {join_type} join {table} on {placeholders};",
            select = self.0,
            base_table = T::NAME,
            join_type = join_type,
        );

        let params = binds!(args.iter());
        let mut rows = conn.query(&query, params).await?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let s = libsql::de::from_row::<T>(&row)?;
            results.push(s);
        }
        Ok(results)
    }
}
