use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::num::TryFromIntError;

use postgres_types::ToSql;
use twox_hash::XxHash3_64;

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

pub fn replace_if_updated<T: Hash>(
    id: &str,
    metadata: &T,
    schema_prefix: SchemaPrefix<'_>,
    hash_table: &'static str,
    init_query: impl Display,
) -> String {
    let hash = hash_metadata(metadata);

    format! { r#"
        DO $$
        DECLARE
            existing_hash TEXT;
        BEGIN
            SELECT hash
            INTO existing_hash
            FROM {schema_prefix}{hash_table}
            WHERE id = '{id}';

            IF existing_hash IS NULL THEN
                {init_query}

                INSERT INTO {schema_prefix}{hash_table} (id, hash)
                VALUES ('{id}', '{hash}');

            ELSIF existing_hash != '{hash}' THEN
                DROP TABLE IF EXISTS {schema_prefix}{id};

                {init_query}

                UPDATE {schema_prefix}{hash_table}
                SET hash = '{hash}'
                WHERE id = '{id}';
            END IF;
        END
        $$;"#
    }
}
