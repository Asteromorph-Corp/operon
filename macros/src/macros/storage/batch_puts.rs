use syn::parse_quote;

use crate::configs::{JobConfig, JobConfigMap};
use crate::utils::{batch_put_entity_ident, entity_ident, operon_ident, variable_ident};

struct BatchPutTempTableQuery<'a>(&'a JobConfig);

impl std::fmt::Display for BatchPutTempTableQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CREATE TEMP TABLE temp (LIKE {{schema_prefix}}{} INCLUDING ALL) ON COMMIT DROP;",
            self.0.to
        )
    }
}

struct BatchPutCopyQuery<'a>(&'a JobConfig);

impl std::fmt::Display for BatchPutCopyQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "COPY temp (")?;
        for dim in &self.0.dims {
            write!(f, "{dim}, ")?;
        }
        if let Some(spawn_dim) = &self.0.spawn_dim {
            write!(f, "{spawn_dim}, ")?;
        }
        write!(f, "value) FROM STDIN WITH (FORMAT csv);")
    }
}

struct BatchPutInsertQuery<'a>(&'a JobConfig);

impl std::fmt::Display for BatchPutInsertQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "INSERT INTO {{schema_prefix}}{} (", self.0.to)?;
        for dim in &self.0.dims {
            write!(f, "{dim}, ")?;
        }
        if let Some(spawn_dim) = &self.0.spawn_dim {
            write!(f, "{spawn_dim}, ")?;
        }
        writeln!(f, "value)")?;
        write!(f, "SELECT ")?;
        for dim in &self.0.dims {
            write!(f, "{dim}, ")?;
        }
        if let Some(spawn_dim) = &self.0.spawn_dim {
            write!(f, "{spawn_dim}, ")?;
        }
        writeln!(f, "value FROM temp")?;
        write!(f, "ON CONFLICT (")?;
        for (idx, dim) in self.0.dims.iter().chain(&self.0.spawn_dim).enumerate() {
            if idx != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{dim}")?;
        }
        write!(f, ") DO UPDATE SET value = EXCLUDED.value;")
    }
}

pub fn batch_puts(jobs: &JobConfigMap) -> impl Iterator<Item = syn::TraitItemFn> {
    jobs.values().filter_map(|job| -> Option<syn::TraitItemFn> {
        let operon = operon_ident();
        let spawn_dim =job.spawn_dim.as_ref()?;

        let temp_table_query = BatchPutTempTableQuery(job).to_string();
        let copy_query = BatchPutCopyQuery(job).to_string();
        let insert_query = BatchPutInsertQuery(job).to_string();

        let batch_put_fn_name = batch_put_entity_ident(&job.to);
        let coord_vars = job.dims.iter().map(|d| variable_ident(d)).collect::<Vec<_>>();
        let spawn_dim_var = variable_ident(spawn_dim);

        let n = job.dims.len();
        let t = entity_ident(&job.to);

        Some(parse_quote! {
            async fn #batch_put_fn_name(&self, entity: #operon::schema::Entity<#n, Vec<#t>>) -> Result<(), #operon::storage::StorageError> {
                let [#(#coord_vars),*] = entity.coordinate;

                let mut conn = self.pool.get().await?;
                let tx = conn.transaction().await?;
                let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());

                let temp_table_stmt = format!(#temp_table_query);
                tx.execute(&temp_table_stmt, &[]).await?;

                let mut writer = #operon::csv::WriterBuilder::new()
                    .has_headers(false)
                    .from_writer(vec![]);
                for (#spawn_dim_var, value) in entity.value.iter().enumerate() {
                    writer.serialize((#(#coord_vars,)* #spawn_dim_var, #operon::serde_json::to_value(value)?.to_string()))?;
                }
                let copy_stmt = #copy_query;
                let sink = tx.copy_in(copy_stmt).await?;
                let mut sink = Box::pin(sink);
                #operon::futures::sink::SinkExt::send(&mut sink, #operon::bytes::Bytes::from(writer.into_inner()?)).await?;
                #operon::futures::sink::SinkExt::close(&mut sink).await?;

                let insert_stmt = format!(#insert_query);
                tx.execute(&insert_stmt, &[]).await?;
                tx.commit().await?;
                Ok(())
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_items_eq_in_trait;
    use crate::test_utils::simple_pipeline::{all_jobs, job_beta};

    #[rstest]
    #[case::simple(
        job_beta(),
        "CREATE TEMP TABLE temp (LIKE {schema_prefix}b INCLUDING ALL) ON COMMIT DROP;"
    )]
    fn test_batch_put_temp_table_query(#[case] job: JobConfig, #[case] expected: &str) {
        let stmt = BatchPutTempTableQuery(&job).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(job_beta(), "COPY temp (i, j, value) FROM STDIN WITH (FORMAT csv);")]
    fn test_batch_put_copy_query(#[case] job: JobConfig, #[case] expected: &str) {
        let stmt = BatchPutCopyQuery(&job).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(
        job_beta(),
        indoc! {"
            INSERT INTO {schema_prefix}b (i, j, value)
            SELECT i, j, value FROM temp
            ON CONFLICT (i, j) DO UPDATE SET value = EXCLUDED.value;"
        }
    )]
    fn test_batch_put_insert_query(#[case] job: JobConfig, #[case] expected: &str) {
        let query = BatchPutInsertQuery(&job).to_string();
        assert_eq!(query, expected);
    }

    #[rstest]
    fn test_batch_puts(all_jobs: JobConfigMap) {
        let items = batch_puts(&all_jobs).collect::<Vec<_>>();
        assert_items_eq_in_trait(&items, "storage/batch_puts.rs");
    }
}
