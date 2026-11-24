//! Computes the SQL that moves a database from one schema to another.
//!
//! The schema difference is first expressed as a list of [`Change`]s. Each
//! change knows how to apply itself and how to undo itself, so the rollback
//! script is simply the inverse of every change in reverse order.

use std::collections::{HashMap, HashSet};

use sqlparser::ast::{
    ColumnOption, CreateTable, Ident, ObjectName, ObjectNamePart, Statement, TableConstraint,
};

use super::{Result, schema::Schema};

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
pub fn between(old: &Schema, new: &Schema) -> Result<SchemaDiff> {
    let changes = changes(old, new)?;

    Ok(SchemaDiff {
        up: join(changes.iter().flat_map(Change::up)),
        down: join(changes.iter().rev().flat_map(Change::down)),
    })
}

fn changes<'a>(old: &'a Schema, new: &'a Schema) -> Result<Vec<Change<'a>>> {
    // Tables being created must come after the tables they reference: PostgreSQL
    // rejects `REFERENCES` to a table that does not exist yet. The schema is a
    // BTreeMap (name order), which is NOT a valid creation order.
    let created = topological_tables(
        new.tables()
            .filter(|(name, _)| !old.contains(name))
            .map(|(_, table)| table),
    )?;

    // Dropping is the mirror image: children first, parents last.
    let mut dropped = topological_tables(
        old.tables()
            .filter(|(name, _)| !new.contains(name))
            .map(|(_, table)| table),
    )?;
    dropped.reverse();

    let rebuilt = old.tables().filter_map(|(name, from)| {
        let to = new.get(name)?;

        // Compare the rendered SQL so that formatting-only differences in
        // the source schema are not treated as changes.
        (from.to_string() != to.to_string()).then_some(Change::Rebuild { from, to })
    });

    Ok(dropped
        .into_iter()
        .map(Change::Drop)
        .chain(created.into_iter().map(Change::Create))
        .chain(rebuilt)
        .collect())
}

/// Orders `tables` so that every referenced table inside the same set is
/// created first (Kahn-style DFS with an explicit cycle error).
///
/// References to tables outside `tables` (already present in the database) are
/// ignored: they are dependencies we do not need to order here.
fn topological_tables<'a>(
    tables: impl Iterator<Item = &'a CreateTable>,
) -> Result<Vec<&'a CreateTable>> {
    let by_name: HashMap<String, &'a CreateTable> = tables
        .map(|table| (normalize_name(&table.name), table))
        .collect();

    let mut names: Vec<String> = by_name.keys().cloned().collect();
    names.sort();

    let mut ordered = Vec::with_capacity(by_name.len());
    let mut done = HashSet::new();
    for name in names {
        visit_table(&name, &by_name, &mut done, &mut Vec::new(), &mut ordered)?;
    }

    Ok(ordered)
}

fn visit_table<'a>(
    name: &str,
    by_name: &HashMap<String, &'a CreateTable>,
    done: &mut HashSet<String>,
    stack: &mut Vec<String>,
    ordered: &mut Vec<&'a CreateTable>,
) -> Result<()> {
    if done.contains(name) {
        return Ok(());
    }
    if stack.iter().any(|entry| entry == name) {
        stack.push(name.to_string());
        return Err(format!("cyclic foreign keys: {}", stack.join(" -> ")).into());
    }
    let Some(table) = by_name.get(name) else {
        // Reference to a table that already exists (or is not part of this
        // migration) — nothing to order.
        return Ok(());
    };

    stack.push(name.to_string());
    for reference in referenced_tables(table) {
        if by_name.contains_key(&reference) {
            visit_table(&reference, by_name, done, stack, ordered)?;
        }
    }
    stack.pop();

    done.insert(name.to_string());
    ordered.push(*table);
    Ok(())
}

/// Every table name this `CREATE TABLE` has a foreign key to, lower-cased so
/// that `AppUser` and `appuser` match (PostgreSQL folds identifiers).
fn referenced_tables(table: &CreateTable) -> Vec<String> {
    let mut refs = Vec::new();

    for column in &table.columns {
        for option in &column.options {
            if let ColumnOption::ForeignKey(fk) = &option.option {
                refs.push(normalize_name(&fk.foreign_table));
            }
        }
    }
    for constraint in &table.constraints {
        if let TableConstraint::ForeignKey(fk) = constraint {
            refs.push(normalize_name(&fk.foreign_table));
        }
    }

    refs.sort();
    refs.dedup();
    refs
}

fn normalize_name(name: &ObjectName) -> String {
    name.to_string().trim_matches('"').to_ascii_lowercase()
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
        between(&Schema::parse(old).unwrap(), &Schema::parse(new).unwrap()).unwrap()
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

    #[test]
    fn creates_follow_foreign_key_dependencies() {
        // Alphabetically, AppSettings comes first — but it references AppUser.
        let result = diff(
            "",
            "CREATE TABLE AppSettings (id INTEGER, user_id INTEGER REFERENCES AppUser (id) ON DELETE CASCADE); \
             CREATE TABLE AppUser (id INTEGER);",
        );

        let user = result.up.find("CREATE TABLE AppUser").unwrap();
        let settings = result.up.find("CREATE TABLE AppSettings").unwrap();
        assert!(user < settings, "up: {}", result.up);

        let drop_settings = result.down.find("DROP TABLE AppSettings").unwrap();
        let drop_user = result.down.find("DROP TABLE AppUser").unwrap();
        assert!(drop_settings < drop_user, "down: {}", result.down);
    }

    #[test]
    fn drops_happen_child_first() {
        let old = "CREATE TABLE AppSettings (id INTEGER, user_id INTEGER REFERENCES AppUser (id)); \
                   CREATE TABLE AppUser (id INTEGER);";
        let result = diff(old, "");

        assert_eq!(result.up, "DROP TABLE AppSettings;\nDROP TABLE AppUser;");
        assert!(
            result.down.find("CREATE TABLE AppUser").unwrap()
                < result.down.find("CREATE TABLE AppSettings").unwrap()
        );
    }

    #[test]
    fn cyclic_foreign_keys_are_rejected_with_a_clear_error() {
        let err = between(
            &Schema::parse("").unwrap(),
            &Schema::parse(
                "CREATE TABLE a (id INTEGER, b_id INTEGER REFERENCES b (id)); \
                 CREATE TABLE b (id INTEGER, a_id INTEGER REFERENCES a (id));",
            )
            .unwrap(),
        )
        .unwrap_err();

        assert!(err.to_string().contains("cyclic foreign keys"), "{err}");
    }
}
