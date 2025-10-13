use syn::parse_quote;

use crate::EntityConfig;
use crate::configs::EntityConfigMap;
use crate::utils::{
    dimension_ident, entity_ident, get_entity_ident, operon_ident, put_entity_ident, variable_ident,
};

struct SelectEntityQuery<'a>(&'a EntityConfig);

impl std::fmt::Display for SelectEntityQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entity = self.0;

        write!(f, "SELECT value FROM {{schema_prefix}}{}", self.0.id)?;
        for (i, dim) in entity.dims.iter().enumerate() {
            if i == 0 {
                write!(f, " WHERE")?;
            } else {
                write!(f, " AND")?;
            }

            write!(f, " {} = ${}", variable_ident(dim), i + 1)?;
        }
        Ok(())
    }
}

struct InsertEntityQuery<'a>(&'a EntityConfig);

impl std::fmt::Display for InsertEntityQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entity = self.0;

        write!(f, "INSERT INTO {{schema_prefix}}{} (", entity.id)?;
        for dim in &entity.dims {
            write!(f, "{dim}, ")?;
        }
        write!(f, "value) VALUES (")?;
        for i in 1..=entity.dims.len() {
            write!(f, "${i}, ")?;
        }
        write!(f, "${})", entity.dims.len() + 1)?;
        write!(f, " ON CONFLICT (")?;
        for (i, dim) in entity.dims.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{dim}")?;
        }
        write!(f, ") DO UPDATE SET value = EXCLUDED.value")
    }
}

fn dim_args(entity: &EntityConfig) -> impl Iterator<Item = syn::FnArg> {
    entity.dims.iter().map(|d| -> syn::FnArg {
        let arg_ident = variable_ident(d);
        let arg_ty = dimension_ident(d);
        parse_quote! { #arg_ident: schema::#arg_ty }
    })
}

fn query_params(entity: &EntityConfig) -> impl Iterator<Item = syn::Ident> {
    entity.dims.iter().map(|d| variable_ident(d))
}

fn single_get(entity: &EntityConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let entity_ident = entity_ident(&entity.id);
    let generic = &entity.generic;

    let get_fn_name = get_entity_ident(&entity.id);
    let get_query = SelectEntityQuery(entity).to_string();

    let dim_args = dim_args(entity);
    let query_params = query_params(entity);

    parse_quote! {
        async fn #get_fn_name(&self, #(#dim_args),*) -> Result<Option<#entity_ident>, operon::storage::StorageError> {
            let conn = self.pool.get().await?;
            let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());
            let stmt = format!(#get_query);
            let row = conn.query_opt(&stmt, &[#(&i64::try_from(#query_params)?),*]).await?;

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

    let dim_args = dim_args(entity);
    let query_params = query_params(entity);

    parse_quote! {
        async fn #put_fn_name(&self, #(#dim_args,)* value: #entity_ident) -> Result<(), operon::storage::StorageError> {
            let conn = self.pool.get().await?;
            let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());
            let stmt = format!(#put_query);
            let value: #generic = value.into();
            conn.execute(&stmt, &[#(&i64::try_from(#query_params)?,)* &#operon::serde_json::to_value(value)?]).await?;
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
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::entity_b;

    #[rstest]
    #[case(
        entity_b(),
        "SELECT value FROM {schema_prefix}b WHERE i = $1 AND j = $2"
    )]
    fn test_select_entity_query(#[case] entity: EntityConfig, #[case] expected: &str) {
        let stmt = SelectEntityQuery(&entity).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case(
        entity_b(),
        "INSERT INTO {schema_prefix}b (i, j, value) VALUES ($1, $2, $3) ON CONFLICT (i, j) DO UPDATE SET value = EXCLUDED.value"
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
