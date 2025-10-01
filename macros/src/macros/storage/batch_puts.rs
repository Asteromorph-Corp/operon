use syn::parse_quote;

use crate::utils::{
    batch_put_entity_ident, dimension_ident, entity_ident, operon_ident, variable_ident,
};
use crate::{JobConfig, JobConfigMap};

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
        for (i, dim) in self.0.dims.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{dim}")?;
        }
        if let Some(spawn_dim) = &self.0.spawn_dim {
            write!(f, ", {spawn_dim}")?;
        }
        write!(f, ") DO UPDATE SET value = EXCLUDED.value;")
    }
}

pub fn batch_puts(jobs: &JobConfigMap) -> impl Iterator<Item = syn::TraitItemFn> {
    jobs.values().filter_map(|job| -> Option<syn::TraitItemFn> {
        let operon = operon_ident();
        let spawn_dim = job.spawn_dim.as_ref()?;

        let temp_table_query = BatchPutTempTableQuery(job).to_string();
        let copy_query = BatchPutCopyQuery(job).to_string();
        let insert_query = BatchPutInsertQuery(job).to_string();

        let entity_ident = entity_ident(&job.to);
        let dim_args = job.dims.iter()
            .map(|d| -> syn::FnArg {
                let arg_ident = variable_ident(d);
                let arg_ty = dimension_ident(d);
                parse_quote! { #arg_ident: schema::#arg_ty }
            });
        let dim_vars = job.dims.iter().map(|d| variable_ident(d));
        let spawn_dim_var = variable_ident(spawn_dim);
        let batch_put_fn_name = batch_put_entity_ident(&job.to);

        Some(parse_quote! {
            async fn #batch_put_fn_name(&self, #(#dim_args,)* values: Vec<#entity_ident>) -> Result<(), #operon::storage::StorageError> {
                let mut conn = self.pool.get().await?;
                let tx = conn.transaction().await?;
                let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());

                let temp_table_stmt = format!(#temp_table_query);
                tx.execute(&temp_table_stmt, &[]).await?;

                let mut writer = #operon::csv::WriterBuilder::new()
                    .has_headers(false)
                    .from_writer(vec![]);
                for (#spawn_dim_var, value) in values.iter().enumerate() {
                    writer.serialize((#(#dim_vars,)* #spawn_dim_var, #operon::serde_json::to_value(value)?.to_string()))?;
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

    use super::*;
    use crate::JobArg;

    #[test]
    fn test_batch_put_temp_table_query() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
            pool_size: 8,
        };
        let query = BatchPutTempTableQuery(&job).to_string();
        let expected =
            "CREATE TEMP TABLE temp (LIKE {schema_prefix}b INCLUDING ALL) ON COMMIT DROP;";

        assert_eq!(query, expected);
    }

    #[test]
    fn test_batch_put_copy_query() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
            pool_size: 8,
        };
        let query = BatchPutCopyQuery(&job).to_string();
        let expected = "COPY temp (i, j, value) FROM STDIN WITH (FORMAT csv);";
        assert_eq!(query, expected);
    }

    #[test]
    fn test_batch_put_insert_query() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
            pool_size: 8,
        };
        let query = BatchPutInsertQuery(&job).to_string();
        let expected = indoc! {"
            INSERT INTO {schema_prefix}b (i, j, value)
            SELECT i, j, value FROM temp
            ON CONFLICT (i, j) DO UPDATE SET value = EXCLUDED.value;"
        };
        assert_eq!(query, expected);
    }

    #[test]
    fn test_batch_puts() {
        let jobs = JobConfigMap::from([(
            "beta".to_string(),
            JobConfig {
                id: "beta".to_string(),
                from: vec![JobArg {
                    id: "a".to_string(),
                    over: vec![],
                }],
                to: "b".to_string(),
                dims: vec!["i".to_string()],
                spawn_dim: Some("j".to_string()),
                pool_size: 8,
            },
        )]);

        let item = batch_puts(&jobs).collect::<Vec<_>>();
        let expected: Vec<syn::TraitItemFn> = vec![parse_quote! {
            async fn put_all_b(&self, i: schema::IDim, values: Vec<B>) -> Result<(), operon::storage::StorageError> {
                let mut conn = self.pool.get().await?;
                let tx = conn.transaction().await?;
                let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());

                let temp_table_stmt = format!("CREATE TEMP TABLE temp (LIKE {schema_prefix}b INCLUDING ALL) ON COMMIT DROP;");
                tx.execute(&temp_table_stmt, &[]).await?;

                let mut writer = operon::csv::WriterBuilder::new()
                    .has_headers(false)
                    .from_writer(vec![]);
                for (j, value) in values.iter().enumerate() {
                    writer.serialize((i, j, operon::serde_json::to_value(value)?.to_string()))?;
                }
                let copy_stmt = "COPY temp (i, j, value) FROM STDIN WITH (FORMAT csv);";
                let sink = tx.copy_in(copy_stmt).await?;
                let mut sink = Box::pin(sink);
                operon::futures::sink::SinkExt::send(&mut sink, operon::bytes::Bytes::from(writer.into_inner()?)).await?;
                operon::futures::sink::SinkExt::close(&mut sink).await?;

                let insert_stmt = format!("INSERT INTO {schema_prefix}b (i, j, value)\nSELECT i, j, value FROM temp\nON CONFLICT (i, j) DO UPDATE SET value = EXCLUDED.value;");
                tx.execute(&insert_stmt, &[]).await?;
                tx.commit().await?;
                Ok(())
            }
        }];

        assert_eq!(item, expected);
    }
}
