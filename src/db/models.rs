//! RusQL Alchemy: A Rust ORM library for SQL databases
//!
//! This module provides traits and implementations for database operations,
//! including querying, inserting, updating, and deleting records.

use lazy_static::lazy_static;
use sqlx::{any::AnyRow, FromRow, Row};

use crate::{get_placeholder, get_type_name, Connection};

lazy_static! {
    /// The placeholder string for SQL queries, determined by the database type.
    pub static ref PLACEHOLDER: &'static str = get_placeholder().expect(
        "DATABASE_URL is not set, make sur the database is 'sqlite', 'postgres' or 'mysql'"
    );
}

/// Represents a condition in a database query.
#[derive(Debug)]
pub enum Condition {
    /// A condition on a specific field.
    FieldCondition {
        field: String,
        value: String,
        value_type: String,
        comparison_operator: String,
    },
    /// A logical operator (AND/OR) for combining conditions.
    LogicalOperator { operator: String },
}

/// Trait for adding OR conditions to a vector of conditions.
pub trait Or {
    /// Adds OR conditions to the existing conditions.
    fn or(self, conditions: Vec<Condition>) -> Vec<Condition>;
}

/// Trait for adding AND conditions to a vector of conditions.
pub trait And {
    /// Adds AND conditions to the existing conditions.
    fn and(self, conditions: Vec<Condition>) -> Vec<Condition>;
}

impl Or for Vec<Condition> {
    fn or(mut self, conditions: Vec<Condition>) -> Vec<Condition> {
        self.push(Condition::LogicalOperator {
            operator: "or".to_string(),
        });
        self.extend(conditions);
        self
    }
}

impl And for Vec<Condition> {
    fn and(mut self, conditions: Vec<Condition>) -> Vec<Condition> {
        self.push(Condition::LogicalOperator {
            operator: "and".to_string(),
        });
        self.extend(conditions);
        self
    }
}

/// Trait for generating SQL queries from conditions.
pub trait Query {
    /// Generates an UPDATE query from the conditions.
    fn to_update_query(&self) -> (String, Vec<(String, String)>);
    /// Generates a SELECT query from the conditions.
    fn to_select_query(&self) -> (String, Vec<(String, String)>);
    /// Generates an INSERT query from the conditions.
    fn to_insert_query(&self) -> (String, String, Vec<(String, String)>);
}

impl Query for Vec<Condition> {
    //                               (placeholders, args:[(value, type)])])
    fn to_update_query(&self) -> (String, Vec<(String, String)>) {
        let mut args = Vec::new();
        let mut placeholders = Vec::new();
        let mut index = 0;
        for condition in self {
            if let Condition::FieldCondition {
                field,
                value,
                value_type,
                ..
            } = condition
            {
                index += 1;
                args.push((value.clone(), value_type.clone()));
                // (field + = + placeholder + index)
                let placeholder = PLACEHOLDER.to_string();
                placeholders.push(format!("{field}={placeholder}{index}",));
            }
        }
        (placeholders.join(", "), args)
    }

    //                               (placeholders, args)
    fn to_select_query(&self) -> (String, Vec<(String, String)>) {
        let mut args = Vec::new();
        let mut placeholders = Vec::new();
        let mut index = 0;
        for condition in self {
            match condition {
                Condition::FieldCondition {
                    field,
                    value,
                    value_type,
                    comparison_operator,
                } => {
                    index += 1;
                    args.push((value.clone(), value_type.clone()));
                    // (field + = + placeholder + index)
                    let placeholder = PLACEHOLDER.to_string();
                    placeholders.push(format!("{field}{comparison_operator}{placeholder}{index}",));
                }
                Condition::LogicalOperator { operator } => {
                    placeholders.push(operator.to_owned());
                }
            }
        }
        (placeholders.join(" "), args)
    }

    //                              fields, placeholders, args:[(value, type)]
    fn to_insert_query(&self) -> (String, String, Vec<(String, String)>) {
        let mut args = Vec::new();
        let mut fields = Vec::new();
        let mut placeholders = Vec::new();
        let mut index = 0;
        for condition in self {
            if let Condition::FieldCondition {
                field,
                value,
                value_type,
                ..
            } = condition
            {
                index += 1;
                args.push((value.clone(), value_type.clone()));
                fields.push(field.clone());
                let placeholder = PLACEHOLDER.to_string();
                placeholders.push(format!("{placeholder}{index}"));
            }
        }
        (fields.join(", "), placeholders.join(", "), args)
    }
}

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
    async fn migrate(conn: &Connection) -> bool
    where
        Self: Sized,
    {
        println!("{:?}", Self::SCHEMA);
        if let Err(err) = sqlx::query(Self::SCHEMA).execute(conn).await {
            eprintln!("Error during the migration\n->{err}");
            false
        } else {
            true
        }
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
    async fn save(&self, conn: &Connection) -> bool
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
    async fn create(kw: Vec<Condition>, conn: &Connection) -> bool
    where
        Self: Sized,
    {
        let (fields, placeholders, args) = kw.to_insert_query();

        let query = format!(
            "insert into {table_name} ({fields}) values ({placeholders});",
            table_name = Self::NAME
        );
        let mut stream = sqlx::query(&query);
        binds!(args, stream);
        stream.execute(conn).await.is_ok()
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
    ///     kwargs!(email = "24nomeniavo@gmail.com").and(kwargs!(password = "strongpassword")),
    ///     &conn,
    /// ).await {
    ///     user.role = "admin".to_string();
    ///     let success = user.update(&conn).await;
    ///     println!("Update success: {}", success);
    /// }
    /// ```
    async fn update(&self, conn: &Connection) -> bool
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
    async fn set<T: ToString + Clone + Send + Sync>(
        id_value: T,
        kw: Vec<Condition>,
        conn: &Connection,
    ) -> bool {
        let (placeholders, mut args) = kw.to_update_query();

        args.push((
            id_value.clone().to_string(),
            get_type_name(id_value.clone()).to_string(),
        ));
        let index_id = args.len();
        let placeholder = PLACEHOLDER.to_string();
        let query = format!(
            "update {table_name} set {placeholders} where {id}={placeholder}{index_id};",
            id = Self::PK,
            table_name = Self::NAME,
        );

        let mut stream = sqlx::query(&query);
        binds!(args, stream);
        stream.execute(conn).await.is_ok()
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
    async fn delete(&self, conn: &Connection) -> bool
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
    async fn all(conn: &Connection) -> Vec<Self>
    where
        Self: Sized + Unpin + for<'r> FromRow<'r, AnyRow> + Clone,
    {
        let query = format!("select * from {table_name}", table_name = Self::NAME);
        println!("query {query}");

        match sqlx::query_as::<_, Self>(&query).fetch_all(conn).await {
            Ok(result) => result,
            Err(err) => {
                println!("{err:?}");
                Default::default()
            }
        }
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
    async fn filter(kw: Vec<Condition>, conn: &Connection) -> Vec<Self>
    where
        Self: Sized + Unpin + for<'r> FromRow<'r, AnyRow> + Clone,
    {
        let (fields, args) = kw.to_select_query();

        let query = format!(
            "SELECT * FROM {table_name} WHERE {fields};",
            table_name = Self::NAME
        );

        let mut stream = sqlx::query_as::<_, Self>(&query);
        binds!(args, stream);

        println!("query = {query}");
        match stream.fetch_all(conn).await {
            Ok(result) => result,
            Err(err) => {
                println!("{err:?}");
                Default::default()
            }
        }
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
    async fn get(kw: Vec<Condition>, conn: &Connection) -> Option<Self>
    where
        Self: Sized + Unpin + for<'r> FromRow<'r, AnyRow> + Clone,
    {
        Self::filter(kw, conn).await.first().cloned()
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
    async fn count(&self, conn: &Connection) -> i64
    where
        Self: Sized,
    {
        let query = format!("select count(*) from {table_name}", table_name = Self::NAME);
        sqlx::query(query.as_str())
            .fetch_one(conn)
            .await
            .map_or(0, |r| r.get(0))
    }
}

/// Trait for deleting database records.
#[async_trait::async_trait]
pub trait Delete {
    async fn delete(&self, conn: &Connection) -> bool;
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
    /// #     #[model(primary_key = true, auto = true, null = false)]
    /// #     id: Integer,
    /// #     #[model(size = 50, null = false)]
    /// #     name: String,
    /// #     price: Float,
    /// #     description: Text,
    /// #     #[model(default = true)]
    /// #     is_sel: Boolean,
    /// #     #[model(null = false, foreign_key = "User.id")]
    /// #     owner: Integer,
    /// #     #[model(default = "now")]
    /// #     at: DateTime,
    /// # }
    /// #
    /// #[tokio::main]
    /// async fn main() {
    ///     let conn = Database::new().await.conn;
    ///
    ///     let products = Product::all(&conn).await;
    ///     let success = products.delete(&conn).await;
    ///     println!("Products delete success: {}", success);
    ///
    ///     let products = Product::all(&conn).await;
    ///     println!("Remaining products: {:#?}", products);
    /// }
    /// ```
    ///
    /// In the above example, all records from the `Product` table will be deleted.
    async fn delete(&self, conn: &Connection) -> bool {
        let query = format!("delete from {table_name}", table_name = T::NAME);
        sqlx::query(query.as_str()).execute(conn).await.is_ok()
    }
}
