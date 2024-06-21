use serde_json::Value;
use sqlx::{any::AnyRow, FromRow, Row};

use crate::{get_placeholder, get_type_name, Connection};

#[derive(Debug)]
pub enum Operator {
    Or,
    And,
}

impl Operator {
    pub fn get(&self) -> &'static str {
        match self {
            Self::Or => " or ",
            Self::And => " and ",
        }
    }
}

#[derive(Debug)]
pub struct Arg {
    pub key: String,
    pub value: Value,
    pub r#type: String,
}

#[derive(Debug)]
pub struct Kwargs {
    pub operator: Operator,
    pub args: Vec<Arg>,
}

impl Kwargs {
    pub fn or(self) -> Self {
        Self {
            operator: Operator::Or,
            args: self.args,
        }
    }
}

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
    async fn create(kw: Kwargs, conn: &Connection) -> bool
    where
        Self: Sized,
    {
        let ph = get_placeholder();
        let mut fields = Vec::new();
        let mut args = Vec::new();
        let mut placeholder = Vec::new();

        for (i, arg) in kw.args.iter().enumerate() {
            fields.push(arg.key.to_owned());
            args.push((arg.r#type.clone(), arg.value.to_string()));
            placeholder.push(format!("{ph}{index}", index = i + 1,));
        }

        let fields = fields.join(", ");
        let placeholder = placeholder.join(", ");
        let query = format!(
            "insert into {name} ({fields}) values ({placeholder});",
            name = Self::NAME
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
    ///     kwargs!(email = "24nomeniavo@gmail.com", password = "strongpassword"),
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
        kw: Kwargs,
        conn: &Connection,
    ) -> bool {
        let ph = get_placeholder();
        let mut fields = Vec::new();
        let mut args = Vec::new();

        for (i, arg) in kw.args.iter().enumerate() {
            let field = format!("{arg_key}={ph}{index}", arg_key = arg.key, index = i + 1);
            fields.push(field);
            args.push((arg.r#type.clone(), arg.value.to_string()));
        }
        args.push((
            get_type_name(id_value.clone()).to_string(),
            id_value.clone().to_string(),
        ));
        let index_id = fields.len() + 1;
        let fields = fields.join(", ");
        let query = format!(
            "update {name} set {fields} where {id}={ph}{index_id};",
            id = Self::PK,
            name = Self::NAME,
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
        let query = format!("select * from {name}", name = Self::NAME);
        sqlx::query_as::<_, Self>(&query)
            .fetch_all(conn)
            .await
            .unwrap_or_default()
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
    ///     kwargs!(age__gte = 18, weight__lte = 80.0),
    ///     &conn,
    /// ).await;
    /// println!("{:#?}", users);
    /// ```
    async fn filter(kw: Kwargs, conn: &Connection) -> Vec<Self>
    where
        Self: Sized + Unpin + for<'r> FromRow<'r, AnyRow> + Clone,
    {
        let ph = get_placeholder();
        let mut fields = Vec::new();
        let mut args = Vec::new();

        let mut join_query = None;

        for (i, arg) in kw.args.iter().enumerate() {
            let parts: Vec<&str> = arg.key.split("__").collect();
            args.push((arg.r#type.clone(), arg.value.to_string()));
            match parts.as_slice() {
                [field_a, table, field_b] if parts.len() == 3 => {
                    join_query = Some(format!(
                        "INNER JOIN {table} ON {name}.{pk} = {table}.{field_a}",
                        name = Self::NAME,
                        pk = Self::PK
                    ));
                    fields.push(format!("{table}.{field_b}={ph}{index}", index = i + 1));
                }
                _ => fields.push(format!(
                    "{arg_key}={ph}{index}",
                    arg_key = arg.key,
                    index = i + 1,
                )),
            }
        }
        let fields = fields.join(kw.operator.get());
        let query = if let Some(join) = join_query {
            format!(
                "SELECT {name}.* FROM {name} {join} WHERE {fields};",
                name = Self::NAME
            )
        } else {
            format!("SELECT * FROM {name} WHERE {fields};", name = Self::NAME)
        };

        let mut stream = sqlx::query_as::<_, Self>(&query);
        binds!(args, stream);
        stream.fetch_all(conn).await.unwrap_or_default()
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
    ///     kwargs!(email = "24nomeniavo@gmail.com", password = "strongpassword"),
    ///     &conn,
    /// ).await;
    /// println!("{:#?}", user);
    /// ```
    async fn get(kw: Kwargs, conn: &Connection) -> Option<Self>
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
        let query = format!("select count(*) from {name}", name = Self::NAME);
        sqlx::query(query.as_str())
            .fetch_one(conn)
            .await
            .map_or(0, |r| r.get(0))
    }
}

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
        let query = format!("delete from {name}", name = T::NAME);
        sqlx::query(query.as_str()).execute(conn).await.is_ok()
    }
}
