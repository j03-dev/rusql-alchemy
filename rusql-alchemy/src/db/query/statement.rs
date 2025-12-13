use super::{builder, condition::Kwargs, Query};
use crate::{
    db::{model, Connection},
    Error,
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

pub struct Statement(pub String);

impl Statement {
    #[cfg(not(feature = "turso"))]
    pub async fn join<
        T: Unpin + Send + Sync + for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> + model::Model,
    >(
        &self,
        join_type: JoinType,
        table: &str,
        kw: Vec<Kwargs>,
        conn: &Connection,
    ) -> Result<Vec<T>, Error> {
        let Query {
            placeholders, args, ..
        } = builder::to_select_query(kw);
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
    pub async fn join<T: Unpin + Send + Sync + for<'de> serde::Deserialize<'de> + model::Model>(
        &self,
        join_type: JoinType,
        table: &str,
        kw: Vec<Kwargs>,
        conn: &Connection,
    ) -> Result<Vec<T>, Error> {
        let Query {
            placeholders, args, ..
        } = builder::to_select_query(kw);
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
