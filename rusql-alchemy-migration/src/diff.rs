//! Computes the SQL that moves a database from one schema to another.
//!
//! The schema difference is first expressed as a list of [`Change`]s. Each
//! change knows how to apply itself and how to undo itself, so the rollback
//! script is simply the inverse of every change in reverse order.

use std::collections::HashSet;

use sqlparser::ast::{CreateTable, Ident, ObjectName, ObjectNamePart, Statement};

use super::schema::Schema;

const TEMP_TABLE_PREFIX: &str = "__rusql_alchemy_tmp_";

/// The `up` and `down` scripts of a migration.
#[derive(Debug, PartialEq, Eq)]
pub struct SchemaDiff {
    pub up: String,
    pub down: String,
}

impl SchemaDiff {
    pub fn is_empty(&self) -> bool {
        self.up.is_empty() && self.down.is_empty()
    }
}
/// One table-level difference between two schemas.
enum Change<'a> {
    Create(&'a CreateTable),
    Drop(&'a CreateTable),
    Rebuild {
        from: &'a CreateTable,
        to: &'a CreateTable,
    },
}

impl Change<'_> {
    fn up(&self) -> Vec<String> {
        match self {
            Self::Create(table) => vec![create_table(table)],
            Self::Drop(table) => vec![drop_table(table)],
            Self::Rebuild { from, to } => rebuild_table(from, to),
        }
    }

    fn down(&self) -> Vec<String> {
        match self {
            Self::Create(table) => vec![drop_table(table)],
            Self::Drop(table) => vec![create_table(table)],
            Self::Rebuild { from, to } => rebuild_table(to, from),
        }
    }
}

/// Builds the migration scripts that turn `old` into `new`.
pub fn between(old: &Schema, new: &Schema) -> SchemaDiff {
    let changes = changes(old, new);

    SchemaDiff {
        up: join(changes.iter().flat_map(Change::up)),
        down: join(changes.iter().rev().flat_map(Change::down)),
    }
}

fn changes<'a>(old: &'a Schema, new: &'a Schema) -> Vec<Change<'a>> {
    let dropped = old
        .tables()
        .filter(|(name, _)| !new.contains(name))
        .map(|(_, table)| Change::Drop(table));

    let created = new
        .tables()
        .filter(|(name, _)| !old.contains(name))
        .map(|(_, table)| Change::Create(table));

    let rebuilt = old.tables().filter_map(|(name, from)| {
        let to = new.get(name)?;

        // Compare the rendered SQL so that formatting-only differences in
        // the source schema are not treated as changes.
        (from.to_string() != to.to_string()).then_some(Change::Rebuild { from, to })
    });

    dropped.chain(created).chain(rebuilt).collect()
}

/// Recreates a table with a new definition, keeping the data of every
/// column that exists in both definitions.
///
/// This is the portable way to alter a table: build the new one under a
/// temporary name, copy the rows, drop the old one and rename.
fn rebuild_table(from: &CreateTable, to: &CreateTable) -> Vec<String> {
    let name = &from.name;
    let temp = temp_name(name);

    let mut staged = to.clone();
    staged.name = temp.clone();

    let mut statements = vec![create_table(&staged)];

    let columns = shared_columns(from, to);
    if !columns.is_empty() {
        let columns = columns.join(", ");

        statements.push(format!(
            "INSERT INTO {temp} ({columns}) SELECT {columns} FROM {name};"
        ));
    }

    statements.push(format!("DROP TABLE {name};"));
    statements.push(format!("ALTER TABLE {temp} RENAME TO {name};"));

    statements
}

/// Names of the columns of `from` that are still present in `to`, in the
/// order they are declared in `from`.
fn shared_columns(from: &CreateTable, to: &CreateTable) -> Vec<String> {
    let kept: HashSet<String> = to.columns.iter().map(|c| c.name.to_string()).collect();

    from.columns
        .iter()
        .map(|column| column.name.to_string())
        .filter(|name| kept.contains(name))
        .collect()
}

fn temp_name(name: &ObjectName) -> ObjectName {
    let ident = Ident::new(format!("{TEMP_TABLE_PREFIX}{name}"));

    ObjectName(vec![ObjectNamePart::Identifier(ident)])
}

fn create_table(table: &CreateTable) -> String {
    format!("{};", Statement::CreateTable(table.clone()))
}

fn drop_table(table: &CreateTable) -> String {
    format!("DROP TABLE {};", table.name)
}

fn join(statements: impl Iterator<Item = String>) -> String {
    statements.collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diff(old: &str, new: &str) -> SchemaDiff {
        between(&Schema::parse(old).unwrap(), &Schema::parse(new).unwrap())
    }

    #[test]
    fn identical_schemas_produce_nothing() {
        let sql = "CREATE TABLE users (id INTEGER PRIMARY KEY);";

        assert_eq!(
            diff(sql, sql),
            SchemaDiff {
                up: String::new(),
                down: String::new()
            }
        );
    }

    #[test]
    fn new_table_is_created_and_dropped_on_rollback() {
        let result = diff("", "CREATE TABLE users (id INTEGER);");

        assert_eq!(result.up, "CREATE TABLE users (id INTEGER);");
        assert_eq!(result.down, "DROP TABLE users;");
    }

    #[test]
    fn removed_table_is_dropped_and_restored_on_rollback() {
        let result = diff("CREATE TABLE users (id INTEGER);", "");

        assert_eq!(result.up, "DROP TABLE users;");
        assert_eq!(result.down, "CREATE TABLE users (id INTEGER);");
    }

    #[test]
    fn added_column_rebuilds_the_table_keeping_shared_columns() {
        let result = diff(
            "CREATE TABLE users (id INTEGER, name TEXT);",
            "CREATE TABLE users (id INTEGER, name TEXT, email TEXT);",
        );

        assert_eq!(
            result.up,
            "CREATE TABLE __rusql_alchemy_tmp_users (id INTEGER, name TEXT, email TEXT);\n\
             INSERT INTO __rusql_alchemy_tmp_users (id, name) SELECT id, name FROM users;\n\
             DROP TABLE users;\n\
             ALTER TABLE __rusql_alchemy_tmp_users RENAME TO users;"
        );
        assert_eq!(
            result.down,
            "CREATE TABLE __rusql_alchemy_tmp_users (id INTEGER, name TEXT);\n\
             INSERT INTO __rusql_alchemy_tmp_users (id, name) SELECT id, name FROM users;\n\
             DROP TABLE users;\n\
             ALTER TABLE __rusql_alchemy_tmp_users RENAME TO users;"
        );
    }

    #[test]
    fn rollback_undoes_changes_in_reverse_order() {
        let result = diff(
            "CREATE TABLE old_table (id INTEGER);",
            "CREATE TABLE new_table (id INTEGER);",
        );

        assert_eq!(
            result.up,
            "DROP TABLE old_table;\nCREATE TABLE new_table (id INTEGER);"
        );
        assert_eq!(
            result.down,
            "DROP TABLE new_table;\nCREATE TABLE old_table (id INTEGER);"
        );
    }

    #[test]
    fn rebuild_without_shared_columns_skips_the_copy() {
        let result = diff("CREATE TABLE t (a INTEGER);", "CREATE TABLE t (b INTEGER);");

        assert!(!result.up.contains("INSERT INTO"));
        assert!(!result.down.contains("INSERT INTO"));
    }
}
