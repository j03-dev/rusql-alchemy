use sqlx::{any::AnyRow, FromRow, Row};

use crate::{get_placeholder, get_type_name, prelude::Kwargs, Connection};

pub(crate) trait Base {
    const SCHEMA: &'static str;
    const NAME: &'static str;
    const PK: &'static str;
}

#[async_trait::async_trait]
pub(crate) trait Migratable: Base {
    async fn migrate(conn: &Connection) -> bool
    where
        Self: Sized,
    {
        if let Err(err) = sqlx::query(Self::SCHEMA).execute(conn).await {
            eprintln!("{err}");
            false
        } else {
            true
        }
    }
}

#[async_trait::async_trait]
pub(crate) trait Creatable: Base {
    async fn save(&self, conn: &Connection) -> bool
    where
        Self: Sized;
    
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
}

#[async_trait::async_trait]
pub(crate) trait Updatable: Base {
    async fn update(&self, conn: &Connection) -> bool
    where
        Self: Sized;

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
    
    async fn delete(&self, conn: &Connection) -> bool
    where
        Self: Sized;
}

#[async_trait::async_trait]
pub(crate) trait Queryable: Base {
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

    async fn all(conn: &Connection) -> Vec<Self>
    where
        Self: Sized + Unpin + for<'r> FromRow<'r, AnyRow> + Clone,
    {
        let query = format!("select * from {name}", name = Self::NAME);
        sqlx::query_as::<_, Self>(&query)
            .fetch_all(conn)
            .await
            .map_or(Vec::new(), |r| r)
    }

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

        let stream = sqlx::query_as::<_, Self>(&query);
        let mut stream = stream;
        binds!(args, stream);
        stream.fetch_all(conn).await.map_or(Vec::new(), |r| r)
    }

    async fn get(kw: Kwargs, conn: &Connection) -> Option<Self>
    where
        Self: Sized + std::marker::Unpin + for<'r> FromRow<'r, AnyRow> + Clone,
    {
        let result = Self::filter(kw, conn).await;
        if let Some(r) = result.first() {
            return Some(r.to_owned());
        }
        None
    }

    async fn count(&self, conn: &Connection) -> usize
    where
        Self: Sized,
    {
        let query = format!("select count(*) from {name}", name = Self::NAME);
        sqlx::query(query.as_str())
            .fetch_one(conn)
            .await
            .map_or(0, |r| r.get::<i64, _>(0) as usize)
    }
}

pub(crate) trait Model: Migratable + Creatable + Updatable + Queryable {}

#[async_trait::async_trait]
pub(crate) trait Delete {
    async fn delete(&self, conn: &Connection) -> bool;
}

#[async_trait::async_trait]
impl<T> Delete for Vec<T>
where
    T: Model + Clone + Sync + Unpin + for<'r> FromRow<'r, AnyRow>,
{
    async fn delete(&self, conn: &Connection) -> bool {
        let query = format!("delete from {name}", name = T::NAME);
        sqlx::query(query.as_str()).execute(conn).await.is_ok()
    }
}
