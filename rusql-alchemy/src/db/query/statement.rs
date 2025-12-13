use super::{builder, condition::Kwargs, Query};
use crate::{
    db::{model::Model, Connection},
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

pub struct SelectBuilder {
    select_clause: String,
    from_table: Option<String>,
    joins: Vec<JoinClause>,
    where_conditions: Option<Vec<Kwargs>>,
}

struct JoinClause {
    join_type: JoinType,
    table: String,
    on_conditions: Vec<Kwargs>,
}

impl SelectBuilder {
    pub fn new(select_clause: String, from_table: Option<String>) -> Self {
        Self {
            select_clause,
            from_table,
            joins: Vec::new(),
            where_conditions: None,
        }
    }

    pub fn inner_join<Base: Model, Join: Model>(mut self, on: Vec<Kwargs>) -> Self {
        if self.from_table.is_none() {
            self.from_table = Some(Base::NAME.to_string());
        }

        self.joins.push(JoinClause {
            join_type: JoinType::Inner,
            table: Join::NAME.to_string(),
            on_conditions: on,
        });

        self
    }

    pub fn left_join<Base: Model, Join: Model>(mut self, on: Vec<Kwargs>) -> Self {
        if self.from_table.is_none() {
            self.from_table = Some(Base::NAME.to_string());
        }

        self.joins.push(JoinClause {
            join_type: JoinType::Left,
            table: Join::NAME.to_string(),
            on_conditions: on,
        });

        self
    }

    pub fn right_join<Base: Model, Join: Model>(mut self, on: Vec<Kwargs>) -> Self {
        if self.from_table.is_none() {
            self.from_table = Some(Base::NAME.to_string());
        }

        self.joins.push(JoinClause {
            join_type: JoinType::Right,
            table: Join::NAME.to_string(),
            on_conditions: on,
        });

        self
    }

    pub fn r#where(mut self, conditions: Vec<Kwargs>) -> Self {
        self.where_conditions = Some(conditions);
        self
    }

    fn build_query(&self) -> (String, Vec<super::Arg>) {
        let mut query = format!("SELECT {}", self.select_clause);
        let mut all_args = Vec::new();

        if let Some(from_table) = &self.from_table {
            query.push_str(&format!(" FROM {}", from_table));
        }

        for join in &self.joins {
            let Query {
                placeholders, args, ..
            } = builder::to_select_query(join.on_conditions.clone());

            query.push_str(&format!(
                " {} JOIN {} ON {}",
                join.join_type, join.table, placeholders
            ));

            all_args.extend(args);
        }

        if let Some(conditions) = &self.where_conditions {
            let Query {
                placeholders, args, ..
            } = builder::to_select_query(conditions.clone());

            query.push_str(&format!(" WHERE {}", placeholders));
            all_args.extend(args);
        }

        query.push(';');

        println!("{}", query);

        (query, all_args)
    }

    #[cfg(not(feature = "turso"))]
    pub async fn fetch_one<Output>(self, conn: &Connection) -> Result<Output, Error>
    where
        Output: Unpin + Send + for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow>,
    {
        let (query, args) = self.build_query();

        let mut stream = sqlx::query_as::<_, Output>(&query);
        binds!(args, stream);

        Ok(stream.fetch_one(conn).await?)
    }

    #[cfg(feature = "turso")]
    pub async fn fetch_one<Output>(self, conn: &Connection) -> Result<Output, Error>
    where
        Output: for<'de> serde::Deserialize<'de>,
    {
        let (query, args) = self.build_query();
        let params = binds!(args.iter());

        let row = conn
            .query(&query, params)
            .await?
            .next()
            .await?
            .ok_or("No rows returned")?;

        Ok(libsql::de::from_row::<Output>(&row)?)
    }

    #[cfg(not(feature = "turso"))]
    pub async fn fetch_all<Output>(self, conn: &Connection) -> Result<Vec<Output>, Error>
    where
        Output: Unpin + Send + for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow>,
    {
        let (query, args) = self.build_query();

        let mut stream = sqlx::query_as::<_, Output>(&query);
        binds!(args, stream);

        Ok(stream.fetch_all(conn).await?)
    }

    #[cfg(feature = "turso")]
    pub async fn fetch_all<Output>(self, conn: &Connection) -> Result<Vec<Output>, Error>
    where
        Output: for<'de> serde::Deserialize<'de>,
    {
        let (query, args) = self.build_query();
        let params = binds!(args.iter());

        let mut rows = conn.query(&query, params).await?;
        let mut results = Vec::new();

        while let Some(row) = rows.next().await? {
            let s = libsql::de::from_row::<Output>(&row)?;
            results.push(s);
        }

        Ok(results)
    }

    #[cfg(not(feature = "turso"))]
    pub async fn fetch_optional<Output>(self, conn: &Connection) -> Result<Option<Output>, Error>
    where
        Output: Unpin + Send + for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow>,
    {
        let (query, args) = self.build_query();

        let mut stream = sqlx::query_as::<_, Output>(&query);
        binds!(args, stream);

        Ok(stream.fetch_optional(conn).await?)
    }

    #[cfg(feature = "turso")]
    pub async fn fetch_optional<Output>(self, conn: &Connection) -> Result<Option<Output>, Error>
    where
        Output: for<'de> serde::Deserialize<'de>,
    {
        let (query, args) = self.build_query();
        let params = binds!(args.iter());

        let mut rows = conn.query(&query, params).await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(libsql::de::from_row::<Output>(&row)?))
        } else {
            Ok(None)
        }
    }
}
