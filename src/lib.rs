#[macro_export]
macro_rules! kwargs {
    ($($key:ident = $value:expr),*) => {
        {
            let mut args = Vec::new();
            $(
                args.push(rust_alchemy::db::models::Arg {
                    key: stringify!($key).to_string(),
                    value: serde_json::json!($value)
                });
            )*
            rust_alchemy::db::models::Kwargs {
                operator: Some(rust_alchemy::db::models::Operator::And),
                args
            }
        }
    };
}

pub mod config {
    pub mod db {
        use libsql::{Builder, Connection};

        async fn establish_connection(url: String, token: String) -> Connection {
            let db = Builder::new_remote_replica("local.db", url, token)
                .build()
                .await
                .unwrap();
            db.connect().unwrap()
        }

        pub struct Database {
            pub conn: Connection,
        }

        impl Database {
            pub async fn new() -> Self {
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
            pub operator: Option<Operator>,
            pub args: Vec<Arg>,
        }

        impl Kwargs {
            pub fn or(self) -> Self {
                Self {
                    operator: Some(Operator::Or),
                    args: self.args,
                }
            }
        }

        #[async_trait]
        pub trait Model: for<'d> Deserialize<'d> {
            fn name() -> String
            where
                Self: Sized;

            async fn conn() -> Connection {
                let database = crate::config::db::Database::new().await;
                database.conn
            }

            async fn save(&self) -> bool
            where
                Self: Sized;

            async fn create(kw: Kwargs) -> bool
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
                    name = Self::name()
                );
                let conn = Self::conn().as_mut().await;
                conn.execute(&query, values).await.is_ok()
            }

            async fn get(kw: Kwargs) -> Self
            where
                Self: Sized,
            {
                let mut fields = Vec::new();
                let mut values = Vec::new();

                for (i, arg) in kw.args.iter().enumerate() {
                    fields.push(format!("{}=?{}", arg.key, i + 1));
                    values.push(arg.value.to_string());
                }
                let fields = fields.join(kw.operator.unwrap().get());
                let query = format!("select * from {name} where {fields};", name = Self::name());

                let conn = Self::conn().as_mut().await;
                let row = conn
                    .query(&query, values)
                    .await
                    .unwrap()
                    .next()
                    .await
                    .unwrap()
                    .unwrap();
                libsql::de::from_row(&row).unwrap()
            }

            async fn all() -> Vec<Self>
            where
                Self: Sized,
            {
                let query = format!("select * from {name}", name = Self::name());

                let conn = Self::conn().as_mut().await;
                let row = conn
                    .query(&query, libsql::params![])
                    .await
                    .unwrap()
                    .next()
                    .await
                    .unwrap()
                    .unwrap();
                libsql::de::from_row(&row).unwrap()
            }

            async fn filter(kw: Kwargs) -> Vec<Self>
            where
                Self: Sized,
            {
                let mut fields = Vec::new();
                let mut values = Vec::new();

                for (i, arg) in kw.args.iter().enumerate() {
                    fields.push(format!("{}=?{}", arg.key, i + 1));
                    values.push(arg.value.to_string());
                }
                let fields = fields.join(kw.operator.unwrap().get());
                let query = format!("select * from {name} where {fields};", name = Self::name());

                let conn = Self::conn().as_mut().await;
                let row = conn
                    .query(&query, values)
                    .await
                    .unwrap()
                    .next()
                    .await
                    .unwrap()
                    .unwrap();
                libsql::de::from_row(&row).unwrap()
            }
        }
    }
}
