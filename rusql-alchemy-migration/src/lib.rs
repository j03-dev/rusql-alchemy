use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use rusql_alchemy::prelude::*;
use sha2::{Digest, Sha256};
use sqlparser::{
    ast::{Ident, ObjectName, Statement},
    dialect::SQLiteDialect,
    parser::Parser,
};

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

type SchemaDiff = Result<(String, String), String>;

type SchemaTables = BTreeMap<String, sqlparser::ast::CreateTable>;

type SchemaDiffResult = Result<SchemaTables, String>;

fn checksum(up: &str, down: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(up.as_bytes());
    hasher.update(b"\0");
    hasher.update(down.as_bytes());

    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn parse_schema(sql: &str) -> SchemaDiffResult {
    let statements = Parser::parse_sql(&SQLiteDialect {}, sql).map_err(|err| err.to_string())?;

    let mut tables = BTreeMap::new();

    for statement in statements {
        match statement {
            Statement::CreateTable(table) => {
                let name = table.name.to_string();

                if tables.insert(name.clone(), table).is_some() {
                    return Err(format!("Duplicate table definition: {name}"));
                }
            }
            other => {
                return Err(format!("Unsupported schema statement: {other}"));
            }
        }
    }

    Ok(tables)
}

fn diff_schema(old_sql: &str, new_sql: &str) -> SchemaDiff {
    let old_tables = parse_schema(old_sql)?;
    let new_tables = parse_schema(new_sql)?;

    let mut up = Vec::new();
    let mut down = Vec::new();

    // Remove tables that no longer exist.
    for name in old_tables.keys() {
        if !new_tables.contains_key(name) {
            up.push(format!("DROP TABLE {name};"));
        }
    }

    // Create newly added tables.
    for (name, table) in &new_tables {
        if !old_tables.contains_key(name) {
            up.push(format!("{};", Statement::CreateTable(table.clone())));
        }
    }

    // Alter existing tables by rebuilding them.
    for (name, old_table) in &old_tables {
        let Some(new_table) = new_tables.get(name) else {
            continue;
        };

        if old_table.to_string() == new_table.to_string() {
            continue;
        }

        let old_columns: BTreeMap<_, _> = old_table
            .columns
            .iter()
            .map(|column| (column.name.to_string(), column))
            .collect();

        let new_columns: BTreeMap<_, _> = new_table
            .columns
            .iter()
            .map(|column| (column.name.to_string(), column))
            .collect();

        let shared_columns: Vec<_> = old_columns
            .keys()
            .filter(|column| new_columns.contains_key(*column))
            .cloned()
            .collect();

        let temp_name = format!("__rusql_alchemy_tmp_{name}");

        // Generate the new table definition under a temporary name.
        let mut temporary_table = new_table.clone();
        temporary_table.name = ObjectName(vec![sqlparser::ast::ObjectNamePart::Identifier(
            Ident::new(&temp_name),
        )]);

        up.push(format!("{};", Statement::CreateTable(temporary_table)));

        // Copy values for columns that exist in both versions.
        if !shared_columns.is_empty() {
            let columns = shared_columns.join(", ");

            up.push(format!(
                "INSERT INTO {temp_name} ({columns}) \
                 SELECT {columns} FROM {name};"
            ));
        }

        up.push(format!("DROP TABLE {name};"));
        up.push(format!("ALTER TABLE {temp_name} RENAME TO {name};"));

        // Generate the reverse rebuild.
        let temp_name = format!("__rusql_alchemy_tmp_{name}");

        let mut temporary_table = old_table.clone();
        temporary_table.name = ObjectName(vec![sqlparser::ast::ObjectNamePart::Identifier(
            Ident::new(&temp_name),
        )]);

        down.push(format!("{};", Statement::CreateTable(temporary_table)));

        if !shared_columns.is_empty() {
            let columns = shared_columns.join(", ");

            down.push(format!(
                "INSERT INTO {temp_name} ({columns}) \
                 SELECT {columns} FROM {name};"
            ));
        }

        down.push(format!("DROP TABLE {name};"));
        down.push(format!("ALTER TABLE {temp_name} RENAME TO {name};"));
    }

    // Reverse table additions.
    for name in new_tables.keys() {
        if !old_tables.contains_key(name) {
            down.push(format!("DROP TABLE {name};"));
        }
    }

    // Restore removed tables.
    for (name, table) in &old_tables {
        if !new_tables.contains_key(name) {
            down.push(format!("{};", Statement::CreateTable(table.clone())));
        }
    }

    Ok((up.join("\n"), down.join("\n")))
}

fn latest_migration(directory: &Path) -> io::Result<Option<(u64, PathBuf)>> {
    let mut latest: Option<(u64, PathBuf)> = None;

    for entry in fs::read_dir(directory)? {
        let path = entry?.path();

        if !path.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        let Some((prefix, _)) = name.split_once('_') else {
            continue;
        };

        let Ok(version) = prefix.parse::<u64>() else {
            continue;
        };

        if latest
            .as_ref()
            .is_none_or(|(current, _)| version > *current)
        {
            latest = Some((version, path));
        }
    }

    Ok(latest)
}

fn migration_directories(directory: &Path) -> io::Result<Vec<(String, PathBuf)>> {
    let mut migrations = Vec::new();

    for entry in fs::read_dir(directory)? {
        let path = entry?.path();

        if !path.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        let Some((prefix, _)) = name.split_once('_') else {
            continue;
        };

        if prefix.parse::<u64>().is_ok() {
            migrations.push((prefix.to_string(), path));
        }
    }

    migrations.sort_by(|a, b| {
        a.0.parse::<u64>()
            .unwrap_or(0)
            .cmp(&b.0.parse::<u64>().unwrap_or(0))
    });

    Ok(migrations)
}

pub fn make_migration(directory: &str) -> Result<(), rusql_alchemy::Error> {
    let directory = Path::new(directory);
    fs::create_dir_all(directory)?;

    // Sort schemas so inventory registration order cannot
    // create false-positive migrations.
    let mut schemas: Vec<&str> = inventory::iter::<Schemas>()
        .map(|model| model.schema)
        .collect();

    schemas.sort_unstable();

    let schemas = schemas.join("\n");

    let latest = latest_migration(directory)?;

    let previous_schema = match &latest {
        Some((_, path)) => fs::read_to_string(path.join("schema.sql"))?,
        None => {
            // Support the original single-schema-file layout.
            let legacy = directory.join("schema.sql");

            if legacy.exists() {
                fs::read_to_string(legacy)?
            } else {
                String::new()
            }
        }
    };

    if previous_schema.trim() == schemas.trim() {
        return Ok(());
    }

    let (up, down) = diff_schema(&previous_schema, &schemas).map_err(io::Error::other)?;

    let version = latest.map_or(1, |(version, _)| version + 1);
    let migration_dir = directory.join(format!("{version:04}_schema_change"));

    // create_dir fails rather than overwriting existing migrations.
    fs::create_dir(&migration_dir)?;

    fs::write(migration_dir.join("up.sql"), up)?;
    fs::write(migration_dir.join("down.sql"), down)?;
    fs::write(migration_dir.join("schema.sql"), schemas)?;

    Ok(())
}

pub async fn migrate(database: &Database, directory: &str) -> Result<(), rusql_alchemy::Error> {
    let directory = Path::new(directory);

    exec_sql(RusqlAlchemyMigration::SCHEMA, &database.conn).await?;

    let migrations = migration_directories(directory)?;

    // Read applied migration records.
    let applied = RusqlAlchemyMigration::all(&database.conn).await?;

    let mut applied_checksums = BTreeMap::new();

    for migration in applied {
        applied_checksums.insert(migration.version.clone(), migration.checksum.clone());
    }

    for (version, path) in migrations {
        let up = fs::read_to_string(path.join("up.sql"))?;
        let down = fs::read_to_string(path.join("down.sql"))?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("migration")
            .to_string();

        let current_checksum = checksum(&up, &down);

        // Never silently accept edits to applied migrations.
        if let Some(stored_checksum) = applied_checksums.get(&version) {
            if stored_checksum != &current_checksum {
                return Err(io::Error::other(format!(
                    "Migration {name} was modified after being applied"
                ))
                .into());
            }

            continue;
        }

        exec_sql(&up, &database.conn).await?;

        RusqlAlchemyMigration {
            version: version.clone(),
            name,
            checksum: current_checksum,
        }
        .save(&database.conn)
        .await?;
    }

    Ok(())
}

async fn exec_sql(sql: &str, conn: &Connection) -> Result<(), rusql_alchemy::Error> {
    #[cfg(feature = "turso")]
    database.conn.execute_batch(sql).await?;

    #[cfg(not(feature = "turso"))]
    sqlx::raw_sql(sql).execute(conn).await?;

    Ok(())
}
