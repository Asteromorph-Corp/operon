use syn::parse_quote;

use crate::configs::{EntityConfig, EntityConfigMap};
use crate::utils::{
    entity_ident, get_entity_ident, operon_ident, put_entity_ident, variable_ident,
};

struct SelectEntityQuery<'a>(&'a EntityConfig);

impl std::fmt::Display for SelectEntityQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entity = self.0;

        write!(f, "SELECT value FROM {{schema_prefix}}{}", self.0.id)?;
        for (idx, dim) in entity.dims.iter().enumerate() {
            if idx == 0 {
                write!(f, " WHERE")?;
            } else {
                write!(f, " AND")?;
            }

            write!(f, " {} = ${}", variable_ident(dim), idx + 1)?;
        }
        write!(f, ";")
    }
}

struct InsertEntityQuery<'a>(&'a EntityConfig);

impl std::fmt::Display for InsertEntityQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entity = self.0;

        write!(f, "INSERT INTO {{schema_prefix}}{} (", entity.id)?;

        if !entity.dims.is_empty() {
            for dim in &entity.dims {
                write!(f, "{dim}, ")?;
            }
        } else {
            write!(f, "id, ")?;
        }
        writeln!(f, "value)")?;

        write!(f, "VALUES (")?;
        if !entity.dims.is_empty() {
            for i in 1..=entity.dims.len() {
                write!(f, "${i}, ")?;
            }
        } else {
            write!(f, "0, ")?;
        }
        writeln!(f, "${})", entity.dims.len() + 1)?;

        write!(f, "ON CONFLICT (")?;
        if !entity.dims.is_empty() {
            write!(f, "{}", entity.dims.join(", "))?;
        } else {
            write!(f, "id")?;
        }
        write!(f, ") DO UPDATE SET value = EXCLUDED.value;")
    }
}

fn single_get(entity: &EntityConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let entity_ident = entity_ident(&entity.id);
    let generic = &entity.generic;

    let get_fn_name = get_entity_ident(&entity.id);
    let get_query = SelectEntityQuery(entity).to_string();

    let args = entity
        .dims
        .iter()
        .map(|d| variable_ident(d))
        .collect::<Vec<_>>();

    parse_quote! {
        async fn #get_fn_name(&self, #(#args: usize),*) -> Result<Option<#entity_ident>, operon::storage::StorageError> {
            let conn = self.pool.get().await?;
            let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());
            let stmt = format!(#get_query);
            let row = conn.query_opt(&stmt, &[#(&i64::try_from(#args)?),*]).await?;

            let Some(row) = row else {
                return Ok(None);
            };
            let value = #operon::serde_json::from_value::<#generic>(row.get(0))?;
            Ok(Some(value.into()))
        }
    }
}

fn single_put(entity: &EntityConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let entity_ident = entity_ident(&entity.id);
    let generic = &entity.generic;

    let put_fn_name = put_entity_ident(&entity.id);
    let put_query = InsertEntityQuery(entity).to_string();

    let args = entity
        .dims
        .iter()
        .map(|d| variable_ident(d))
        .collect::<Vec<_>>();

    parse_quote! {
        async fn #put_fn_name(&self, #(#args: usize,)* value: #entity_ident) -> Result<(), operon::storage::StorageError> {
            let conn = self.pool.get().await?;
            let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());
            let stmt = format!(#put_query);
            let value: #generic = value.into();
            conn.execute(&stmt, &[#(&i64::try_from(#args)?,)* &#operon::serde_json::to_value(value)?]).await?;
            Ok(())
        }
    }
}

pub fn single_ops(entities: &EntityConfigMap) -> impl Iterator<Item = syn::ImplItemFn> {
    entities
        .values()
        .flat_map(|entity| [single_get(entity), single_put(entity)])
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::complicated_pipeline::entity_a as entity_empty;
    use crate::test_utils::simple_pipeline::entity_b;

    #[rstest]
    #[case(
        entity_b(),
        "SELECT value FROM {schema_prefix}b WHERE i = $1 AND j = $2;"
    )]
    fn test_select_entity_query(#[case] entity: EntityConfig, #[case] expected: &str) {
        let stmt = SelectEntityQuery(&entity).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::empty(
        entity_empty(),
        indoc! {"
            INSERT INTO {schema_prefix}a (id, value)
            VALUES (0, $1)
            ON CONFLICT (id) DO UPDATE SET value = EXCLUDED.value;"
        }
    )]
    #[case(
        entity_b(),
        indoc! {"
            INSERT INTO {schema_prefix}b (i, j, value)
            VALUES ($1, $2, $3)
            ON CONFLICT (i, j) DO UPDATE SET value = EXCLUDED.value;"
        }
    )]
    fn test_insert_entity_query(#[case] entity: EntityConfig, #[case] expected: &str) {
        let stmt = InsertEntityQuery(&entity).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(entity_b(), "storage/single_get.rs")]
    fn test_single_get(#[case] entity: EntityConfig, #[case] fixture_path: &str) {
        let item = single_get(&entity);
        assert_item_eq(&item, fixture_path);
    }

    #[rstest]
    #[case::simple(entity_b(), "storage/single_put.rs")]
    fn test_single_put(#[case] entity: EntityConfig, #[case] fixture_path: &str) {
        let item = single_put(&entity);
        assert_item_eq(&item, fixture_path);
    }
}
