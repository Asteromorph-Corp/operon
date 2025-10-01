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

pub fn single_ops(entities: &EntityConfigMap) -> impl Iterator<Item = syn::ImplItemFn> {
    entities.values().flat_map(|entity| -> [syn::ImplItemFn; 2] {
        let operon = operon_ident();
        let entity_ident = entity_ident(&entity.id);
        let generic = &entity.generic;
        let get_fn_name = get_entity_ident(&entity.id);
        let put_fn_name = put_entity_ident(&entity.id);
        let get_query = SelectEntityQuery(entity).to_string();
        let put_query = InsertEntityQuery(entity).to_string();

        let dim_args = entity
            .dims
            .iter()
            .map(|d| -> syn::FnArg {
                let arg_ident = variable_ident(d);
                let arg_ty = dimension_ident(d);

                parse_quote! { #arg_ident: schema::#arg_ty }
            })
            .collect::<Vec<_>>();
        let query_params = entity
            .dims
            .iter()
            .map(|d| variable_ident(d))
            .collect::<Vec<_>>();

        let get_fn = parse_quote! {
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
        };
        let put_fn = parse_quote! {
            async fn #put_fn_name(&self, #(#dim_args,)* value: #entity_ident) -> Result<(), operon::storage::StorageError> {
                let conn = self.pool.get().await?;
                let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());
                let stmt = format!(#put_query);
                let value: #generic = value.into();
                conn.execute(&stmt, &[#(&i64::try_from(#query_params)?,)* &#operon::serde_json::to_value(value)?]).await?;
                Ok(())
            }
        };

        [get_fn, put_fn]
    })
}

#[cfg(test)]
mod tests {
    use quote::format_ident;

    use super::*;

    #[test]
    fn test_select_entity_query() {
        let entity = EntityConfig {
            id: "b".to_string(),
            dims: vec!["i".to_string(), "j".to_string()],
            generic: format_ident!("B_"),
        };
        let query = SelectEntityQuery(&entity).to_string();
        let expected = "SELECT value FROM {schema_prefix}b WHERE i = $1 AND j = $2";
        assert_eq!(query, expected);
    }

    #[test]
    fn test_insert_entity_query() {
        let entity = EntityConfig {
            id: "b".to_string(),
            dims: vec!["i".to_string(), "j".to_string()],
            generic: format_ident!("B_"),
        };
        let query = InsertEntityQuery(&entity).to_string();
        let expected = "INSERT INTO {schema_prefix}b (i, j, value) VALUES ($1, $2, $3) ON CONFLICT (i, j) DO UPDATE SET value = EXCLUDED.value";
        assert_eq!(query, expected);
    }

    #[test]
    fn test_single_ops() {
        let entities = EntityConfigMap::from_iter([(
            "b".to_string(),
            EntityConfig {
                id: "b".to_string(),
                dims: vec!["i".to_string(), "j".to_string()],
                generic: format_ident!("B_"),
            },
        )]);
        let item = single_ops(&entities).collect::<Vec<_>>();
        let expected: Vec<syn::ImplItemFn> = vec![
            parse_quote! {
                async fn get_b(&self, i: schema::IDim, j: schema::JDim) -> Result<Option<B>, operon::storage::StorageError> {
                    let conn = self.pool.get().await?;
                    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());
                    let stmt = format!("SELECT value FROM {schema_prefix}b WHERE i = $1 AND j = $2");
                    let row = conn.query_opt(&stmt, &[&i64::try_from(i)?, &i64::try_from(j)?]).await?;

                    let Some(row) = row else {
                        return Ok(None);
                    };
                    let value = operon::serde_json::from_value::<B_>(row.get(0))?;
                    Ok(Some(value.into()))
                }
            },
            parse_quote! {
                async fn put_b(&self, i: schema::IDim, j: schema::JDim, value: B) -> Result<(), operon::storage::StorageError> {
                    let conn = self.pool.get().await?;
                    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());
                    let stmt = format!("INSERT INTO {schema_prefix}b (i, j, value) VALUES ($1, $2, $3) ON CONFLICT (i, j) DO UPDATE SET value = EXCLUDED.value");
                    let value: B_ = value.into();
                    conn.execute(&stmt, &[&i64::try_from(i)?, &i64::try_from(j)?, &operon::serde_json::to_value(value)?]).await?;
                    Ok(())
                }
            },
        ];
        assert_eq!(item, expected);
    }
}
