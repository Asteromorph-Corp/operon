use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::num::TryFromIntError;

use postgres_types::ToSql;
use twox_hash::XxHash3_64;

use crate::schema::{TableShape, TicketStatus};

/// The primary key for the run footprint table.
pub(crate) const GLOBAL: &str = "global";

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

fn hash_metadata<T: Hash>(metadata: &T) -> String {
    let mut hasher = XxHash3_64::new();
    metadata.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// The statement reading the hash the table keyed to `id` was last built under.
pub fn recorded_hash_query(
    id: &str,
    schema_prefix: SchemaPrefix<'_>,
    hash_table: &'static str,
) -> String {
    format!("SELECT hash FROM {schema_prefix}{hash_table} WHERE id = '{id}';")
}

/// The shape of the table `recorded` was recorded for, against the current `metadata`.
///
/// [`STALE`](TableShape::STALE) is the condition [`replace_if_updated`] rebuilds a table under.
pub fn table_shape<T: Hash>(recorded: Option<&str>, metadata: &T) -> TableShape {
    match recorded {
        Some(hash) if hash != hash_metadata(metadata) => TableShape::STALE,
        _ => TableShape::CURRENT,
    }
}

/// Wraps `init_query` in a guard that runs it only when `tables` do not already carry the current
/// `metadata` hash, dropping them together first when they carry another one.
/// Also sets the database-side hash to the current metadata hash.
pub fn replace_if_updated<T: Hash>(
    id: &str,
    tables: &[&str],
    metadata: &T,
    schema_prefix: SchemaPrefix<'_>,
    hash_table: &'static str,
    init_query: impl Display,
) -> String {
    let hash = hash_metadata(metadata);

    // Cross-referencing tables should be dropped in a single statement.
    let qualified = tables
        .iter()
        .map(|table| format!("{schema_prefix}{table}"))
        .collect::<Vec<_>>()
        .join(", ");

    format! { r#"
        DO $$
        DECLARE
            existing_hash TEXT;
        BEGIN
            SELECT hash
            INTO existing_hash
            FROM {schema_prefix}{hash_table}
            WHERE id = '{id}';

            IF existing_hash IS NOT DISTINCT FROM '{hash}' THEN
                RETURN;
            END IF;

            IF existing_hash IS NOT NULL THEN
                DROP TABLE IF EXISTS {qualified};
            END IF;

            {init_query}

            INSERT INTO {schema_prefix}{hash_table} (id, hash)
            VALUES ('{id}', '{hash}')
            ON CONFLICT (id) DO UPDATE SET hash = EXCLUDED.hash;
        END
        $$;"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_if_updated_drops_named_tables_together() {
        let stmt = replace_if_updated(
            "runs",
            &["run_executions", "runs"],
            &1u32,
            SchemaPrefix(Some("test_meta")),
            "_footprint_hash",
            "CREATE TABLE test_meta.runs ();",
        );
        assert!(
            stmt.contains("DROP TABLE IF EXISTS test_meta.run_executions, test_meta.runs;"),
            "{stmt}"
        );
    }
}
