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

/// The footprint's scheming version.
///
/// Whenever reading/writing the footprint changes, this is bumped to a new version.
/// The footprint tables are dropped and rebuilt when the running version and the recorded version
/// differ.
pub(crate) const FOOTPRINT_VERSION: u32 = 1;

/// An optional schema prefix with a `Display` impl.
#[derive(Debug, Clone, Copy)]
pub struct SchemaPrefix<'a>(pub Option<&'a str>);

/// An optional powned schema prefix with a `Display` impl.
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

/// The specification to find a shape ID in the database.
#[derive(Debug, Clone, Copy)]
pub struct ShapeRecord<'a> {
    /// The table the shape IDs are recorded in.
    pub table: &'a str,
    /// The column a shape ID is held in.
    pub column: &'a str,
    /// The primary key value to match in `WHERE id = {id}`.
    pub id: &'a str,
}

/// The record holding a storage's footprint version, keyed by `id`.
///
/// Giving each storage a different `id` lets one schema hold both footprints.
pub const fn footprint_record(id: &str) -> ShapeRecord<'_> {
    ShapeRecord {
        table: "_footprint_version",
        column: "version",
        id,
    }
}

/// Creates the table `record` points at.
pub fn init_shape_record_query(record: ShapeRecord<'_>, schema_prefix: SchemaPrefix<'_>) -> String {
    let ShapeRecord { table, column, .. } = record;

    formatdoc! {"
        CREATE TABLE IF NOT EXISTS {schema_prefix}{table} (
            id TEXT PRIMARY KEY,
            {column} TEXT NOT NULL
        );"
    }
}

/// The SQL condition for any of `tables` existing.
fn tables_present(tables: &[&str], schema_prefix: SchemaPrefix<'_>) -> String {
    tables
        .iter()
        .map(|table| format!("to_regclass('{schema_prefix}{table}') IS NOT NULL"))
        .collect::<Vec<_>>()
        .join(" OR ")
}

/// The query for the shape ID `record` points to and whether any of `tables` exists.
///
/// Returns one row, with the columns `shape_id` and `present`.
pub fn shape_query(
    record: ShapeRecord<'_>,
    tables: &[&str],
    schema_prefix: SchemaPrefix<'_>,
) -> String {
    let ShapeRecord { table, column, id } = record;
    let present = tables_present(tables, schema_prefix);

    formatdoc! {"
        SELECT
            (SELECT {column} FROM {schema_prefix}{table} WHERE id = '{id}') AS shape_id,
            ({present}) AS present;"
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
    /// The decision is deferred to the [`build_tables`] statement.
    Defer,
}

impl ShapeAction {
    /// The action to take, given what the database holds.
    ///
    /// A table that exists without a matching shape ID is rebuilt: nothing records its shape.
    pub fn new(recorded: Option<&str>, present: bool, shape_id: &str) -> Self {
        match (recorded, present) {
            (_, false) => Self::Build,
            (Some(recorded), _) if recorded == shape_id => Self::Keep,
            _ => Self::Rebuild,
        }
    }

    /// [`ShapeAction::new`] over a [`shape_query`] row.
    pub fn from_row(row: Option<&Row>, shape_id: &str) -> Self {
        row.map_or(Self::Build, |row| {
            Self::new(row.get("shape_id"), row.get("present"), shape_id)
        })
    }
}

impl TryFrom<ShapeAction> for TableShape {
    /// The deferred action, which has no shape until the query runs.
    type Error = ShapeAction;

    fn try_from(action: ShapeAction) -> Result<Self, Self::Error> {
        match action {
            ShapeAction::Rebuild => Ok(Self::STALE),
            ShapeAction::Keep | ShapeAction::Build => Ok(Self::CURRENT),
            ShapeAction::Defer => Err(action),
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
/// - [`Defer`](`ShapeAction::Defer`): Chooses between the above in SQL, as [`ShapeAction::new`]
///   does in Rust.
pub fn build_tables(
    record: ShapeRecord<'_>,
    tables: &[&str],
    shape_id: &str,
    schema_prefix: SchemaPrefix<'_>,
    action: ShapeAction,
    init_query: impl Display,
) -> Option<String> {
    let ShapeRecord { table, column, id } = record;

    // Cross-referencing tables should be dropped in a single statement.
    let qualified = tables
        .iter()
        .map(|table| format!("{schema_prefix}{table}"))
        .collect::<Vec<_>>()
        .join(", ");
    let drop_tables = format!("DROP TABLE IF EXISTS {qualified};");
    let present = tables_present(tables, schema_prefix);
    let record_shape = formatdoc! {"
        INSERT INTO {schema_prefix}{table} (id, {column})
        VALUES ('{id}', '{shape_id}')
        ON CONFLICT (id) DO UPDATE SET {column} = EXCLUDED.{column};"
    };

    match action {
        ShapeAction::Keep => None,
        ShapeAction::Build => Some(format!("{init_query}\n\n{record_shape}")),
        ShapeAction::Rebuild => Some(format!("{drop_tables}\n\n{init_query}\n\n{record_shape}")),
        ShapeAction::Defer => Some(formatdoc! {"
            DO $$
            DECLARE
                recorded TEXT;
            BEGIN
                SELECT {column}
                INTO recorded
                FROM {schema_prefix}{table}
                WHERE id = '{id}';

                IF recorded IS NOT DISTINCT FROM '{shape_id}' AND ({present}) THEN
                    RETURN;
                END IF;

                IF ({present}) THEN
                    {drop_tables}
                END IF;

                {init_query}

                {record_shape}
            END
            $$;"
        }),
    }
}

#[cfg(test)]
pub mod fixtures {
    use std::fmt::Display;
    use std::path::{Path, PathBuf};

    use super::FOOTPRINT_VERSION;

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

    /// Whitespace-stripped tokens of a SQL query for comparison in tests.
    pub fn query_tokens(stmt: &str) -> Vec<&str> {
        stmt.split_whitespace().collect()
    }

    /// The current version of the footprint DDL as recorded in the fixtures.
    pub fn footprint_shape(store: FootprintStore) -> String {
        let path = fixture_path(FOOTPRINT_VERSION, store);
        let shape = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Could not read {}: {e}", path.display()));
        shape.trim_end().to_owned()
    }

    fn fixture_path(version: u32, store: FootprintStore) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/footprint")
            .join(format!("v{version}"))
            .join(format!("{store}.sql"))
    }

    #[test]
    fn test_all_fixtures_present() {
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

    #[rstest]
    #[case::unbuilt(None, false, ShapeAction::Build)]
    #[case::unrecorded(None, true, ShapeAction::Rebuild)]
    #[case::matching(Some("1"), true, ShapeAction::Keep)]
    #[case::diverged(Some("2"), true, ShapeAction::Rebuild)]
    #[case::matching_but_dropped(Some("1"), false, ShapeAction::Build)]
    #[case::diverged_and_dropped(Some("2"), false, ShapeAction::Build)]
    fn test_shape_action_new(
        #[case] recorded: Option<&str>,
        #[case] present: bool,
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
            footprint_record("runs"),
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
