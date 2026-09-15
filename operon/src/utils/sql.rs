use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::num::TryFromIntError;

use indoc::formatdoc;
use postgres_types::ToSql;
use tokio_postgres::Row;
use twox_hash::XxHash3_64;

use crate::schema::{TableShape, TicketStatus};

/// The primary key for the run footprint table.
pub(crate) const GLOBAL: &str = "global";

/// The footprint tables' schema version.
///
/// Whenever reading/writing the footprint changes, this is bumped to a new version.
/// The footprint tables are dropped and rebuilt when the running version and the recorded version
/// differ.
///
/// Every version stores the run's `run_id` and `updated_at`, which a rebuild carries over.
pub(crate) const FOOTPRINT_VERSION: u32 = 1;

/// An optional schema prefix with a `Display` impl.
#[derive(Debug, Clone, Copy)]
pub struct SchemaPrefix<'a>(pub Option<&'a str>);

/// An optional owned schema prefix with a `Display` impl.
#[derive(Debug, Clone)]
pub struct SchemaPrefixOwned(pub Option<String>);

impl SchemaPrefix<'_> {
    pub fn to_owned(self) -> SchemaPrefixOwned {
        SchemaPrefixOwned(self.0.map(|s| s.to_owned()))
    }
}

impl std::fmt::Display for SchemaPrefix<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(schema) = self.0 {
            write!(f, "{schema}.")
        } else {
            Ok(())
        }
    }
}

impl std::fmt::Display for SchemaPrefixOwned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(schema) = &self.0 {
            write!(f, "{schema}.")
        } else {
            Ok(())
        }
    }
}

/// Values that can be used as sql parameter.
pub(crate) trait SqlParam: ToSql + Display + Send + Sync + 'static {
    /// Borrows the value as the trait object `tokio_postgres` takes its parameters as.
    fn as_param(&self) -> &(dyn ToSql + Sync + 'static);
}

impl SqlParam for String {
    fn as_param(&self) -> &(dyn ToSql + Sync + 'static) {
        self
    }
}
impl SqlParam for i64 {
    fn as_param(&self) -> &(dyn ToSql + Sync + 'static) {
        self
    }
}
impl SqlParam for serde_json::Value {
    fn as_param(&self) -> &(dyn ToSql + Sync + 'static) {
        self
    }
}
impl SqlParam for TicketStatus {
    fn as_param(&self) -> &(dyn ToSql + Sync + 'static) {
        self
    }
}

/// A list of sql parameters.
#[repr(transparent)]
pub(crate) struct SqlParams(Vec<Box<dyn SqlParam>>);

impl SqlParams {
    pub fn new(params: Vec<Box<dyn SqlParam>>) -> Self {
        Self(params)
    }

    pub fn from_usize(items: impl IntoIterator<Item = usize>) -> Result<Self, TryFromIntError> {
        let items = items
            .into_iter()
            .map(|item| i64::try_from(item).map(box_sql))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(items))
    }

    pub fn extend(mut self, params: Vec<Box<dyn SqlParam>>) -> Self {
        self.0.extend(params);
        self
    }

    pub fn borrow(&self) -> Vec<&(dyn ToSql + Sync + 'static)> {
        self.0.iter().map(|x| x.as_param()).collect()
    }

    pub fn to_copy_string(&self) -> String {
        let mut out = self
            .0
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",");
        out.push('\n');
        out
    }
}

pub fn box_sql<T: SqlParam>(value: T) -> Box<dyn SqlParam> {
    Box::new(value)
}

/// Renders `values` as a quoted, comma-separated SQL list.
pub fn sql_value_list<T: Display>(values: impl IntoIterator<Item = T>) -> String {
    values
        .into_iter()
        .map(|value| format!("'{value}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Compacts metadata of any length into a shape ID.
pub fn hash_metadata<T: Hash>(metadata: &T) -> String {
    let mut hasher = XxHash3_64::new();
    metadata.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// The maximum length of a PostgreSQL identifier.
const PSQL_MAX_IDENTIFIER_LENGTH: usize = 63;

/// The number of characters to take from the original ID for human readability.
const PSQL_ID_PREFIX_LENGTH: usize = 4;

/// Generates a PostgreSQL-safe identifier from a user-provided ID.
///
/// Postgres limits identifiers to 63 bytes, so this function hashes the ID to ensure it fits.
/// The output consists of:
/// - The provided `prefix`.
/// - The first 4 characters of the provided `id`.
/// - A 16-character hash of the provided `id`.
///
/// in the format `{prefix}_{id_prefix}_{hash}`.
pub(crate) fn psql_identifier(prefix: &str, id: &str) -> String {
    let id_prefix: String = id.chars().take(PSQL_ID_PREFIX_LENGTH).collect();
    let hash = hash_str(id);
    let result = format!("{prefix}_{id_prefix}_{hash}");

    debug_assert!(
        result.len() <= PSQL_MAX_IDENTIFIER_LENGTH,
        "PSQL identifier may exceed 63 bytes: {result} (length: {})",
        result.len(),
    );

    result
}

/// Hashes a string into a 16-character hex string.
fn hash_str(s: &str) -> String {
    let mut hasher = XxHash3_64::new();
    s.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// The table the shape IDs are recorded in.
#[derive(Debug, Clone, Copy)]
pub struct ShapeTable<'a> {
    /// The table the shape IDs are recorded in.
    pub table: &'a str,
    /// The column a shape ID is held in.
    pub column: &'a str,
}

impl<'a> ShapeTable<'a> {
    /// The row of this table holding `id`'s shape ID.
    pub const fn record(self, id: &'a str) -> ShapeRecord<'a> {
        ShapeRecord { table: self, id }
    }
}

/// The specification to find a shape ID in the database.
#[derive(Debug, Clone, Copy)]
pub struct ShapeRecord<'a> {
    /// The table the shape ID is recorded in.
    pub table: ShapeTable<'a>,
    /// The primary key value to match in `WHERE id = {id}`.
    pub id: &'a str,
}

/// The table holding each storage's footprint version.
///
/// Giving each storage a different [`record`](ShapeTable::record) lets one schema hold both
/// footprints.
pub const FOOTPRINT_SHAPES: ShapeTable<'static> = ShapeTable {
    table: "_footprint_version",
    column: "version",
};

/// Creates the table shape IDs are recorded in.
pub fn init_shape_table_query(
    shape_table: ShapeTable<'_>,
    schema_prefix: SchemaPrefix<'_>,
) -> String {
    let ShapeTable { table, column } = shape_table;

    formatdoc! {"
        CREATE TABLE IF NOT EXISTS {schema_prefix}{table} (
            id TEXT PRIMARY KEY,
            {column} TEXT NOT NULL
        );"
    }
}

/// The SQL conditions for each of `tables` existing, joined by `connective`.
fn tables_present(tables: &[&str], schema_prefix: SchemaPrefix<'_>, connective: &str) -> String {
    tables
        .iter()
        .map(|table| format!("to_regclass('{schema_prefix}{table}') IS NOT NULL"))
        .collect::<Vec<_>>()
        .join(connective)
}

/// The SQL condition for any of `tables` existing.
fn any_table_present(tables: &[&str], schema_prefix: SchemaPrefix<'_>) -> String {
    tables_present(tables, schema_prefix, " OR ")
}

/// The SQL condition for all of `tables` existing.
fn all_tables_present(tables: &[&str], schema_prefix: SchemaPrefix<'_>) -> String {
    tables_present(tables, schema_prefix, " AND ")
}

/// Which tables of a group exist in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TablesPresent {
    /// None of the tables exist.
    None,
    /// Some, but not all, of the tables exist.
    Partial,
    /// All of the tables exist.
    All,
}

impl TablesPresent {
    /// Reads the presence columns of a [`shape_query`] row.
    fn from_row(row: &Row) -> Self {
        let any: bool = row.get("any_present");
        let all: bool = row.get("all_present");

        match (any, all) {
            (_, true) => Self::All,
            (true, false) => Self::Partial,
            (false, false) => Self::None,
        }
    }
}

/// The query for the shape ID `record` points to and which of `tables` exist.
///
/// Returns one row, with the columns `shape_id`, `any_present`, and `all_present`.
pub fn shape_query(
    record: ShapeRecord<'_>,
    tables: &[&str],
    schema_prefix: SchemaPrefix<'_>,
) -> String {
    let ShapeRecord {
        table: ShapeTable { table, column },
        id,
    } = record;
    let any_present = any_table_present(tables, schema_prefix);
    let all_present = all_tables_present(tables, schema_prefix);

    formatdoc! {"
        SELECT
            (SELECT {column} FROM {schema_prefix}{table} WHERE id = '{id}') AS shape_id,
            ({any_present}) AS any_present,
            ({all_present}) AS all_present;"
    }
}

/// What action to take when initializing a table group via [`build_tables`].
/// Found by comparing the shape ID in the database with the current one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeAction {
    /// The tables need no changes.
    Keep,
    /// The tables can be built without dropping anything.
    Build,
    /// The tables carry a mismatching shape ID, and should be dropped before being built.
    Rebuild,
}

impl ShapeAction {
    /// The action to take, given what the database holds.
    ///
    /// A group is rebuilt when its recorded shape ID differs, or when only some of its tables
    /// exist.
    pub fn new(recorded: Option<&str>, present: TablesPresent, shape_id: &str) -> Self {
        match (recorded, present) {
            (_, TablesPresent::None) => Self::Build,
            (Some(recorded), TablesPresent::All) if recorded == shape_id => Self::Keep,
            _ => Self::Rebuild,
        }
    }

    /// [`ShapeAction::new`] over a [`shape_query`] row.
    pub fn from_row(row: Option<&Row>, shape_id: &str) -> Self {
        row.map_or(Self::Build, |row| {
            Self::new(row.get("shape_id"), TablesPresent::from_row(row), shape_id)
        })
    }
}

impl From<ShapeAction> for TableShape {
    fn from(action: ShapeAction) -> Self {
        match action {
            ShapeAction::Rebuild => Self::STALE,
            ShapeAction::Keep | ShapeAction::Build => Self::CURRENT,
        }
    }
}

/// A query that wraps `init_query` and records `shape_id` in the specified `record`.
/// Depending on `action`, it also drops outdated tables or skips redundant queries.
///
/// # Assumptions
///
/// `init_query` should perform the correct initialization (`CREATE TABLE` and other necessary
/// statements) for the tables in `tables`.
///
/// # Output query behaviour by `action`
///
/// - [`Keep`](`ShapeAction::Keep`): no-op (`None`).
/// - [`Build`](`ShapeAction::Build`): Runs `init_query` and records `shape_id`.
/// - [`Rebuild`](`ShapeAction::Rebuild`): Drops the tables in `tables`, runs `init_query`, and
///   records `shape_id`.
pub fn build_tables(
    record: ShapeRecord<'_>,
    tables: &[&str],
    shape_id: &str,
    schema_prefix: SchemaPrefix<'_>,
    action: ShapeAction,
    init_query: impl Display,
) -> Option<String> {
    let ShapeRecord {
        table: ShapeTable { table, column },
        id,
    } = record;

    // Cross-referencing tables should be dropped in a single statement.
    let qualified = tables
        .iter()
        .map(|table| format!("{schema_prefix}{table}"))
        .collect::<Vec<_>>()
        .join(", ");
    let drop_tables = format!("DROP TABLE IF EXISTS {qualified};");
    let record_shape = formatdoc! {"
        INSERT INTO {schema_prefix}{table} (id, {column})
        VALUES ('{id}', '{shape_id}')
        ON CONFLICT (id) DO UPDATE SET {column} = EXCLUDED.{column};"
    };

    match action {
        ShapeAction::Keep => None,
        ShapeAction::Build => Some(format!("{init_query}\n\n{record_shape}")),
        ShapeAction::Rebuild => Some(format!("{drop_tables}\n\n{init_query}\n\n{record_shape}")),
    }
}

#[cfg(test)]
pub mod fixtures {
    use std::fmt::Display;
    use std::path::{Path, PathBuf};

    use pretty_assertions::assert_eq;

    use super::FOOTPRINT_VERSION;

    /// The environment variable that makes a test record the current shape instead of comparing
    /// against it.
    const BLESS: &str = "BLESS_FOOTPRINT_SHAPE";

    #[derive(Debug, Clone, Copy)]
    pub enum FootprintStore {
        Data,
        Meta,
    }
    impl FootprintStore {
        const ALL: [Self; 2] = [Self::Data, Self::Meta];
    }
    impl Display for FootprintStore {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                FootprintStore::Data => write!(f, "data"),
                FootprintStore::Meta => write!(f, "meta"),
            }
        }
    }

    /// Asserts that `stmt` matches the shape recorded for the current [`FOOTPRINT_VERSION`].
    ///
    /// Set `BLESS_FOOTPRINT_SHAPE` to record `stmt` for a [`FOOTPRINT_VERSION`] that has no
    /// fixture yet.
    /// A version that already has a fixture keeps it.
    /// Changing the shape of a released version therefore needs a version bump, since schemas
    /// built by that release still hold the old shape.
    pub fn assert_footprint_shape(store: FootprintStore, stmt: &str) {
        let path = fixture_path(FOOTPRINT_VERSION, store);
        let blessing = std::env::var_os(BLESS).is_some();

        let recorded = match std::fs::read_to_string(&path) {
            Ok(recorded) => recorded,
            Err(e) if blessing && e.kind() == std::io::ErrorKind::NotFound => {
                return record(&path, stmt);
            }
            Err(e) => panic!(
                "Could not read {}: {e}\nRun the tests with {BLESS}=1 to record it.",
                path.display()
            ),
        };
        assert_eq!(
            tokens(stmt),
            tokens(&recorded),
            "The {store} footprint no longer matches the shape recorded for \
             v{FOOTPRINT_VERSION}. Raise FOOTPRINT_VERSION to {next} and re-run the tests with \
             {BLESS}=1 to record the new shape. If v{FOOTPRINT_VERSION} has not shipped, delete \
             {} and re-run with {BLESS}=1 instead.",
            path.display(),
            next = FOOTPRINT_VERSION + 1,
        );
    }

    /// Writes `stmt` into the fixture at `path`, creating its version directory.
    fn record(path: &Path, stmt: &str) {
        let dir = path
            .parent()
            .expect("the fixture sits in a version directory");
        std::fs::create_dir_all(dir)
            .unwrap_or_else(|e| panic!("Could not create {}: {e}", dir.display()));
        std::fs::write(path, format!("{stmt}\n"))
            .unwrap_or_else(|e| panic!("Could not write {}: {e}", path.display()));
    }

    /// A SQL statement split for whitespace-insensitive comparison.
    fn tokens(stmt: &str) -> Vec<&str> {
        stmt.split_whitespace().collect()
    }

    fn fixture_path(version: u32, store: FootprintStore) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/footprint")
            .join(format!("v{version}"))
            .join(format!("{store}.sql"))
    }

    #[test]
    fn test_all_fixtures_present() {
        if std::env::var_os(BLESS).is_some() {
            return;
        }

        for version in 1..=FOOTPRINT_VERSION {
            for store in FootprintStore::ALL {
                let path = fixture_path(version, store);
                assert!(
                    path.exists(),
                    "Missing footprint fixture for v{version} {store}: {}",
                    path.display()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_psql_identifier_format() {
        // Check that the identifier has the expected format: prefix_id_prefix_hash
        let result = psql_identifier("ticket", "my_task");
        assert!(result.starts_with("ticket_my_t_"));
        assert_eq!(result.len(), "ticket_my_t_".len() + 16); // 16 hex chars for hash
    }

    #[test]
    fn test_psql_identifier_short_id() {
        // IDs shorter than 4 chars should use the full ID
        let result = psql_identifier("ticket", "ab");
        assert!(result.starts_with("ticket_ab_"));
    }

    #[test]
    fn test_psql_identifier_deterministic() {
        // Same input should produce same output
        let result1 = psql_identifier("ticket", "my_task");
        let result2 = psql_identifier("ticket", "my_task");
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_psql_identifier_uniqueness() {
        // Different IDs should produce different identifiers
        let result1 = psql_identifier("ticket", "task_a");
        let result2 = psql_identifier("ticket", "task_b");
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_psql_identifier_long_id_fits() {
        // Even very long IDs should produce identifiers within 63 bytes
        let long_id = "this_is_a_very_long_task_name_that_would_exceed_postgresql_limits";
        let result = psql_identifier("ticket", long_id);
        assert!(result.len() <= 63);
    }

    #[rstest]
    #[case::unbuilt(None, TablesPresent::None, ShapeAction::Build)]
    #[case::unrecorded(None, TablesPresent::All, ShapeAction::Rebuild)]
    #[case::matching(Some("1"), TablesPresent::All, ShapeAction::Keep)]
    #[case::diverged(Some("2"), TablesPresent::All, ShapeAction::Rebuild)]
    #[case::matching_but_dropped(Some("1"), TablesPresent::None, ShapeAction::Build)]
    #[case::diverged_and_dropped(Some("2"), TablesPresent::None, ShapeAction::Build)]
    #[case::matching_but_partial(Some("1"), TablesPresent::Partial, ShapeAction::Rebuild)]
    #[case::unrecorded_and_partial(None, TablesPresent::Partial, ShapeAction::Rebuild)]
    #[case::diverged_and_partial(Some("2"), TablesPresent::Partial, ShapeAction::Rebuild)]
    fn test_shape_action_new(
        #[case] recorded: Option<&str>,
        #[case] present: TablesPresent,
        #[case] expected: ShapeAction,
    ) {
        assert_eq!(ShapeAction::new(recorded, present, "1"), expected);
    }

    #[rstest]
    #[case::build(ShapeAction::Build, false)]
    #[case::rebuild(ShapeAction::Rebuild, true)]
    fn test_build_tables_drops_named_tables_together(
        #[case] action: ShapeAction,
        #[case] drops: bool,
    ) {
        let stmt = build_tables(
            FOOTPRINT_SHAPES.record("runs"),
            &["run_executions", "runs"],
            "1",
            SchemaPrefix(Some("test_meta")),
            action,
            "CREATE TABLE test_meta.runs ();",
        )
        .expect("a statement to run");
        assert_eq!(
            stmt.contains("DROP TABLE IF EXISTS test_meta.run_executions, test_meta.runs;"),
            drops,
            "{stmt}"
        );
    }
}
