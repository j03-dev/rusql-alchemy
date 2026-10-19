//! On-disk layout of migrations.
//!
//! ```text
//! <root>/
//! ├── 0001_schema_change/
//! │   ├── up.sql
//! │   ├── down.sql
//! │   └── schema.sql      <- full schema after this migration
//! └── 0002_schema_change/
//! ```

use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

use sha2::{Digest, Sha256};

use super::{Error, Result, diff::SchemaDiff};

const UP: &str = "up.sql";
const DOWN: &str = "down.sql";
const SCHEMA: &str = "schema.sql";

/// Label used when a migration is written without a usable name.
const DEFAULT_NAME: &str = "schema_change";

/// Position of a migration in the history.
///
/// Versions compare numerically, so `0002` and `2` are the same version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(u64);

impl Version {
    pub const FIRST: Self = Self(1);

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}", self.0)
    }
}

impl FromStr for Version {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

/// A migration directory found on disk.
#[derive(Debug)]
pub struct MigrationEntry {
    pub version: Version,
    pub name: String,
    path: PathBuf,
}

impl MigrationEntry {
    /// Returns `None` for anything that is not named `<number>_<label>`.
    fn from_path(path: PathBuf) -> Option<Self> {
        if !path.is_dir() {
            return None;
        }

        let name = path.file_name()?.to_str()?.to_owned();
        let (prefix, _) = name.split_once('_')?;
        let version = prefix.parse().ok()?;

        Some(Self {
            version,
            name,
            path,
        })
    }

    pub fn read_up(&self) -> Result<String> {
        read(&self.path.join(UP))
    }

    pub fn read_down(&self) -> Result<String> {
        read(&self.path.join(DOWN))
    }

    fn read_schema(&self) -> Result<String> {
        read(&self.path.join(SCHEMA))
    }
}

/// The directory that holds every migration.
#[derive(Debug)]
pub struct MigrationDir {
    root: PathBuf,
}

impl MigrationDir {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn ensure_exists(&self) -> Result<()> {
        fs::create_dir_all(&self.root).map_err(|err| io_error("create", &self.root, err))
    }

    /// Lists every migration, ordered from oldest to newest.
    pub fn scan(&self) -> Result<Vec<MigrationEntry>> {
        let entries = fs::read_dir(&self.root).map_err(|err| io_error("read", &self.root, err))?;

        let mut migrations = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|err| io_error("read", &self.root, err))?;

            migrations.extend(MigrationEntry::from_path(entry.path()));
        }

        migrations.sort_by_key(|migration| migration.version);

        if let Some(pair) = migrations
            .windows(2)
            .find(|pair| pair[0].version == pair[1].version)
        {
            return Err(format!(
                "migrations {} and {} share the same version",
                pair[0].name, pair[1].name
            )
            .into());
        }

        Ok(migrations)
    }

    /// The schema snapshot that the next migration is compared against.
    ///
    /// Falls back to a root-level `schema.sql` (the original layout), and
    /// to an empty schema when there is nothing yet.
    pub fn previous_schema(&self, latest: Option<&MigrationEntry>) -> Result<String> {
        if let Some(latest) = latest {
            return latest.read_schema();
        }

        let legacy = self.root.join(SCHEMA);

        if legacy.exists() {
            read(&legacy)
        } else {
            Ok(String::new())
        }
    }

    /// Writes a new migration and returns its directory name.
    ///
    /// `name` is free text and is normalized with [`slug`]; an empty or
    /// unusable name falls back to `schema_change`. Fails instead of
    /// overwriting an existing migration.
    pub fn write(
        &self,
        version: Version,
        name: &str,
        diff: &SchemaDiff,
        schema: &str,
    ) -> Result<String> {
        let label = Some(slug(name))
            .filter(|label| !label.is_empty())
            .unwrap_or_else(|| DEFAULT_NAME.to_owned());

        let dir_name = format!("{version}_{label}");
        let dir = self.root.join(&dir_name);

        fs::create_dir(&dir).map_err(|err| io_error("create", &dir, err))?;

        write(&dir.join(UP), &diff.up)?;
        write(&dir.join(DOWN), &diff.down)?;
        write(&dir.join(SCHEMA), schema)?;

        Ok(dir_name)
    }
}

/// Normalizes free text into a directory-safe label: lowercase ASCII
/// letters and digits separated by single underscores.
///
/// `"First migration!"` becomes `"first_migration"`.
pub fn slug(text: &str) -> String {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("_")
}

/// Fingerprint of a migration's scripts, used to detect edits after the
/// migration has been applied.
pub fn checksum(up: &str, down: &str) -> String {
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

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|err| io_error("read", path, err))
}

fn write(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents).map_err(|err| io_error("write", path, err))
}

fn io_error(action: &str, path: &Path, err: io::Error) -> Error {
    format!("failed to {action} {}: {err}", path.display()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty directory that is removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("rusql_alchemy_{label}_{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn version_is_numeric_and_padded() {
        assert!("0002".parse::<Version>().unwrap() > "0001".parse().unwrap());
        assert!("10".parse::<Version>().unwrap() > "9".parse().unwrap());
        assert_eq!("2".parse::<Version>().unwrap(), "0002".parse().unwrap());
        assert_eq!(Version::FIRST.next().to_string(), "0002");
    }

    #[test]
    fn checksum_is_stable_and_separates_up_from_down() {
        assert_eq!(checksum("a", "b"), checksum("a", "b"));
        assert_ne!(checksum("a", "b"), checksum("ab", ""));
        assert_ne!(checksum("a", "b"), checksum("b", "a"));
        assert_eq!(checksum("a", "b").len(), 64);
    }

    #[test]
    fn scan_orders_numerically_and_ignores_other_entries() {
        let tmp = TempDir::new("scan");

        for name in ["0010_c", "0002_b", "0001_a", "notes", "x_y"] {
            fs::create_dir(tmp.0.join(name)).unwrap();
        }
        fs::write(tmp.0.join("0003_file"), "").unwrap();

        let names: Vec<_> = MigrationDir::new(&tmp.0)
            .scan()
            .unwrap()
            .into_iter()
            .map(|m| m.name)
            .collect();

        assert_eq!(names, ["0001_a", "0002_b", "0010_c"]);
    }

    #[test]
    fn scan_rejects_duplicate_versions() {
        let tmp = TempDir::new("duplicate");

        fs::create_dir(tmp.0.join("0001_a")).unwrap();
        fs::create_dir(tmp.0.join("1_b")).unwrap();

        let err = MigrationDir::new(&tmp.0).scan().unwrap_err();

        assert!(err.to_string().contains("share the same version"));
    }

    #[test]
    fn write_then_read_round_trips_and_never_overwrites() {
        let tmp = TempDir::new("write");
        let dir = MigrationDir::new(&tmp.0);
        let diff = SchemaDiff {
            up: "UP".into(),
            down: "DOWN".into(),
        };

        let created = dir.write(Version::FIRST, "", &diff, "SCHEMA").unwrap();
        assert_eq!(created, "0001_schema_change");

        let entries = dir.scan().unwrap();
        assert_eq!(entries[0].name, "0001_schema_change");
        assert_eq!(entries[0].read_up().unwrap(), "UP");
        assert_eq!(entries[0].read_down().unwrap(), "DOWN");
        assert_eq!(dir.previous_schema(entries.last()).unwrap(), "SCHEMA");

        assert!(dir.write(Version::FIRST, "", &diff, "SCHEMA").is_err());
    }

    #[test]
    fn write_normalizes_the_name() {
        let tmp = TempDir::new("named");
        let dir = MigrationDir::new(&tmp.0);
        let diff = SchemaDiff {
            up: String::new(),
            down: String::new(),
        };

        let created = dir
            .write(Version::FIRST, "First migration!", &diff, "")
            .unwrap();
        assert_eq!(created, "0001_first_migration");

        let created = dir.write(Version::FIRST.next(), "!!!", &diff, "").unwrap();
        assert_eq!(created, "0002_schema_change");

        let names: Vec<_> = dir.scan().unwrap().into_iter().map(|m| m.name).collect();
        assert_eq!(names, ["0001_first_migration", "0002_schema_change"]);
    }

    #[test]
    fn slug_normalizes_text() {
        assert_eq!(slug("first migration"), "first_migration");
        assert_eq!(slug("  Add  User--Table!! "), "add_user_table");
        assert_eq!(slug("already_snake_case"), "already_snake_case");
        assert_eq!(slug("v2 schema"), "v2_schema");
        assert_eq!(slug("../etc/passwd"), "etc_passwd");
        assert_eq!(slug("???"), "");
    }

    #[test]
    fn previous_schema_falls_back_to_legacy_file_then_empty() {
        let tmp = TempDir::new("legacy");
        let dir = MigrationDir::new(&tmp.0);

        assert_eq!(dir.previous_schema(None).unwrap(), "");

        fs::write(tmp.0.join("schema.sql"), "LEGACY").unwrap();
        assert_eq!(dir.previous_schema(None).unwrap(), "LEGACY");
    }

    #[test]
    fn missing_file_error_names_the_path() {
        let tmp = TempDir::new("missing");
        fs::create_dir(tmp.0.join("0001_a")).unwrap();

        let entries = MigrationDir::new(&tmp.0).scan().unwrap();
        let err = entries[0].read_up().unwrap_err();

        assert!(err.to_string().contains("0001_a"));
        assert!(err.to_string().contains("up.sql"));
    }
}
