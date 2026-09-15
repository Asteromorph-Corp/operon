use async_trait::async_trait;
use bytes::Bytes;
use futures::SinkExt;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::schema::{Entity, EntityMetadata};
use crate::storage::StorageResult;
use crate::storage::psql::client::StorageClient;
use crate::storage::psql::{PsqlStorageError, PsqlStorageResult};
use crate::utils::{
    SchemaPrefix, SchemaPrefixOwned, ShapeAction, ShapeTable, SqlParams, build_tables,
    hash_metadata, psql_identifier, shape_query,
};

/// The table recording each entity's shape ID, keyed by entity ID.
pub(crate) const ENTITY_SHAPES: ShapeTable<'static> = ShapeTable {
    table: "_entity_hash",
    column: "hash",
};

pub trait PsqlEntity: Serialize + DeserializeOwned + Send + Sync + 'static {}
impl<T> PsqlEntity for T where T: Serialize + DeserializeOwned + Send + Sync + 'static {}

/// Helper struct for building SQL queries related to entities.
pub struct EntityQueryBuilder<'a, const N: usize, T: PsqlEntity> {
    client: StorageClient<'a>,
    entity_meta: EntityMetadata<N, T>,
}

impl<'a> StorageClient<'a> {
    pub fn entity<const N: usize, T: PsqlEntity>(
        self,
        entity_meta: EntityMetadata<N, T>,
    ) -> EntityQueryBuilder<'a, N, T> {
        EntityQueryBuilder {
            client: self,
            entity_meta,
        }
    }
}

impl<const N: usize, T: PsqlEntity> EntityQueryBuilder<'_, N, T> {
    /// Initializes the entity table.
    pub async fn init(&self) -> PsqlStorageResult<()> {
        self.entity_meta.init(&self.client).await
    }

    /// Gets the entity for the given primary key.
    pub async fn get(&self, coordinate: [usize; N]) -> PsqlStorageResult<Option<T>> {
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
    pub async fn put(&self, entity: Entity<N, T>) -> PsqlStorageResult<()> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = PutEntityQuery(schema_prefix, self.entity_meta);
        let params = SqlParams::from_usize(entity.coordinate)?
            .extend(vec![Box::new(serde_json::to_value(&entity.value)?)]);
        self.client.execute_stmt(&stmt, &params.borrow()).await?;
        Ok(())
    }

    /// Gets a vector of entity from the table.
    pub async fn batch_get<const M: usize, const K: usize>(
        &self,
        coordinate: [usize; M],
        over: [&'static str; K],
    ) -> PsqlStorageResult<Vec<Entity<K, T>>> {
        const { assert!(M + K == N) }

        let schema_prefix = self.client.schema_prefix();
        let stmt = BatchGetQuery(schema_prefix, self.entity_meta, over);
        let params = SqlParams::from_usize(coordinate)?;
        let rows = self.client.query_stmt(&stmt, &params.borrow()).await?;
        let entities = rows
            .into_iter()
            .map(|r| -> PsqlStorageResult<Entity<K, T>> {
                let value = serde_json::from_value::<T>(r.get(0))?;
                let coordinate: [usize; K] = (1..=K)
                    .map(|idx| usize::try_from(r.get::<_, i64>(idx)))
                    .collect::<Result<Vec<_>, _>>()?
                    .try_into()
                    .expect("Length is always K");
                Ok(Entity { value, coordinate })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entities)
    }

    /// Puts a vector of entity into the table.
    pub async fn batch_put<const M: usize>(
        &mut self,
        entity: Entity<M, Vec<T>>,
    ) -> PsqlStorageResult<()> {
        const { assert!(M + 1 == N) }

        let schema_prefix = self.client.schema_prefix().to_owned();
        let tx = self.client.transaction().await?;

        let temp_table_stmt = BatchPutTempTableQuery(&schema_prefix, self.entity_meta).to_string();
        tx.execute(&temp_table_stmt, &[]).await?;

        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(vec![]);
        for (idx, value) in entity.value.iter().enumerate() {
            let mut record: Vec<String> = Vec::with_capacity(M + 2);
            record.extend(entity.coordinate.iter().map(|c| c.to_string()));
            record.push(idx.to_string());
            record.push(serde_json::to_value(value)?.to_string());
            writer.write_record(&record)?;
        }

        let copy_stmt = BatchPutCopyQuery(self.entity_meta).to_string();
        let sink = tx.copy_in(&copy_stmt).await?;
        let mut sink = Box::pin(sink);
        sink.send(Bytes::from(writer.into_inner()?)).await?;
        sink.close().await?;

        let insert_stmt = BatchPutInsertQuery(&schema_prefix, self.entity_meta).to_string();
        tx.execute(&insert_stmt, &[]).await?;

        tx.commit().await?;
        Ok(())
    }
}

/// Prepares one entity's table, erased of the entity's arity and type.
///
/// The generated storage implements this over every entity of a pipeline, so that its `init` walks
/// one collection.
#[async_trait]
pub trait EntityQueries: Send + Sync + 'static {
    /// Creates this entity's table, rebuilding it when the recorded shape no longer matches the
    /// entity.
    async fn init(&self, client: &StorageClient<'_>) -> StorageResult<(), PsqlStorageError>;
}

#[async_trait]
impl<const N: usize, T: Send + Sync + 'static> EntityQueries for EntityMetadata<N, T> {
    async fn init(&self, client: &StorageClient<'_>) -> PsqlStorageResult<()> {
        let schema_prefix = client.schema_prefix();
        let record = ENTITY_SHAPES.record(self.id);
        let table = psql_identifier("entity", self.id);
        let tables = [table.as_str()];
        let shape_id = hash_metadata(self);

        let shape_stmt = shape_query(record, &tables, schema_prefix);
        let row = client.query_opt(&shape_stmt, &[]).await?;
        let action = ShapeAction::from_row(row.as_ref(), &shape_id);

        if let Some(stmt) = build_tables(
            record,
            &tables,
            &shape_id,
            schema_prefix,
            action,
            InitEntityQuery(schema_prefix, *self),
        ) {
            client.batch_execute(&stmt).await?;
        }
        Ok(())
    }
}

/// A helper struct to generate SQL query for initializing a entity table.
struct InitEntityQuery<'a, const N: usize, T>(SchemaPrefix<'a>, EntityMetadata<N, T>);

impl<const N: usize, T> std::fmt::Display for InitEntityQuery<'_, N, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let dims = self.1.dims;
        let table = psql_identifier("entity", id);

        writeln!(f, "CREATE TABLE IF NOT EXISTS {schema}{table} (",)?;

        for dim in &dims {
            writeln!(f, "    {dim} BIGINT,")?;
        }
        if dims.is_empty() {
            writeln!(f, "    id BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),")?;
        }

        writeln!(f, "    value JSONB,")?;
        write!(f, "    created_at TIMESTAMP WITH TIME ZONE DEFAULT now()")?;

        if !dims.is_empty() {
            writeln!(f, ",")?;
            writeln!(f, "    PRIMARY KEY ({})", dims.join(", "))?;
        } else {
            writeln!(f)?;
        }

        write!(f, ");")
    }
}

/// A helper struct to generate SQL query for getting an entity.
struct GetEntityQuery<'a, const N: usize, T>(SchemaPrefix<'a>, EntityMetadata<N, T>);

impl<const N: usize, T> std::fmt::Display for GetEntityQuery<'_, N, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let dims = self.1.dims;
        let table = psql_identifier("entity", id);

        write!(f, "SELECT value FROM {schema}{table}")?;
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

/// A helper struct to generate SQL query for inserting an entity.
struct PutEntityQuery<'a, const N: usize, T>(SchemaPrefix<'a>, EntityMetadata<N, T>);

impl<const N: usize, T> std::fmt::Display for PutEntityQuery<'_, N, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let dims = self.1.dims;
        let table = psql_identifier("entity", id);

        write!(f, "INSERT INTO {schema}{table} (")?;
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

/// A helper struct to generate SQL query for creating temp tables for batch get.
struct BatchGetQuery<'a, const N: usize, const M: usize, T>(
    SchemaPrefix<'a>,
    EntityMetadata<N, T>,
    [&'static str; M],
);

impl<const N: usize, const M: usize, T> std::fmt::Display for BatchGetQuery<'_, N, M, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let dims = self.1.dims;
        let over_dims = self.2;
        let table = psql_identifier("entity", id);

        write!(f, "SELECT value")?;
        for over_dim in over_dims {
            write!(f, ", {over_dim}")?;
        }
        writeln!(f)?;

        writeln!(f, "FROM {schema}{table}")?;

        for (idx, dim) in dims
            .iter()
            .filter(|dim| !over_dims.contains(dim))
            .enumerate()
        {
            if idx == 0 {
                write!(f, "WHERE")?;
            } else {
                write!(f, " AND")?;
            }
            write!(f, " {} = ${}", dim, idx + 1)?;
        }
        writeln!(f)?;

        for (idx, dim) in over_dims.iter().enumerate() {
            if idx == 0 {
                write!(f, "ORDER BY {dim}")?;
            } else {
                write!(f, ", {dim}")?;
            }
        }
        Ok(())
    }
}

/// A helper struct to generate SQL query for creating temp tables for batch insertion.
struct BatchPutTempTableQuery<'a, const N: usize, T>(&'a SchemaPrefixOwned, EntityMetadata<N, T>);

impl<const N: usize, T> std::fmt::Display for BatchPutTempTableQuery<'_, N, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let table = psql_identifier("entity", id);

        writeln!(f, "CREATE TEMP TABLE temp (")?;
        writeln!(f, "    LIKE {schema}{table} INCLUDING ALL")?;
        writeln!(f, ")")?;
        write!(f, "ON COMMIT DROP;")
    }
}

/// A helper struct to generate SQL query for inserting data into temp tables for batch insertion.
struct BatchPutCopyQuery<const N: usize, T>(EntityMetadata<N, T>);

impl<const N: usize, T> std::fmt::Display for BatchPutCopyQuery<N, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dims = self.0.dims;

        write!(f, "COPY temp (")?;
        for dim in dims {
            write!(f, "{dim}, ")?;
        }
        write!(f, "value) FROM STDIN WITH (FORMAT csv);")
    }
}

/// A helper struct to generate SQL query for inserting data from temp table to main table for batch
/// insertion.
struct BatchPutInsertQuery<'a, const N: usize, T>(&'a SchemaPrefixOwned, EntityMetadata<N, T>);

impl<const N: usize, T> std::fmt::Display for BatchPutInsertQuery<'_, N, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let dims = self.1.dims;
        let table = psql_identifier("entity", id);

        write!(f, "INSERT INTO {schema}{table} (")?;
        for dim in dims {
            write!(f, "{dim}, ")?;
        }
        writeln!(f, "value)")?;

        write!(f, "SELECT ")?;
        for dim in dims {
            write!(f, "{dim}, ")?;
        }
        writeln!(f, "value FROM temp")?;
        write!(f, "ON CONFLICT (")?;
        write!(f, "{}", dims.join(", "))?;
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
                created_at TIMESTAMP WITH TIME ZONE DEFAULT now()
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

    #[rstest]
    #[case::simple(
        entity_b(),
        indoc! {"
            CREATE TEMP TABLE temp (
                LIKE test_meta.b INCLUDING ALL
            )
            ON COMMIT DROP;"
        }
    )]
    fn test_batch_put_temp_table_query<const N: usize>(
        schema_prefix: SchemaPrefix<'_>,
        #[case] metadata: EntityMetadata<N, ()>,
        #[case] expected: &str,
    ) {
        let stmt = BatchPutTempTableQuery(&schema_prefix.to_owned(), metadata).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(entity_b(), "COPY temp (i, value) FROM STDIN WITH (FORMAT csv);")]
    fn test_batch_put_copy_query<const N: usize>(
        #[case] metadata: EntityMetadata<N, ()>,
        #[case] expected: &str,
    ) {
        let stmt = BatchPutCopyQuery(metadata).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(
        entity_b(),
        indoc! {"
            INSERT INTO test_meta.b (i, value)
            SELECT i, value FROM temp
            ON CONFLICT (i) DO UPDATE SET value = EXCLUDED.value;"
        }
    )]
    fn test_batch_put_insert_query<const N: usize>(
        schema_prefix: SchemaPrefix<'_>,
        #[case] metadata: EntityMetadata<N, ()>,
        #[case] expected: &str,
    ) {
        let query = BatchPutInsertQuery(&schema_prefix.to_owned(), metadata).to_string();
        assert_eq!(query, expected);
    }
}
