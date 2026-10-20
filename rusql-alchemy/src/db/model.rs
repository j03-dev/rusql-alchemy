//! Models: the `Model` trait and the collection helpers `Delete` and `Save`.
//!
//! This module provides traits and implementations for database operations,
//! including querying, inserting, updating, and deleting records.

use super::query::{Arg, builder, condition::Kwargs};
use super::{Connection, PLACEHOLDER};
use crate::{Error, utils};
use serde::Serialize;

/// `Model` is a foundational trait in `rusql-alchemy` that provides an interface
/// for Rust structs to interact with a database table. By deriving or implementing
/// this trait, your struct gains capabilities for schema registration (used by
/// [`Database::up`](crate::Database::up) and `rusql-alchemy-migration`) and CRUD
/// operations (Create, Read, Update, Delete).
///
/// This trait is typically implemented automatically using the `#[derive(Model)]`
/// procedural macro from `rusql_alchemy_derive`.
///
/// # Constants
///
/// * `SCHEMA` — the `create table if not exists ...` statement for this model.
/// * `NAME` — the table name.
/// * `PK` — the primary key column name.
///
/// Deriving the macro also submits `SCHEMA` to `inventory`, registers a
/// `Default` impl, and implements `save`, `update` and `delete`.
///
/// # Examples
///
/// Below is an example of a `User` struct deriving the `Model` trait.
///
/// ```ignore
/// use rusql_alchemy::prelude::*;
/// use sqlx::FromRow;
///
/// #[derive(Model, Clone, FromRow, Debug)]
/// struct User {
///     #[field(primary_key = true, auto = true)]
///     id: Option<Integer>,
///     name: String,
///     #[field(default = "user")]
///     role: String,
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), rusql_alchemy::Error> {
///     let database = Database::new("sqlite::memory:").await?;
///
///     // Creates the `User` table, and every other registered model's table
///     database.up().await?;
///
///     Ok(())
/// }
/// ```
#[async_trait::async_trait]
pub trait Model {
    const SCHEMA: &'static str;
    const NAME: &'static str;
    const PK: &'static str;

    #[cfg(not(feature = "turso"))]
    /// Saves the current model instance to the database.
    ///
    /// # Arguments
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `Ok(self)` with the persisted instance as returned by the database
    /// (including generated fields such as an auto-incremented primary key),
    /// or an `Error` if the insert failed.
    ///
    /// # Example
    /// ```ignore
    /// let user = User {
    ///     name: "johnDoe@gmail.com".to_string(),
    ///     email: "21john@gmail.com".to_string(),
    ///     password: "p455w0rd".to_string(),
    ///     age: 18,
    ///     weight: 60.0,
    ///     ..Default::default()
    /// };
    /// // Returns the persisted instance, with generated fields filled in
    /// let saved = user.save(&conn).await?;
    /// println!("Saved with id: {:?}", saved.id);
    /// ```
    async fn save<'e, E>(&self, conn: E) -> Result<Self, Error>
    where
        Self: Sized,
        E: sqlx::Executor<'e, Database = sqlx::Any> + Send;

    #[cfg(feature = "turso")]
    /// Saves the current model instance to the database.
    ///
    /// # Arguments
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `Ok(self)` with the persisted instance as returned by the database
    /// (including generated fields such as an auto-incremented primary key),
    /// or an `Error` if the insert failed.
    ///
    /// # Example
    /// ```ignore
    /// let user = User {
    ///     name: "johnDoe@gmail.com".to_string(),
    ///     email: "21john@gmail.com".to_string(),
    ///     password: "p455w0rd".to_string(),
    ///     age: 18,
    ///     weight: 60.0,
    ///     ..Default::default()
    /// };
    /// // Returns the persisted instance, with generated fields filled in
    /// let saved = user.save(&conn).await?;
    /// println!("Saved with id: {:?}", saved.id);
    /// ```
    async fn save(&self, conn: &Connection) -> Result<Self, Error>
    where
        Self: Sized;

    /// Creates a new model instance with the specified parameters.
    ///
    /// # Arguments
    /// * `kw` - The key-value arguments for the new instance.
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `Ok(model)` with the created instance as returned by the database
    /// (including defaults and generated fields), or an `Error` if the
    /// insert failed.
    ///
    /// # Example
    /// ```ignore
    /// let user = User::create(
    ///     kwargs!(
    ///         name = "joe",
    ///         email = "24nomeniavo@gmail.com",
    ///         password = "strongpassword",
    ///         age = 19,
    ///         weight = 80.1
    ///     ),
    ///     &conn,
    /// ).await?;
    /// println!("Created user: {:?}", user);
    /// ```
    #[cfg(not(feature = "turso"))]
    async fn create<'e, E>(kw: Vec<Kwargs>, conn: E) -> Result<Self, Error>
    where
        Self: Sized + Unpin + for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> + Clone,
        E: sqlx::Executor<'e, Database = sqlx::Any> + Send,
    {
        let insert_query = builder::to_insert_query(kw);

        let query = format!(
            "insert into {name} ({fields}) values ({placeholders}) returning *;",
            name = Self::NAME,
            fields = insert_query.fields,
            placeholders = insert_query.placeholders,
        );
        let mut stream = sqlx::query_as::<_, Self>(sqlx::AssertSqlSafe(query));
        binds!(insert_query.args.iter(), stream);
        Ok(stream.fetch_one(conn).await?)
    }

    /// Creates a new model instance with the specified parameters.
    ///
    /// # Arguments
    /// * `kw` - The key-value arguments for the new instance.
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `Ok(model)` with the created instance as returned by the database
    /// (including defaults and generated fields), or an `Error` if the
    /// insert failed.
    ///
    /// # Example
    /// ```ignore
    /// let user = User::create(
    ///     kwargs!(
    ///         name = "joe",
    ///         email = "24nomeniavo@gmail.com",
    ///         password = "strongpassword",
    ///         age = 19,
    ///         weight = 80.1
    ///     ),
    ///     &conn,
    /// ).await?;
    /// println!("Created user: {:?}", user);
    /// ```
    #[cfg(feature = "turso")]
    async fn create(kw: Vec<Kwargs>, conn: &Connection) -> Result<Self, Error>
    where
        Self: Sized + for<'de> serde::Deserialize<'de>,
    {
        let insert_query = builder::to_insert_query(kw);

        let query = format!(
            "insert into {name} ({fields}) values ({placeholders}) returning *;",
            name = Self::NAME,
            fields = insert_query.fields,
            placeholders = insert_query.placeholders,
        );

        let params = binds!(insert_query.args.iter());
        let mut rows = conn.query(&query, params).await?;
        let row = rows.next().await?.unwrap();
        Ok(libsql::de::from_row(&row)?)
    }

    /// Updates the current model instance in the database.
    ///
    /// # Arguments
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `Ok(())` on success, or an `Error` if the update failed.
    ///
    /// # Example
    /// ```ignore
    /// if let Some(mut user) = User::get(
    ///     kwargs!(email == "24nomeniavo@gmail.com"),
    ///     &conn,
    /// ).await? {
    ///     user.role = "admin".to_string();
    ///     user.update(&conn).await?;
    /// }
    /// ```
    async fn update(&self, conn: &Connection) -> Result<(), Error>
    where
        Self: Sized;

    /// Updates a specific model instance identified by its primary key with the given parameters.
    ///
    /// # Arguments
    /// * `id` - The value of the primary key.
    /// * `kw` - The key-value arguments for the update.
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `Ok(())` on success, or an `Error` if the update failed.
    ///
    /// # Example
    /// ```ignore
    /// User::set(
    ///     user_id,
    ///     kwargs!(role = "admin"),
    ///     &conn,
    /// ).await?;
    /// ```
    async fn set<T: Serialize + Clone + Send + Sync>(
        id: T,
        kw: Vec<Kwargs>,
        conn: &Connection,
    ) -> Result<(), Error> {
        let mut update_query = builder::to_update_query(kw);

        update_query.args = update_query
            .args
            .into_iter()
            .chain([Arg {
                value: serde_json::json!(id.clone()).to_string(),
                ty: utils::get_type_name(id).to_string(),
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
            let mut stream = sqlx::query(sqlx::AssertSqlSafe(query));
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

    #[cfg(not(feature = "turso"))]
    /// Deletes the current model instance from the database.
    ///
    /// # Arguments
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `Ok(())` on success, or an `Error` if the delete failed.
    ///
    /// # Example
    /// ```ignore
    /// user.delete(&conn).await?;
    /// ```
    async fn delete<'e, E>(&self, conn: E) -> Result<(), Error>
    where
        Self: Sized,
        E: sqlx::Executor<'e, Database = sqlx::Any> + Send;

    #[cfg(feature = "turso")]
    /// Deletes the current model instance from the database.
    ///
    /// # Arguments
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// `Ok(())` on success, or an `Error` if the delete failed.
    ///
    /// # Example
    /// ```ignore
    /// user.delete(&conn).await?;
    /// ```
    async fn delete(&self, conn: &Connection) -> Result<(), Error>
    where
        Self: Sized;

    #[cfg(not(feature = "turso"))]
    /// Retrieves all instances of the model from the database.
    ///
    /// # Arguments
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// A vector of all instances of the model.
    ///
    /// # Example
    /// ```ignore
    /// let users = User::all(&conn).await?;
    /// println!("{:#?}", users);
    /// ```
    async fn all(conn: &Connection) -> Result<Vec<Self>, Error>
    where
        Self: Sized + Unpin + for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> + Clone,
    {
        let query = format!("select * from {name}", name = Self::NAME);
        Ok(sqlx::query_as::<_, Self>(sqlx::AssertSqlSafe(query))
            .fetch_all(conn)
            .await?)
    }

    #[cfg(feature = "turso")]
    /// Retrieves all instances of the model from the database.
    ///
    /// # Arguments
    /// * `conn` - The database connection.
    ///
    /// # Returns
    /// A vector of all instances of the model.
    ///
    /// # Example
    /// ```ignore
    /// let users = User::all(&conn).await?;
    /// println!("{:#?}", users);
    /// ```
    async fn all(conn: &Connection) -> Result<Vec<Self>, Error>
    where
        Self: Sized + for<'de> serde::Deserialize<'de>,
    {
        let query = format!("select * from {name}", name = Self::NAME);
        let rows = conn.query(&query, ()).await?;
        let results = utils::libsql_from_row(rows).await?;
        Ok(results)
    }

    #[cfg(not(feature = "turso"))]
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
    /// ```ignore
    /// let users = User::filter(
    ///     kwargs!(age <= 18),
    ///     &conn,
    /// ).await?;
    /// println!("{:#?}", users);
    /// ```
    async fn filter(kw: Vec<Kwargs>, conn: &Connection) -> Result<Vec<Self>, Error>
    where
        Self: Sized + Unpin + for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> + Clone,
    {
        let select_query = builder::to_select_query(kw);

        let query = format!(
            "SELECT * FROM {name} WHERE {placeholders};",
            name = Self::NAME,
            placeholders = select_query.placeholders,
        );

        let mut stream = sqlx::query_as::<_, Self>(sqlx::AssertSqlSafe(query));
        binds!(select_query.args, stream);
        Ok(stream.fetch_all(conn).await?)
    }

    #[cfg(feature = "turso")]
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
    /// ```ignore
    /// let users = User::filter(
    ///     kwargs!(age <= 18),
    ///     &conn,
    /// ).await?;
    /// println!("{:#?}", users);
    /// ```
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

    #[cfg(not(feature = "turso"))]
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
    /// ```ignore
    /// let user = User::get(kwargs!(email == "24nomeniavo@gmail.com"), &conn).await?;
    /// println!("{:#?}", user);
    /// ```
    async fn get(kw: Vec<Kwargs>, conn: &Connection) -> Result<Option<Self>, Error>
    where
        Self: Sized + Unpin + for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> + Clone,
    {
        Ok(Self::filter(kw, conn).await?.first().cloned())
    }

    #[cfg(feature = "turso")]
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
    /// ```ignore
    /// let user = User::get(kwargs!(email == "24nomeniavo@gmail.com"), &conn).await?;
    /// println!("{:#?}", user);
    /// ```
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
    /// ```ignore
    /// let count = User::count(&conn).await?;
    /// println!("User count: {count}");
    /// ```
    async fn count(conn: &Connection) -> Result<i64, Error>
    where
        Self: Sized,
    {
        let query = format!("select count(*) from {name}", name = Self::NAME);
        #[cfg(not(feature = "turso"))]
        {
            let slf = sqlx::query(sqlx::AssertSqlSafe(query))
                .fetch_one(conn)
                .await?;
            Ok(sqlx::Row::try_get(&slf, 0)?)
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
    /// Deletes the record, or every record for a `Vec<T>`, from the table.
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
    /// `Ok(())` on success, or an `Error` if the delete failed.
    ///
    /// # Example
    /// ```ignore
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
    /// async fn main() -> Result<(), rusql_alchemy::Error> {
    ///     let database = Database::new("sqlite::memory:").await?;
    ///     let conn = &database.conn;
    ///
    ///     let products = Product::all(conn).await?;
    ///     products.delete(conn).await?;
    ///
    ///     let remaining = Product::all(conn).await?;
    ///     println!("Remaining products: {:#?}", remaining);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// In the above example, all records from the `Product` table will be deleted.
    async fn delete(&self, conn: &Connection) -> Result<(), Error> {
        let query = format!("delete from {name}", name = T::NAME);
        #[cfg(not(feature = "turso"))]
        sqlx::query(sqlx::AssertSqlSafe(query))
            .execute(conn)
            .await?;

        #[cfg(feature = "turso")]
        conn.execute(&query, ()).await?;
        Ok(())
    }
}

/// Trait for saving collections of models.
///
/// A `Vec<T>` saves every element in order and stops on the first error.
#[async_trait::async_trait]
pub trait Save {
    /// Saves every element of the collection, in order.
    ///
    /// # Returns
    /// `Ok(())` when all saves succeeded, or the first `Error` encountered.
    async fn save(&self, conn: &Connection) -> Result<(), Error>;
}

#[async_trait::async_trait]
impl<T> Save for Vec<T>
where
    T: Model + Sync,
{
    async fn save(&self, conn: &Connection) -> Result<(), Error> {
        for m in self.iter() {
            m.save(conn).await?;
        }
        Ok(())
    }
}
