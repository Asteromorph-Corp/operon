use syn::parse_quote;

use crate::configs::EntityConfigMap;
use crate::utils::{operon_ident, sql_storage_ident};

pub(super) fn impl_new(service_id: &str, entities: &EntityConfigMap) -> syn::ItemImpl {
    let operon = operon_ident();
    let sql_storage_ident = sql_storage_ident(service_id);

    let generics = entities
        .values()
        .map(|entity| &entity.generic)
        .collect::<Vec<_>>();

    parse_quote! {
        impl<#(#generics),*> #sql_storage_ident<#(#generics),*> {
            pub fn new(options: #operon::storage::StorageOptions) -> Result<Self, #operon::storage::StorageError> {
                let pg_config: #operon::tokio_postgres::Config = {
                    let mut config = #operon::secrecy::ExposeSecret::expose_secret(&options.database_uri)
                        .parse::<#operon::tokio_postgres::Config>()
                        .map_err(|e| #operon::storage::StorageError::DatabaseUriParseError(e.to_string()))?;
                    config
                        .keepalives(true)
                        .keepalives_idle(options.keepalives_idle)
                        .keepalives_interval(options.keepalives_interval);
                    config
                };
                let manager_config = #operon::deadpool_postgres::ManagerConfig {
                    recycling_method: #operon::deadpool_postgres::RecyclingMethod::Clean,
                };
                let manager = #operon::deadpool_postgres::Manager::from_config(
                    pg_config,
                    #operon::tokio_postgres::NoTls,
                    manager_config,
                );
                let pool = #operon::deadpool_postgres::Pool::builder(manager)
                    .max_size(options.pool_size)
                    .build()?;

                Ok(Self {
                    pool,
                    schema: options.schema,
                    _phantom: std::marker::PhantomData,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_entities, service_id};

    #[rstest]
    fn test_impl_new(service_id: &str, all_entities: EntityConfigMap) {
        let item = impl_new(service_id, &all_entities);
        assert_item_eq(&item, "storage/impl_new.rs");
    }
}
