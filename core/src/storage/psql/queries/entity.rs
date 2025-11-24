use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::schema::{Entity, EntityMetadata};
use crate::storage::StorageError;
use crate::storage::psql::StorageClient;
use crate::utils::{SchemaPrefix, SqlParams};

pub trait PsqlEntity: Serialize + DeserializeOwned + Send + Sync + 'static {}
impl<T> PsqlEntity for T where T: Serialize + DeserializeOwned + Send + Sync + 'static {}

/// Helper struct for building SQL queries related to entities.
pub struct EntityQueryBuilder<'a, const N: usize, T: PsqlEntity> {
    client: &'a StorageClient<'a>,
    entity_meta: EntityMetadata<N, T>,
}

impl<'a> StorageClient<'a> {
    pub fn entity<const N: usize, T: PsqlEntity>(
        &self,
        entity_meta: EntityMetadata<N, T>,
    ) -> EntityQueryBuilder<'_, N, T> {
        EntityQueryBuilder {
            client: self,
            entity_meta,
        }
    }
}

impl<const N: usize, T: PsqlEntity> EntityQueryBuilder<'_, N, T> {
    /// Initializes the entity table.
    pub async fn init(&self) -> Result<(), StorageError> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = self.entity_meta.init_stmt(schema_prefix);
        self.client.execute(&stmt, &[]).await?;
        Ok(())
    }

    /// Clears the entity table.
    pub async fn clear(&self) -> Result<(), StorageError> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = self.entity_meta.clear_stmt(schema_prefix);
        self.client.execute_stmt(&stmt, &[]).await?;
        Ok(())
    }

    /// Gets the entity for the given primary key.
    pub async fn get(&self, coordinate: [usize; N]) -> Result<Option<T>, StorageError> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = GetEntityQuery(schema_prefix, self.entity_meta);
        let params = SqlParams::from_usize(coordinate)?;
        let Some(row) = self.client.query_opt_stmt(&stmt, &params.borrow()).await? else {
            return Ok(None);
        };
        let value = serde_json::from_value::<T>(row.get(0))?;
        Ok(Some(value))
    }

    /// Puts an entity into the table.
    pub async fn put(&self, entity: Entity<N, T>) -> Result<(), StorageError> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = PutEntityQuery(schema_prefix, self.entity_meta);
        let params = SqlParams::from_usize(entity.coordinate)?
            .extend(vec![Box::new(serde_json::to_value(&entity.value)?)]);
        self.client.execute_stmt(&stmt, &params.borrow()).await?;
        Ok(())
    }
}

pub trait EntityQueries: Send + Sync + 'static {
    fn init_stmt(&self, schema: SchemaPrefix<'_>) -> String;
    fn clear_stmt(&self, schema: SchemaPrefix<'_>) -> String;
}

impl<const N: usize, T: Send + Sync + 'static> EntityQueries for EntityMetadata<N, T> {
    fn init_stmt(&self, schema: SchemaPrefix<'_>) -> String {
        InitEntityQuery(schema, *self).to_string()
    }

    fn clear_stmt(&self, schema: SchemaPrefix<'_>) -> String {
        ClearEntityQuery(schema, *self).to_string()
    }
}

/// A helper struct to generate SQL query for initializing a entity table.
struct InitEntityQuery<'a, const N: usize, T>(SchemaPrefix<'a>, EntityMetadata<N, T>);

impl<const N: usize, T> std::fmt::Display for InitEntityQuery<'_, N, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let dims = self.1.dims;

        writeln!(f, "CREATE TABLE IF NOT EXISTS {schema}{id} (",)?;

        for dim in &dims {
            writeln!(f, "    {dim} BIGINT,")?;
        }
        if dims.is_empty() {
            writeln!(f, "    id BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),")?;
        }

        writeln!(f, "    value JSONB,")?;
        writeln!(f, "    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),")?;

        if !dims.is_empty() {
            writeln!(f, "    PRIMARY KEY ({})", dims.join(", "))?;
        }

        write!(f, ");")
    }
}

/// A helper struct to generate SQL query for clearing a entity table.
struct ClearEntityQuery<'a, const N: usize, T>(SchemaPrefix<'a>, EntityMetadata<N, T>);

impl<const N: usize, T> std::fmt::Display for ClearEntityQuery<'_, N, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;

        write!(f, "TRUNCATE TABLE {schema}{id};")
    }
}

struct GetEntityQuery<'a, const N: usize, T>(SchemaPrefix<'a>, EntityMetadata<N, T>);

impl<const N: usize, T> std::fmt::Display for GetEntityQuery<'_, N, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let dims = self.1.dims;

        write!(f, "SELECT value FROM {schema}{id}")?;
        for (idx, dim) in dims.iter().enumerate() {
            if idx == 0 {
                write!(f, " WHERE")?;
            } else {
                write!(f, " AND")?;
            }

            write!(f, " {} = ${}", dim, idx + 1)?;
        }
        write!(f, ";")
    }
}

struct PutEntityQuery<'a, const N: usize, T>(SchemaPrefix<'a>, EntityMetadata<N, T>);

impl<const N: usize, T> std::fmt::Display for PutEntityQuery<'_, N, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let dims = self.1.dims;

        write!(f, "INSERT INTO {schema}{id} (")?;
        for dim in dims {
            write!(f, "{dim}, ")?;
        }
        writeln!(f, "value)")?;

        write!(f, "VALUES (")?;
        for idx in 0..=dims.len() {
            if idx != 0 {
                write!(f, ", ")?;
            }
            write!(f, "${}", idx + 1)?;
        }
        writeln!(f, ")")?;

        write!(f, "ON CONFLICT (")?;
        if !dims.is_empty() {
            write!(f, "{}", dims.join(", "))?;
        } else {
            write!(f, "id")?;
        }
        write!(f, ") DO UPDATE SET value = EXCLUDED.value;")
    }
}

#[cfg(test)]
mod test {
    use indoc::indoc;
    use rstest::{fixture, rstest};

    use super::*;

    #[fixture]
    fn schema_prefix() -> SchemaPrefix<'static> {
        SchemaPrefix(Some("test_meta"))
    }

    fn entity_a() -> EntityMetadata<0, ()> {
        EntityMetadata {
            id: "a",
            dims: [],
            _phantom: std::marker::PhantomData,
        }
    }

    fn entity_b() -> EntityMetadata<1, ()> {
        EntityMetadata {
            id: "b",
            dims: ["i"],
            _phantom: std::marker::PhantomData,
        }
    }

    #[rstest]
    #[case(
        entity_a(),
        indoc! {"
            CREATE TABLE IF NOT EXISTS test_meta.a (
                id BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
                value JSONB,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
            );"
        }
    )]
    #[case(
        entity_b(),
        indoc! {"
            CREATE TABLE IF NOT EXISTS test_meta.b (
                i BIGINT,
                value JSONB,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
                PRIMARY KEY (i)
            );"
        }
    )]
    fn test_init_entity_query<const N: usize>(
        schema_prefix: SchemaPrefix<'_>,
        #[case] metadata: EntityMetadata<N, ()>,
        #[case] expected: &str,
    ) {
        let stmt = InitEntityQuery(schema_prefix, metadata).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case(entity_a(), "TRUNCATE TABLE test_meta.a;")]
    fn test_clear_entity_query<const N: usize>(
        schema_prefix: SchemaPrefix<'_>,
        #[case] metadata: EntityMetadata<N, ()>,
        #[case] expected: &str,
    ) {
        let stmt = ClearEntityQuery(schema_prefix, metadata).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case(entity_a(), "SELECT value FROM test_meta.a;")]
    #[case(entity_b(), "SELECT value FROM test_meta.b WHERE i = $1;")]
    fn test_get_entity_query<const N: usize>(
        schema_prefix: SchemaPrefix<'_>,
        #[case] metadata: EntityMetadata<N, ()>,
        #[case] expected: &str,
    ) {
        let stmt = GetEntityQuery(schema_prefix, metadata).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case(entity_a(), indoc! {"
        INSERT INTO test_meta.a (value)
        VALUES ($1)
        ON CONFLICT (id) DO UPDATE SET value = EXCLUDED.value;"
    })]
    #[case(entity_b(), indoc! {"
        INSERT INTO test_meta.b (i, value)
        VALUES ($1, $2)
        ON CONFLICT (i) DO UPDATE SET value = EXCLUDED.value;"
    })]
    fn test_put_entity_query<const N: usize>(
        schema_prefix: SchemaPrefix<'_>,
        #[case] metadata: EntityMetadata<N, ()>,
        #[case] expected: &str,
    ) {
        let stmt = PutEntityQuery(schema_prefix, metadata).to_string();
        assert_eq!(stmt, expected);
    }
}
