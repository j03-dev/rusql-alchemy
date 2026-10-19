//! Command line parsing for the migration CLI.

use super::Result;

pub const USAGE: &str = "\
Usage: <program> <command> [arguments]

Commands:
  make-migration [name]   Generate a migration from the current models
  migrate                 Apply every pending migration
  help                    Show this message";

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// `name` is free text; it is normalized when the migration is written.
    MakeMigration {
        name: Option<String>,
    },
    Migrate,
    Help,
}

impl Command {
    /// Parses the arguments that follow the program name.
    ///
    /// Every word after `make-migration` belongs to the name, so quoting is
    /// optional: `make-migration first migration` and
    /// `make-migration "first migration"` are the same.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut args = args.into_iter();

        match args.next().as_deref() {
            Some("make-migration") => {
                let name = args.collect::<Vec<_>>().join(" ");

                Ok(Self::MakeMigration {
                    name: Some(name).filter(|name| !name.trim().is_empty()),
                })
            }
            Some("migrate") => match args.next() {
                None => Ok(Self::Migrate),
                Some(extra) => Err(format!("unexpected argument `{extra}` for `migrate`").into()),
            },
            Some("help" | "-h" | "--help") => Ok(Self::Help),
            Some(other) => Err(format!("unknown command `{other}`\n\n{USAGE}").into()),
            None => Err(format!("missing command\n\n{USAGE}").into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Command> {
        Command::parse(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn make_migration_without_a_name() {
        assert_eq!(
            parse(&["make-migration"]).unwrap(),
            Command::MakeMigration { name: None }
        );
    }

    #[test]
    fn make_migration_joins_every_word_into_the_name() {
        let expected = Command::MakeMigration {
            name: Some("first migration".into()),
        };

        assert_eq!(
            parse(&["make-migration", "first", "migration"]).unwrap(),
            expected
        );
        assert_eq!(
            parse(&["make-migration", "first migration"]).unwrap(),
            expected
        );
    }

    #[test]
    fn blank_name_counts_as_no_name() {
        assert_eq!(
            parse(&["make-migration", "  "]).unwrap(),
            Command::MakeMigration { name: None }
        );
    }

    #[test]
    fn migrate_takes_no_arguments() {
        assert_eq!(parse(&["migrate"]).unwrap(), Command::Migrate);

        let err = parse(&["migrate", "now"]).unwrap_err();
        assert!(err.to_string().contains("unexpected argument `now`"));
    }

    #[test]
    fn help_has_several_spellings() {
        for flag in ["help", "-h", "--help"] {
            assert_eq!(parse(&[flag]).unwrap(), Command::Help);
        }
    }

    #[test]
    fn unknown_and_missing_commands_show_the_usage() {
        let unknown = parse(&["nope"]).unwrap_err().to_string();
        assert!(unknown.contains("unknown command `nope`") && unknown.contains("Usage:"));

        let missing = parse(&[]).unwrap_err().to_string();
        assert!(missing.contains("missing command") && missing.contains("Usage:"));
    }
}
