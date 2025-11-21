use crate::meta_storage::{MetaClient, MetaStorageError};
use crate::schema_base::{DimensionMetadata, Resolution};
use crate::utils::{SchemaPrefix, SqlParams};

/// Helper struct for building SQL queries related to resolutions.
pub struct ResolutionQueryBuilder<'a, const N: usize> {
    client: &'a MetaClient<'a>,
    dim_meta: DimensionMetadata<N>,
}

impl<'a> MetaClient<'a> {
    /// Helper method to create a `ResolutionQueryBuilder` for a resolution of given dimension.
    pub fn resolution<const N: usize>(
        &'a self,
        dim_meta: DimensionMetadata<N>,
    ) -> ResolutionQueryBuilder<'a, N> {
        ResolutionQueryBuilder {
            client: self,
            dim_meta,
        }
    }
}

impl<const N: usize> ResolutionQueryBuilder<'_, N> {
    /// Initializes the resolution table.
    pub async fn init(&self) -> Result<(), MetaStorageError> {
        let schema = self.client.schema_prefix();
        let stmt = InitResolutionQuery(schema, self.dim_meta);
        self.client.execute_stmt(&stmt, &[]).await?;
        Ok(())
    }

    /// Clears the resolution table.
    pub async fn clear(&self) -> Result<(), MetaStorageError> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = ClearResolutionQuery(schema_prefix, self.dim_meta);
        self.client.execute_stmt(&stmt, &[]).await?;
        Ok(())
    }

    /// Gets the resolution for the given primary key.
    pub async fn get(
        &self,
        coordinate: [usize; N],
    ) -> Result<Option<Resolution<N>>, MetaStorageError> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = GetResolutionQuery(schema_prefix, self.dim_meta);
        let params = SqlParams::from_usize(coordinate)?;
        let Some(row) = self.client.query_opt_stmt(&stmt, &params.borrow()).await? else {
            return Ok(None);
        };
        let ub = usize::try_from(row.get::<_, i64>(&"ub"))?;
        Ok(Some(Resolution { coordinate, ub }))
    }

    /// Puts the resolution into the table.
    pub async fn put(&self, resolution: Resolution<N>) -> Result<(), MetaStorageError> {
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

        writeln!(f, "CREATE TABLE IF NOT EXISTS {schema}dimension_{id} (")?;
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

/// Helper struct to generate the SQL query for getting a dimension's resolution.
struct GetResolutionQuery<'a, const N: usize>(SchemaPrefix<'a>, DimensionMetadata<N>);

impl<const N: usize> std::fmt::Display for GetResolutionQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let deps = self.1.deps;

        write!(f, "SELECT ub FROM {schema}dimension_{id}")?;

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

/// Helper struct to generate the SQL query for inserting a resolution into a dimension.
pub struct PutResolutionQuery<'a, const N: usize>(SchemaPrefix<'a>, DimensionMetadata<N>);

impl<const N: usize> std::fmt::Display for PutResolutionQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let deps = self.1.deps;

        write!(f, "INSERT INTO {schema}dimension_{id} (")?;
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

/// Helper struct to generate the SQL query for clearing a dimension's resolution.
struct ClearResolutionQuery<'a, const N: usize>(SchemaPrefix<'a>, DimensionMetadata<N>);

impl<const N: usize> std::fmt::Display for ClearResolutionQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;

        write!(f, "TRUNCATE TABLE {schema}dimension_{id};")
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
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
            CREATE TABLE IF NOT EXISTS test_meta.dimension_i (
                ub BIGINT NOT NULL
            );"
        },
    )]
    #[case::with_dependency(
        dimension_j(),
        indoc! {"
            CREATE TABLE IF NOT EXISTS test_meta.dimension_j (
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
    #[case::simple(dimension_i(), "SELECT ub FROM test_meta.dimension_i;")]
    #[case::with_dependency(dimension_j(), "SELECT ub FROM test_meta.dimension_j WHERE i = $1;")]
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
        "INSERT INTO test_meta.dimension_i (ub) VALUES ($1) ON CONFLICT DO NOTHING;"
    )]
    #[case::with_dependency(
        dimension_j(),
        "INSERT INTO test_meta.dimension_j (i, ub) VALUES ($1, $2) ON CONFLICT DO NOTHING;"
    )]
    fn test_put_resolution_query_no_dependency<const N: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] metadata: DimensionMetadata<N>,
        #[case] expected: &str,
    ) {
        let stmt = PutResolutionQuery(schema_prefix, metadata).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(dimension_i(), "TRUNCATE TABLE test_meta.dimension_i;")]
    fn test_clear_resolution_query<const N: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] metadata: DimensionMetadata<N>,
        #[case] expected: &str,
    ) {
        let stmt = ClearResolutionQuery(schema_prefix, metadata).to_string();
        assert_eq!(stmt, expected);
    }
}
