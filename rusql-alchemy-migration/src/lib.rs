use rusql_alchemy::prelude::*;
use sha2::{Digest, Sha256};

#[cfg(not(feature = "turso"))]
#[derive(Model, Clone, FromRow)]
pub struct RusqlAlchemyMigration {
    #[field(primary_key = true)]
    pub version: String,
    pub name: String,
    pub checksum: String,
}

#[cfg(feature = "turso")]
#[derive(Model serde::Deserialize, Clone)]
pub struct RusqlAlchemyMigration {
    #[field(primary_key = true)]
    pub version: String,
    pub name: String,
    pub checksum: String,
}

pub async fn migrate(database: &Database, name: &str) -> Result<(), rusql_alchemy::Error> {
    let mut up = String::new();
    let mut down = String::new();
    for model in inventory::iter::<MigrationRegistrar>() {
        up.push_str(model.up_schema);
        down.push_str(model.down_schema);
    }

    database.up().await?;

    let checksum = Sha256::digest(up.as_bytes())
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
