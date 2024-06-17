#[macro_export]
macro_rules! kwargs {
    ($($key:ident = $value:expr),*) => {
        {
            let mut args = Vec::new();
            $(
                args.push(rusql_alchemy::db::models::Arg {
                    key: stringify!($key).to_string(),
                    value: rusql_alchemy::to_value($value.clone())
                });
            )*
            rusql_alchemy::db::models::Kwargs {
                operator: rusql_alchemy::db::models::Operator::And,
                args
            }
        }
    };
}

pub fn to_value(value: impl Into<serde_json::Value>) -> serde_json::Value {
    let json_value = value.into();
    match json_value {
        serde_json::Value::Bool(true) => serde_json::json!(1),
        serde_json::Value::Bool(false) => serde_json::json!(0),
        _ => json_value,
    }
}

#[macro_export]
macro_rules! migrate {
    ([$($struct:ident),*], $conn:expr) => {
        $( $struct::migrate($conn).await; )*
    };
}

pub mod config {
    pub mod db {
        use libsql::{Connection, Database as LibsqlDatabase};

        async fn establish_connection(url: String, token: String) -> Connection {
            let db = LibsqlDatabase::open_remote(url, token).unwrap();
            db.connect().unwrap()
        }

        pub struct Database {
            pub conn: Connection,
        }

        impl Database {
            pub async fn new() -> Self {
                dotenv::dotenv().ok();
                let turso_database_url = std::env::var("DATABASE_URL").unwrap();
                let turso_auth_token = std::env::var("TOKEN_KEY").unwrap();
                Self {
                    conn: establish_connection(turso_database_url, turso_auth_token).await,
                }
            }
        }
    }
}

pub mod db {
    pub mod models {
        use async_trait::async_trait;
        use libsql::Connection;
        use serde::Deserialize;
        use serde_json::Value;

        pub type Integer = i32;
        pub type Date = String;
        pub type DateTime = String;
        pub type Text = String;
        pub type Float = f32;

        pub enum Operator {
            Or,
            And,
        }

        impl Operator {
            fn get(&self) -> &'static str {
                match self {
                    Self::Or => " or ",
                    Self::And => " and ",
                }
            }
        }

        pub struct Arg {
            pub key: String,
            pub value: Value,
        }

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

        #[async_trait]
        pub trait Model: Clone + Sync + for<'d> Deserialize<'d> {
            const SCHEMA: &'static str;
            const NAME: &'static str;
            const PK: &'static str;

            async fn migrate(conn: &Connection) -> bool
            where
                Self: Sized,
            {
                conn.execute(Self::SCHEMA, libsql::params![]).await.is_ok()
            }

            async fn update(&self, conn: &Connection) -> bool
            where
                Self: Sized;

            async fn set<T: ToString + Send + Sync>(
                id_value: T,
                kw: Kwargs,
                conn: &Connection,
            ) -> bool {
                let mut fields = Vec::new();
                let mut values = Vec::new();

                for (i, arg) in kw.args.iter().enumerate() {
                    fields.push(format!("{}=?{}", arg.key, i + 1));
                    values.push(arg.value.to_string());
                }
                values.push(id_value.to_string());
                let fields = fields.join(", ");
                let query = format!(
                    "update {name} set {fields} where {id}=?{index_id};",
                    id = Self::PK,
                    name = Self::NAME,
                    index_id = fields.len() + 1
                );
                values = values.iter().map(|v| v.replace('"', "")).collect();
                conn.execute(&query, values).await.is_ok()
            }

            async fn save(&self, conn: &Connection) -> bool
            where
                Self: Sized;

            async fn create(kw: Kwargs, conn: &Connection) -> bool
            where
                Self: Sized,
            {
                let mut fields = Vec::new();
                let mut values = Vec::new();
                let mut placeholder = Vec::new();

                for (i, arg) in kw.args.iter().enumerate() {
                    fields.push(arg.key.to_owned());
                    values.push(arg.value.to_string());
                    placeholder.push(format!("?{}", i + 1));
                }

                let fields = fields.join(", ");
                let placeholder = placeholder.join(", ");
                let query = format!(
                    "insert into {name} ({fields}) values ({placeholder});",
                    name = Self::NAME
                );
                values = values.iter().map(|v| v.replace('"', "")).collect();
                conn.execute(&query, values).await.is_ok()
            }

            async fn all(conn: &Connection) -> Vec<Self>
            where
                Self: Sized,
            {
                let query = format!("select * from {name}", name = Self::NAME);

                let mut result = Vec::new();
                if let Ok(mut rows) = conn.query(&query, libsql::params![]).await {
                    while let Ok(Some(row)) = rows.next() {
                        if let Ok(model) = libsql::de::from_row(&row) {
                            result.push(model);
                        }
                    }
                }
                result
            }

            async fn filter(kw: Kwargs, conn: &Connection) -> Vec<Self>
            where
                Self: Sized,
            {
                let mut fields = Vec::new();

                let mut join_query = None;

                for (i, arg) in kw.args.iter().enumerate() {
                    let parts: Vec<&str> = arg.key.split("__").collect();
                    match parts.as_slice() {
                        [field_a, table, field_b] if parts.len() == 3 => {
                            join_query = Some(format!(
                                "INNER JOIN {table} ON {name}.{pk} = {table}.{field_a}",
                                name = Self::NAME,
                                pk = Self::PK
                            ));
                            fields.push(format!("{table}.{field_b}=?{}", i + 1));
                        }
                        _ => fields.push(format!("{}=?{}", arg.key, i + 1)),
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

                let values: Vec<_> = kw
                    .args
                    .iter()
                    .map(|arg| arg.value.to_string().replace('"', ""))
                    .collect();

                let mut result = Vec::new();
                if let Ok(mut rows) = conn.query(&query, values.clone()).await {
                    while let Ok(Some(row)) = rows.next() {
                        if let Ok(model) = libsql::de::from_row(&row) {
                            result.push(model);
                        }
                    }
                }
                result
            }

            async fn get(kw: Kwargs, conn: &Connection) -> Option<Self>
            where
                Self: Sized,
            {
                let result = Self::filter(kw, conn).await;
                if let Some(r) = result.first() {
                    return Some(r.to_owned());
                }
                None
            }

            async fn delete(&self, conn: &Connection) -> bool
            where
                Self: Sized;

            async fn count(&self, conn: &Connection) -> usize
            where
                Self: Sized,
            {
                let query = format!("select count(*) from {name}", name = Self::NAME);
                let mut results = conn.query(&query, libsql::params![]).await.unwrap();

                let mut count: usize = 0;

                while let Some(row) = results.next().unwrap() {
                    count = row.get::<i32>(0).unwrap_or_default() as usize;
                }

                count
            }
        }

        #[async_trait]
        pub trait Delete {
            async fn delete(&self, conn: &Connection) -> bool;
        }

        #[async_trait]
        impl<T> Delete for Vec<T>
        where
            T: Model,
        {
            async fn delete(&self, conn: &Connection) -> bool {
                let query = format!("delete from {name}", name = T::NAME);
                conn.execute(&query, libsql::params![]).await.is_ok()
            }
        }
    }
}

pub mod prelude {
    pub use crate::{
        config,
        db::models::{Date, DateTime, Delete, Float, Integer, Model, Text},
        kwargs, migrate,
    };
    pub use async_trait::async_trait;
    pub use libsql::Connection;
    pub use rusql_alchemy_macro::Model;
    pub use serde::{Deserialize, Serialize};
    pub use serde_json;
}
