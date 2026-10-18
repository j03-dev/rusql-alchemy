use std::path;

use rusql_alchemy::prelude::*;
use sha2::{Digest, Sha256};

#[cfg(not(feature = "turso"))]
#[derive(Model, Clone, FromRow)]
struct RusqlAlchemyMigration {
    #[field(primary_key = true)]
    version: String,
    name: String,
    checksum: String,
}

#[cfg(feature = "turso")]
#[derive(Model serde::Deserialize, Clone)]
struct RusqlAlchemyMigration {
    #[field(primary_key = true)]
    version: String,
    name: String,
    checksum: String,
}

pub fn make_migration(directory: &str) -> Result<(), rusql_alchemy::Error> {
    // TODO: we should make diff with existing and new one
    let mut schemas = String::new();
    for model in inventory::iter::<Schemas>() {
        schemas.push_str(model.schema);
    }
    let path = format!("{directory}/schema.sql");
    std::fs::write(path, schemas)?;
    Ok(())
}

pub async fn migrate(database: &Database, name: &str) -> Result<(), rusql_alchemy::Error> {
    let mut schemas = String::new();
    for model in inventory::iter::<Schemas>() {
        schemas.push_str(model.schema);
    }

    // database.up().await?;

    let checksum = Sha256::digest(schemas.as_bytes())
        .iter()
        .map(|bytes| format!("{bytes:02x}"))
        .collect::<String>();
    let version = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();

    RusqlAlchemyMigration {
        version: version,
        name: name.to_string(),
        checksum: checksum,
    }
    .save(&database.conn)
    .await?;

    Ok(())
}
