//! RusQL Alchemy: A Rust ORM library for SQL databases
//!
//! This module provides traits and implementations for database operations,
//! including querying, inserting, updating, and deleting records.

use serde::Serialize;
#[cfg(not(feature = "turso"))]
use sqlx::{any::AnyRow, FromRow, Row};

use super::query::{builder, condition::Kwargs, Arg};
use super::{Connection, PLACEHOLDER};
#[allow(unused_imports)]
use crate::{utils, Error};

/// Trait for database model operations.
#[async_trait::async_trait]
pub trait Model {
    const UP: &'static str;
    const DOWN: &'static str;
    const NAME: &'static str;
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
    fn migrate(conn: &'_ Connection) -> crate::FutRes<'_, (), Error>
    where
        Self: Sized,
    {
        Box::pin(async move {
            #[cfg(debug_assertions)]
            {
                let formatted_sql = sqlformat::format(
                    Self::UP,
                    &sqlformat::QueryParams::None,
                    &sqlformat::FormatOptions::default(),
                );
                println!("{formatted_sql}");
            }

            #[cfg(not(feature = "turso"))]
            {
                sqlx::query(Self::DOWN).execute(conn).await?;
                sqlx::query(Self::UP).execute(conn).await?;
            }

            #[cfg(feature = "turso")]
            {
                conn.execute(Self::DOWN, ()).await?;
                conn.execute(Self::UP, ()).await?;
            }

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
        let insert_query = builder::to_insert_query(kw);

        let query = format!(
            "insert into {name} ({fields}) values ({placeholders});",
            name = Self::NAME,
            fields = insert_query.fields,
            placeholders = insert_query.placeholders,
        );

        #[cfg(not(feature = "turso"))]
        {
            let mut stream = sqlx::query(&query);
            binds!(insert_query.args.iter(), stream);
            stream.execute(conn).await?;
        }

        #[cfg(feature = "turso")]
        {
            let params = binds!(insert_query.args.iter());
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
        let mut update_query = builder::to_update_query(kw);

        update_query.args = update_query
            .args
            .into_iter()
            .chain([Arg {
                value: serde_json::json!(id_value).to_string(),
                ty: crate::utils::get_type_name(id_value.clone()).to_string(),
            }])
            .collect();

        let index_id = update_query.args.len();
        let query = format!(
            "update {name} set {placeholders} where {id}={PLACEHOLDER}{index_id};",
            id = Self::PK,
            name = Self::NAME,
            placeholders = update_query.placeholders,
        );

        #[cfg(not(feature = "turso"))]
        {
            let mut stream = sqlx::query(&query);
            binds!(update_query.args, stream);
            stream.execute(conn).await?;
        }

        #[cfg(feature = "turso")]
        {
            let params = binds!(update_query.args.iter());
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
        let query = format!("select * from {name}", name = Self::NAME);
        Ok(sqlx::query_as::<_, Self>(&query).fetch_all(conn).await?)
    }
    #[cfg(feature = "turso")]
    async fn all(conn: &Connection) -> Result<Vec<Self>, Error>
    where
        Self: Sized + for<'de> serde::Deserialize<'de>,
    {
        let query = format!("select * from {name}", name = Self::NAME);
        let rows = conn.query(&query, ()).await?;
        let results = utils::libsql_from_row(rows).await?;
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
        let select_query = builder::to_select_query(kw);

        let query = format!(
            "SELECT * FROM {name} WHERE {placeholders};",
            name = Self::NAME,
            placeholders = select_query.placeholders,
        );

        let mut stream = sqlx::query_as::<_, Self>(&query);
        binds!(select_query.args, stream);
        Ok(stream.fetch_all(conn).await?)
    }
    #[cfg(feature = "turso")]
    async fn filter(kw: Vec<Kwargs>, conn: &Connection) -> Result<Vec<Self>, Error>
    where
        Self: Sized + for<'de> serde::Deserialize<'de>,
    {
        let select_query = builder::to_select_query(kw);

        let query = format!(
            "SELECT * FROM {name} WHERE {placeholders};",
            name = Self::NAME,
            placeholders = select_query.placeholders,
        );
        let params = binds!(select_query.args.iter());
        let rows = conn.query(&query, params).await?;
        let results = utils::libsql_from_row(rows).await?;
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
        let query = format!("select count(*) from {name}", name = Self::NAME);
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
    /// async fn main() -> Result<(), rusql_alchemy::error> {
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
        let query = format!("delete from {name}", name = T::NAME);
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
