//! Parsing of the `CREATE TABLE` snapshot that describes a database schema.

use std::collections::BTreeMap;

use sqlparser::{
    ast::{CreateTable, Statement},
    dialect::SQLiteDialect,
    parser::Parser,
};

use super::Result;

/// A set of table definitions, keyed by table name.
#[derive(Debug, Default)]
pub struct Schema {
    tables: BTreeMap<String, CreateTable>,
}

impl Schema {
    /// Parses a script made only of `CREATE TABLE` statements.
    ///
    /// Any other statement, or a table defined twice, is an error.
    pub fn parse(sql: &str) -> Result<Self> {
        let statements = Parser::parse_sql(&SQLiteDialect {}, sql)
            .map_err(|err| format!("invalid schema SQL: {err}"))?;

        let mut tables = BTreeMap::new();

        for statement in statements {
            let Statement::CreateTable(table) = statement else {
                return Err(format!("unsupported schema statement: {statement}").into());
            };

            let name = table.name.to_string();

            if tables.insert(name.clone(), table).is_some() {
                return Err(format!("duplicate table definition: {name}").into());
            }
        }

        Ok(Self { tables })
    }

    /// Iterates over `(name, definition)` pairs in name order.
    pub fn tables(&self) -> impl Iterator<Item = (&str, &CreateTable)> {
        self.tables
            .iter()
            .map(|(name, table)| (name.as_str(), table))
    }

    pub fn get(&self, name: &str) -> Option<&CreateTable> {
        self.tables.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tables.contains_key(name)
    }

    pub fn without(mut self, name: &str) -> Self {
        self.tables
            .retain(|table, _| !table.eq_ignore_ascii_case(name));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tables_by_name() {
        let schema =
            Schema::parse("CREATE TABLE a (id INTEGER); CREATE TABLE b (id INTEGER);").unwrap();

        assert!(schema.contains("a"));
        assert!(schema.contains("b"));
        assert_eq!(schema.tables().count(), 2);
    }

    #[test]
    fn empty_sql_is_an_empty_schema() {
        assert_eq!(Schema::parse("").unwrap().tables().count(), 0);
    }

    #[test]
    fn rejects_duplicate_tables() {
        let err =
            Schema::parse("CREATE TABLE a (id INTEGER); CREATE TABLE a (id INTEGER);").unwrap_err();

        assert!(err.to_string().contains("duplicate table definition: a"));
    }

    #[test]
    fn rejects_non_table_statements() {
        let err = Schema::parse("DROP TABLE a;").unwrap_err();

        assert!(err.to_string().contains("unsupported schema statement"));
    }
}
