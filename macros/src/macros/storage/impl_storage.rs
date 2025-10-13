use syn::parse_quote;

use crate::configs::{EntityConfigMap, EntityId};
use crate::macros::storage::generic_constraints;
use crate::utils::{operon_ident, sql_storage_ident};

/// A helper struct to generate SQL queries for creating tables based on job configurations.
struct CreateTablesQuery<'a>(&'a EntityConfigMap);

impl std::fmt::Display for CreateTablesQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for entity in self.0.values() {
            writeln!(
                f,
                "CREATE TABLE IF NOT EXISTS {{schema_prefix}}{} (",
                entity.id
            )?;
            for dim in &entity.dims {
                writeln!(f, "    {dim} BIGINT,")?;
            }
            writeln!(f, "    value JSONB,")?;
            writeln!(f, "    PRIMARY KEY ({})", entity.dims.join(", "))?;
            writeln!(f, ");")?;
        }
        writeln!(
            f,
            "CREATE TABLE IF NOT EXISTS {{schema_prefix}}_data_footprint ("
        )?;
        writeln!(f, "    key TEXT PRIMARY KEY,")?;
        writeln!(f, "    value TEXT NOT NULL")?;
        write!(f, ");")
    }
}

/// A helper struct to generate the SQL storage identifier based on the service ID.
struct TruncateTablesQuery<'a>(&'a EntityId, &'a EntityConfigMap);

impl std::fmt::Display for TruncateTablesQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TRUNCATE TABLE ")?;
        for entity in self.1.values() {
            if entity.id == *self.0 {
                continue;
            }
            write!(f, "{{schema_prefix}}{}, ", entity.id)?;
        }
        write!(f, "{{schema_prefix}}_data_footprint;")
    }
}

pub(super) fn impl_storage(
    service_id: &str,
    primary_entity: &EntityId,
    entities: &EntityConfigMap,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let sql_storage_ident = sql_storage_ident(service_id);

    let create_tables_query = CreateTablesQuery(entities).to_string();
    let truncate_tables_query = TruncateTablesQuery(primary_entity, entities).to_string();

    let generics = entities
        .values()
        .map(|entity| &entity.generic)
        .collect::<Vec<_>>();
    let generic_constraints = generic_constraints(entities);

    parse_quote! {
        #[#operon::async_trait::async_trait]
        impl<#(#generics),*> #operon::storage::OperonStorage for #sql_storage_ident<#(#generics),*>
        where
            #(#generic_constraints,)*
        {
            async fn init(&self) -> Result<(), #operon::storage::StorageError> {
                let client = self.pool.get().await?;
                let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());

                if let Some(schema) = &self.schema {
                    client.execute(&format!("CREATE SCHEMA IF NOT EXISTS {}", schema), &[]).await?;
                }
                client.batch_execute(&format!(#create_tables_query)).await?;
                Ok(())
            }

            async fn clear(&self) -> Result<(), #operon::storage::StorageError> {
                let client = self.pool.get().await?;
                let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());

                client.execute(&format!(#truncate_tables_query), &[]).await?;
                Ok(())
            }

            async fn get_footprint(&self) -> Result<Option<String>, #operon::storage::StorageError> {
                // Get the footprint from the database.
                let client = self.pool.get().await?;
                let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());
                let row = client
                    .query_opt(
                        &format!("SELECT value FROM {schema_prefix}_data_footprint WHERE key = 'global'"),
                        &[],
                    )
                    .await?;
                Ok(row.map(|r| r.get::<_, &str>(0).to_string()))
            }

            async fn put_footprint(&self, value: &str) -> Result<(), #operon::storage::StorageError> {
                // Insert or update the footprint in the database.
                let client = self.pool.get().await?;
                let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());
                client
                    .execute(
                        &format!(
                            "INSERT INTO {schema_prefix}_data_footprint (key, value) VALUES ('global', $1)
                            ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
                        ),
                        &[&value],
                    )
                    .await?;
                Ok(())
            }

            async fn clear_footprint(&self) -> Result<(), #operon::storage::StorageError> {
                // Clear the footprint table.
                let client = self.pool.get().await?;
                let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());
                client
                    .execute(&format!("TRUNCATE TABLE {schema_prefix}_data_footprint"), &[])
                    .await?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;
    use crate::test_utils::simple_pipeline::{all_entities, primary_entity};

    #[rstest]
    fn test_create_tables_query(all_entities: EntityConfigMap) {
        let stmt = super::CreateTablesQuery(&all_entities).to_string();
        let expected = indoc! {"
            CREATE TABLE IF NOT EXISTS {schema_prefix}a (
                i BIGINT,
                value JSONB,
                PRIMARY KEY (i)
            );
            CREATE TABLE IF NOT EXISTS {schema_prefix}b (
                i BIGINT,
                j BIGINT,
                value JSONB,
                PRIMARY KEY (i, j)
            );
            CREATE TABLE IF NOT EXISTS {schema_prefix}c (
                i BIGINT,
                k BIGINT,
                value JSONB,
                PRIMARY KEY (i, k)
            );
            CREATE TABLE IF NOT EXISTS {schema_prefix}d (
                i BIGINT,
                j BIGINT,
                k BIGINT,
                value JSONB,
                PRIMARY KEY (i, j, k)
            );
            CREATE TABLE IF NOT EXISTS {schema_prefix}e (
                i BIGINT,
                k BIGINT,
                value JSONB,
                PRIMARY KEY (i, k)
            );
            CREATE TABLE IF NOT EXISTS {schema_prefix}f (
                i BIGINT,
                value JSONB,
                PRIMARY KEY (i)
            );
            CREATE TABLE IF NOT EXISTS {schema_prefix}_data_footprint (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
        };

        assert_eq!(stmt, expected);
    }

    #[rstest]
    fn test_clear_tables_query(primary_entity: EntityId, all_entities: EntityConfigMap) {
        let stmt = super::TruncateTablesQuery(&primary_entity, &all_entities).to_string();
        let expected = "TRUNCATE TABLE {schema_prefix}b, {schema_prefix}c, {schema_prefix}d, {schema_prefix}e, {schema_prefix}f, {schema_prefix}_data_footprint;";
        assert_eq!(stmt, expected);
    }
}
