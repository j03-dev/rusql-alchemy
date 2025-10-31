//! RusQL Alchemy: A Rust ORM library for SQL databases
//!
//! This module provides traits and implementations for database operations,
//! including querying, inserting, updating, and deleting records.

#[cfg(not(feature = "turso"))]
pub use sqlx::{any::AnyRow, FromRow, Row};

use serde::Serialize;

use super::{Arg, Kwargs, Query, PLACEHOLDER};
use crate::Error;
use crate::{get_type_name, Connection, FutRes};

/// Trait for database model operations.
#[async_trait::async_trait]
pub trait Model {
    // The SQL schema of the model
    const SCHEMA: &'static str;
    // The Table name of the model
    const NAME: &'static str;
    // The Primary Key of the model
    const PK: &'static str;

    /// Migrates the model schema to the database
    ///
    /// # Arguments
    /// * `conn` - The database connection
    ///
    /// # Returns
    /// `true` if the migration was successful, `false` otherwise
    ///
    /// # Example
    /// ```rust
    /// let success = User::migrate(&conn).await;
    /// println!("Migration success: {}", success);
    /// ```
    fn migrate(conn: &'_ Connection) -> FutRes<'_, (), Error>
    where
        Self: Sized,
    {
        Box::pin(async move {
            #[cfg(debug_assertions)]
            {
                let formatted_sql = sqlformat::format(
                    Self::SCHEMA,
                    &sqlformat::QueryParams::None,
                    &sqlformat::FormatOptions::default(),
                );
                println!("{formatted_sql}");
            }

            #[cfg(not(feature = "turso"))]
            sqlx::query(Self::SCHEMA).execute(conn).await?;

            #[cfg(feature = "turso")]
            conn.execute(Self::SCHEMA, ()).await?;

            Ok(())
        })
    }
    /// Saves the current model instance to the database.
    ///
    /// # Arguments
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `true` if save is successful, `false` otherwise.
    ///
    /// # Example
    /// ```
    /// let user = User {
    ///     name: "johnDoe@gmail.com".to_string(),
    ///     email: "21john@gmail.com".to_string(),
    ///     password: "p455w0rd".to_string(),
    ///     age: 18,
    ///     weight: 60.0,
    ///     ..Default::default()
    /// };
    /// let success = user.save(&conn).await;
    /// println!("Save success: {}", success);
    /// ```
    async fn save(&self, conn: &Connection) -> Result<(), Error>
    where
        Self: Sized;

    /// Creates a new model instance with the specified parameters.
    ///
    /// # Arguments
    /// * `kw` - The key-value arguments for the new instance.
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `true` if creation is successful, `false` otherwise.
    ///
    /// # Example
    /// ```
    /// let success = User::create(
    ///     kwargs!(
    ///         name = "joe",
    ///         email = "24nomeniavo@gmail.com",
    ///         password = "strongpassword",
    ///         age = 19,
    ///         weight = 80.1
    ///     ),
    ///     &conn,
    /// ).await;
    /// println!("Create success: {}", success);
    /// ```
    async fn create(kw: Vec<Kwargs>, conn: &Connection) -> Result<(), Error>
    where
        Self: Sized,
    {
        let Query {
            placeholders,
            fields,
            args,
        } = super::to_insert_query(kw);

        let query = format!(
            "insert into {table_name} ({fields}) values ({placeholders});",
            table_name = Self::NAME
        );

        #[cfg(not(feature = "turso"))]
        {
            let mut stream = sqlx::query(&query);
            binds!(args.iter(), stream);
            stream.execute(conn).await?;
        }

        #[cfg(feature = "turso")]
        {
            let params = binds!(args.iter());
            conn.execute(&query, params).await?;
        }
        Ok(())
    }

    /// Updates the current model instance in the database.
    ///
    /// # Arguments
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `true` if update is successful, `false` otherwise.
    ///
    /// # Example
    /// ```
    /// if let Some(mut user) = User::get(
    ///     kwargs!(email == "24nomeniavo@gmail.com").and(kwargs!(password == "strongpassword")),
    ///     &conn,
    /// ).await {
    ///     user.role = "admin".to_string();
    ///     let success = user.update(&conn).await;
    ///     println!("Update success: {}", success);
    /// }
    /// ```
    async fn update(&self, conn: &Connection) -> Result<(), Error>
    where
        Self: Sized;

    /// Updates a specific model instance identified by its primary key with the given parameters.
    ///
    /// # Arguments
    /// * `id_value` - The value of the primary key.
    /// * `kw` - The key-value arguments for the update.
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `true` if update is successful, `false` otherwise.
    ///
    /// # Example
    /// ```
    /// let success = User::set(
    ///     user_id,
    ///     kwargs!(role = "admin"),
    ///     &conn,
    /// ).await;
    /// println!("Set success: {}", success);
    /// ```
    async fn set<T: Serialize + Clone + Send + Sync>(
        id_value: T,
        kw: Vec<Kwargs>,
        conn: &Connection,
    ) -> Result<(), Error> {
        let Query {
            placeholders,
            mut args,
            ..
        } = super::to_update_query(kw);

        args = args
            .into_iter()
            .chain([Arg {
                value: serde_json::json!(id_value).to_string(),
                ty: get_type_name(id_value.clone()).to_string(),
            }])
            .collect();

        let index_id = args.len();
        let query = format!(
            "update {table_name} set {placeholders} where {id}={PLACEHOLDER}{index_id};",
            id = Self::PK,
            table_name = Self::NAME,
        );

        #[cfg(not(feature = "turso"))]
        {
            let mut stream = sqlx::query(&query);
            binds!(args, stream);
            stream.execute(conn).await?;
        }

        #[cfg(feature = "turso")]
        {
            let params = binds!(args.iter());
            conn.execute(&query, params).await?;
        }
        Ok(())
    }

    /// Deletes the current model instance from the database.
    ///
    /// # Arguments
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `true` if delete is successful, `false` otherwise.
    ///
    /// # Example
    /// ```
    /// let success = user.delete(&conn).await;
    /// println!("Delete success: {}", success);
    /// ```
    async fn delete(&self, conn: &Connection) -> Result<(), Error>
    where
        Self: Sized;

    /// Retrieves all instances of the model from the database.
    ///
    /// # Arguments
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// A vector of all instances of the model.
    ///
    /// # Example
    /// ```
    /// let users = User::all(&conn).await;
    /// println!("{:#?}", users);
    /// ```
    #[cfg(not(feature = "turso"))]
    async fn all(conn: &Connection) -> Result<Vec<Self>, Error>
    where
        Self: Sized + Unpin + for<'r> FromRow<'r, AnyRow> + Clone,
    {
        let query = format!("select * from {table_name}", table_name = Self::NAME);
        Ok(sqlx::query_as::<_, Self>(&query).fetch_all(conn).await?)
    }
    #[cfg(feature = "turso")]
    async fn all(conn: &Connection) -> Result<Vec<Self>, Error>
    where
        Self: Sized + for<'de> serde::Deserialize<'de>,
    {
        let query = format!("select * from {table_name}", table_name = Self::NAME);
        let mut rows = conn.query(&query, ()).await?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let s = libsql::de::from_row::<Self>(&row)?;
            results.push(s);
        }
        Ok(results)
    }

    /// Filters instances of the model based on the provided parameters.
    ///
    /// # Arguments
    /// * `kw` - The key-value arguments for filtering.
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// A vector of instances matching the filter criteria.
    ///
    /// # Example
    /// ```
    /// let users = User::filter(
    ///     kwargs!(age <= 18).and(kwargs!(weight == 80.0)),
    ///     &conn,
    /// ).await;
    /// println!("{:#?}", users);
    /// ```
    #[cfg(not(feature = "turso"))]
    async fn filter(kw: Vec<Kwargs>, conn: &Connection) -> Result<Vec<Self>, Error>
    where
        Self: Sized + Unpin + for<'r> FromRow<'r, AnyRow> + Clone,
    {
        let Query {
            placeholders, args, ..
        } = super::to_select_query(kw);

        let query = format!(
            "SELECT * FROM {table_name} WHERE {placeholders};",
            table_name = Self::NAME
        );

        let mut stream = sqlx::query_as::<_, Self>(&query);
        binds!(args, stream);
        Ok(stream.fetch_all(conn).await?)
    }
    #[cfg(feature = "turso")]
    async fn filter(kw: Vec<Kwargs>, conn: &Connection) -> Result<Vec<Self>, Error>
    where
        Self: Sized + for<'de> serde::Deserialize<'de>,
    {
        let Query {
            placeholders, args, ..
        } = super::to_select_query(kw);

        let query = format!(
            "SELECT * FROM {table_name} WHERE {placeholders};",
            table_name = Self::NAME
        );
        let params = binds!(args.iter());
        let mut rows = conn.query(&query, params).await?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let s = libsql::de::from_row::<Self>(&row)?;
            results.push(s);
        }
        Ok(results)
    }

    /// Retrieves the first instance of the model matching the filter criteria.
    ///
    /// # Arguments
    /// * `kw` - The key-value arguments for filtering.
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// An optional instance matching the filter criteria.
    ///
    /// # Example
    /// ```
    /// let user = User::get(
    ///     kwargs!(email == "24nomeniavo@gmail.com").and(kwargs!(password == "strongpassword")),
    ///     &conn,
    /// ).await;
    /// println!("{:#?}", user);
    /// ```
    #[cfg(not(feature = "turso"))]
    async fn get(kw: Vec<Kwargs>, conn: &Connection) -> Result<Option<Self>, Error>
    where
        Self: Sized + Unpin + for<'r> FromRow<'r, AnyRow> + Clone,
    {
        Ok(Self::filter(kw, conn).await?.first().cloned())
    }

    #[cfg(feature = "turso")]
    async fn get(kw: Vec<Kwargs>, conn: &Connection) -> Result<Option<Self>, Error>
    where
        Self: Sized + Clone + for<'de> serde::Deserialize<'de>,
    {
        Ok(Self::filter(kw, conn).await?.first().cloned())
    }

    /// Counts the number of instances of the model in the database.
    ///
    /// # Arguments
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// The count of instances.
    ///
    /// # Example
    /// ```
    /// let count = User::count(&conn).await;
    /// println!("User count: {}", count);
    /// ```
    async fn count(conn: &Connection) -> Result<i64, Error>
    where
        Self: Sized,
    {
        let query = format!("select count(*) from {table_name}", table_name = Self::NAME);
        #[cfg(not(feature = "turso"))]
        {
            Ok(sqlx::query(&query).fetch_one(conn).await?.get(0))
        }

        #[cfg(feature = "turso")]
        {
            let row = conn
                .query(&query, ())
                .await?
                .next()
                .await?
                .ok_or("no rows returned")?;
            Ok(row.get(0)?)
        }
    }
}

/// Trait for deleting database records.
#[async_trait::async_trait]
pub trait Delete {
    async fn delete(&self, conn: &Connection) -> Result<(), Error>;
}
#[async_trait::async_trait]
impl<T> Delete for Vec<T>
where
    T: Model + Sync,
{
    /// Deletes all instances of the model from the database.
    ///
    /// This method will delete all records from the table corresponding to the model `T`.
    /// Be cautious when using this method, as it will remove all entries without conditions.
    ///
    /// # Arguments
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `true` if deletion is successful, `false` otherwise.
    ///
    /// # Example
    /// ```
    /// # use rusql_alchemy::prelude::*;
    /// # use sqlx::FromRow;
    /// #
    /// # #[derive(FromRow, Debug, Default, Model, Clone)]
    /// # struct Product {
    /// #     #[field(primary_key = true, auto = true)]
    /// #     id: Integer,
    /// #     #[field(size = 50)]
    /// #     name: String,
    /// #     price: Float,
    /// #     description: Text,
    /// #     #[field(default = true)]
    /// #     is_sel: Boolean,
    /// #     #[field(foreign_key = "User.id")]
    /// #     owner: Integer,
    /// #     #[field(default = "now")]
    /// #     at: DateTime,
    /// # }
    /// #
    /// #[tokio::main]
    /// async fn main() -> Result<(), sqlx::error> {
    ///     let conn = Database::new().await?.conn;
    ///
    ///     let products = Product::all(&conn).await?;
    ///     let success = products.delete(&conn).await;
    ///     println!("Products delete success: {}", success);
    ///
    ///     let products = Product::all(&conn).await;
    ///     println!("Remaining products: {:#?}", products);
    /// }
    /// ```
    ///
    /// In the above example, all records from the `Product` table will be deleted.
    async fn delete(&self, conn: &Connection) -> Result<(), Error> {
        let query = format!("delete from {table_name}", table_name = T::NAME);
        #[cfg(not(feature = "turso"))]
        {
            sqlx::query(&query).execute(conn).await?;
        }

        #[cfg(feature = "turso")]
        {
            conn.execute(&query, ()).await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
pub trait JoinTable<A, B> {
    async fn inner_join(
        self,
        column_a: &str,
        column_b: &str,
        kw: Option<Vec<Condition>>,
        conn: &Connection,
    ) -> Result<Vec<(A, B)>, sqlx::Error>;

    async fn left_join(
        self,
        column_a: &str,
        column_b: &str,
        kw: Option<Vec<Condition>>,
        conn: &Connection,
    ) -> Result<Vec<(A, B)>, sqlx::Error>;
}

async fn join_table<A, B>(
    join: &str,
    column_a: &str,
    column_b: &str,
    kw: Option<Vec<Condition>>,
    conn: &Connection,
) -> Result<Vec<(A, B)>, sqlx::Error>
where
    A: Model + Sync + Send + Unpin + for<'r> FromRow<'r, AnyRow>,
    B: Model + Sync + Send + Unpin + for<'r> FromRow<'r, AnyRow>,
{
    let mut query = format!(
        "SELECT {table_a}.*, {table_b}.* \
             FROM {table_a} \
             {join} JOIN {table_b} \
             ON {table_a}.{column_a} = {table_b}.{column_b}",
        table_a = A::NAME,
        table_b = B::NAME
    );

    let mut arguments = None;

    if let Some(kwargs) = kw {
        let UpSel { placeholders, args } = kwargs.to_select_query();
        query.push_str(&format!(" WHERE {placeholders}"));
        arguments = Some(args);
    }

    let mut rows = sqlx::query(&query);
    if arguments.is_some() {
        binds!(arguments.unwrap(), rows);
    }
    let rows = rows.fetch_all(conn).await?;

    let mut result = Vec::new();
    for row in rows {
        let a = A::from_row(&row)?;
        let b = B::from_row(&row)?;
        result.push((a, b));
    }

    Ok(result)
}

#[async_trait::async_trait]
impl<A, B> JoinTable<A, B> for (A, B)
where
    A: Model + Sync + Send + Unpin + for<'r> FromRow<'r, AnyRow>,
    B: Model + Sync + Send + Unpin + for<'r> FromRow<'r, AnyRow>,
{
    async fn inner_join(
        self,
        column_a: &str,
        column_b: &str,
        kw: Option<Vec<Condition>>,
        conn: &Connection,
    ) -> Result<Vec<(A, B)>, sqlx::Error> {
        join_table("INNER", column_a, column_b, kw, conn).await
    }

    async fn left_join(
        self,
        column_a: &str,
        column_b: &str,
        kw: Option<Vec<Condition>>,
        conn: &Connection,
    ) -> Result<Vec<(A, B)>, sqlx::Error> {
        join_table("LEFT", column_a, column_b, kw, conn).await
    }
}
