use std::num::TryFromIntError;

use crate::meta_storage::MetaResolutionApi;
use crate::meta_storage::psql::error::PsqlResult;
use crate::meta_storage::psql::{PsqlClient, PsqlMetaError};
use crate::schema::{DimensionMetadata, Resolution, TableShape};
use crate::utils::{
    SchemaPrefix, ShapeAction, ShapeTable, SqlParams, build_tables, hash_metadata, psql_identifier,
    shape_query,
};

/// The table recording each dimension's shape ID, keyed by dimension ID.
pub(crate) const DIMENSION_SHAPES: ShapeTable<'static> = ShapeTable {
    table: "_dimension_hash",
    column: "hash",
};

/// Postgres wire serialization for [`Resolution`], alongside the query builders that use it.
impl<const N: usize> Resolution<N> {
    /// Serializes a resolution into the ordered parameter list expected by the resolution table.
    fn as_sql_params(&self) -> Result<SqlParams, TryFromIntError> {
        SqlParams::from_usize(self.coordinate.into_iter().chain([self.ub]))
    }
}

/// Helper struct for building SQL queries related to resolutions.
pub struct PsqlResolutionQueryBuilder<'a, const N: usize> {
    client: &'a PsqlClient<'a>,
    dim_meta: DimensionMetadata<N>,
}

impl<'a> PsqlClient<'a> {
    /// Helper method to create a `PsqlResolutionQueryBuilder` for a resolution of given dimension.
    pub fn resolution<const N: usize>(
        &'a self,
        dim_meta: DimensionMetadata<N>,
    ) -> PsqlResolutionQueryBuilder<'a, N> {
        PsqlResolutionQueryBuilder {
            client: self,
            dim_meta,
        }
    }
}

impl<const N: usize> MetaResolutionApi<N> for PsqlResolutionQueryBuilder<'_, N> {
    type Error = PsqlMetaError;

    async fn init(&self) -> PsqlResult<TableShape> {
        let schema_prefix = self.client.schema_prefix();
        let id = self.dim_meta.id;
        let shape_record = DIMENSION_SHAPES.record(id);
        let shape_id = hash_metadata(&self.dim_meta);
        let dimension_table = psql_identifier("dimension", id);
        let tables = [dimension_table.as_str()];

        let shape_stmt = shape_query(shape_record, &tables, schema_prefix);
        let row = self.client.query_opt(&shape_stmt, &[]).await?;
        let action = ShapeAction::from_row(row.as_ref(), &shape_id);

        if let Some(stmt) = build_tables(
            shape_record,
            &tables,
            &shape_id,
            schema_prefix,
            action,
            InitResolutionQuery(schema_prefix, self.dim_meta),
        ) {
            self.client.batch_execute(&stmt).await?;
        }
        Ok(action.into())
    }

    async fn clear(&self) -> PsqlResult<()> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = ClearResolutionQuery(schema_prefix, self.dim_meta);
        self.client.execute_stmt(&stmt, &[]).await?;
        Ok(())
    }

    async fn get(&self, coordinate: [usize; N]) -> PsqlResult<Option<Resolution<N>>> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = GetResolutionQuery(schema_prefix, self.dim_meta);
        let params = SqlParams::from_usize(coordinate)?;
        let Some(row) = self.client.query_opt_stmt(&stmt, &params.borrow()).await? else {
            return Ok(None);
        };
        let ub = usize::try_from(row.get::<_, i64>(&"ub"))?;
        Ok(Some(Resolution { coordinate, ub }))
    }

    async fn put(&self, resolution: Resolution<N>) -> PsqlResult<()> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = PutResolutionQuery(schema_prefix, self.dim_meta);
        let params = resolution.as_sql_params()?;
        self.client.execute_stmt(&stmt, &params.borrow()).await?;
        Ok(())
    }
}

/// Helper struct to generate the SQL query for initializing a dimension's resolution.
struct InitResolutionQuery<'a, const N: usize>(SchemaPrefix<'a>, DimensionMetadata<N>);

impl<const N: usize> std::fmt::Display for InitResolutionQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let deps = self.1.deps;
        let table = psql_identifier("dimension", id);

        writeln!(f, "CREATE TABLE IF NOT EXISTS {schema}{table} (")?;
        for dep in deps {
            writeln!(f, "    {dep} BIGINT,")?;
        }
        write!(f, "    ub BIGINT NOT NULL")?;

        if deps.is_empty() {
            writeln!(f)?;
        } else {
            writeln!(f, ",")?;
            writeln!(f, "    PRIMARY KEY ({})", deps.join(","))?;
        }
        write!(f, ");")
    }
}

/// Helper struct to generate the SQL query for clearing a dimension's resolution.
struct ClearResolutionQuery<'a, const N: usize>(SchemaPrefix<'a>, DimensionMetadata<N>);

impl<const N: usize> std::fmt::Display for ClearResolutionQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let table = psql_identifier("dimension", id);

        write!(f, "TRUNCATE TABLE {schema}{table};")
    }
}

/// Helper struct to generate the SQL query for getting a dimension's resolution.
struct GetResolutionQuery<'a, const N: usize>(SchemaPrefix<'a>, DimensionMetadata<N>);

impl<const N: usize> std::fmt::Display for GetResolutionQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let deps = self.1.deps;
        let table = psql_identifier("dimension", id);

        write!(f, "SELECT ub FROM {schema}{table}")?;

        for (idx, dim) in deps.iter().enumerate() {
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

/// Helper struct to generate the SQL query for inserting a resolution for a dimension.
pub struct PutResolutionQuery<'a, const N: usize>(SchemaPrefix<'a>, DimensionMetadata<N>);

impl<const N: usize> std::fmt::Display for PutResolutionQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let deps = self.1.deps;
        let table = psql_identifier("dimension", id);

        write!(f, "INSERT INTO {schema}{table} (")?;
        for dep in deps {
            write!(f, "{dep}, ")?;
        }
        write!(f, "ub) VALUES ($1")?;
        for i in 1..(deps.len() + 1) {
            write!(f, ", ${}", i + 1)?;
        }
        write!(f, ") ON CONFLICT DO NOTHING;")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};

    use super::*;

    fn dimension_i() -> DimensionMetadata<0> {
        DimensionMetadata { id: "i", deps: [] }
    }

    fn dimension_j() -> DimensionMetadata<1> {
        DimensionMetadata {
            id: "j",
            deps: ["i"],
        }
    }

    #[fixture]
    fn schema_prefix() -> SchemaPrefix<'static> {
        SchemaPrefix(Some("test_meta"))
    }

    #[rstest]
    #[case::simple(
        dimension_i(),
        indoc! {"
            CREATE TABLE IF NOT EXISTS test_meta.dimension_i_b61efb9fe0d18f80 (
                ub BIGINT NOT NULL
            );"
        },
    )]
    #[case::with_dependency(
        dimension_j(),
        indoc! {"
            CREATE TABLE IF NOT EXISTS test_meta.dimension_j_0ba8200fda5f47f7 (
                i BIGINT,
                ub BIGINT NOT NULL,
                PRIMARY KEY (i)
            );"
        }
    )]
    fn test_init_resolution_query<const N: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] metadata: DimensionMetadata<N>,
        #[case] expected: &str,
    ) {
        let stmt = InitResolutionQuery(schema_prefix, metadata).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(
        dimension_i(),
        "TRUNCATE TABLE test_meta.dimension_i_b61efb9fe0d18f80;"
    )]
    fn test_clear_resolution_query<const N: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] metadata: DimensionMetadata<N>,
        #[case] expected: &str,
    ) {
        let stmt = ClearResolutionQuery(schema_prefix, metadata).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(
        dimension_i(),
        "SELECT ub FROM test_meta.dimension_i_b61efb9fe0d18f80;"
    )]
    #[case::with_dependency(
        dimension_j(),
        "SELECT ub FROM test_meta.dimension_j_0ba8200fda5f47f7 WHERE i = $1;"
    )]
    fn test_get_resolution_query<const N: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] metadata: DimensionMetadata<N>,
        #[case] expected: &str,
    ) {
        let stmt = GetResolutionQuery(schema_prefix, metadata).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(
        dimension_i(),
        "INSERT INTO test_meta.dimension_i_b61efb9fe0d18f80 (ub) VALUES ($1) ON CONFLICT DO NOTHING;"
    )]
    #[case::with_dependency(
        dimension_j(),
        "INSERT INTO test_meta.dimension_j_0ba8200fda5f47f7 (i, ub) VALUES ($1, $2) ON CONFLICT DO NOTHING;"
    )]
    fn test_put_resolution_query_no_dependency<const N: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] metadata: DimensionMetadata<N>,
        #[case] expected: &str,
    ) {
        let stmt = PutResolutionQuery(schema_prefix, metadata).to_string();
        assert_eq!(stmt, expected);
    }
}
