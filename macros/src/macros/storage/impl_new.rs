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
    use super::*;

    #[test]
    fn test_impl_new() {
        let service_id = "cooking";
        let entities = EntityConfigMap::default(); // Assuming a default or mock EntityConfigMap

        let impl_item = impl_new(service_id, &entities);
        let expected: syn::ItemImpl = parse_quote! {
            impl<> PsqlCookingStorage<> {
                pub fn new(options: operon::storage::StorageOptions) -> Result<Self, operon::storage::StorageError> {
                    let pg_config: operon::tokio_postgres::Config = {
                        let mut config = operon::secrecy::ExposeSecret::expose_secret(&options.database_uri)
                            .parse::<operon::tokio_postgres::Config>()
                            .map_err(|e| operon::storage::StorageError::DatabaseUriParseError(e.to_string()))?;
                        config
                            .keepalives(true)
                            .keepalives_idle(options.keepalives_idle)
                            .keepalives_interval(options.keepalives_interval);
                        config
                    };
                    let manager_config = operon::deadpool_postgres::ManagerConfig {
                        recycling_method: operon::deadpool_postgres::RecyclingMethod::Clean,
                    };
                    let manager = operon::deadpool_postgres::Manager::from_config(
                        pg_config,
                        operon::tokio_postgres::NoTls,
                        manager_config,
                    );
                    let pool = operon::deadpool_postgres::Pool::builder(manager)
                        .max_size(options.pool_size)
                        .build()?;

                    Ok(Self {
                        pool,
                        schema: options.schema,
                        _phantom: std::marker::PhantomData,
                    })
                }
            }
        };
        assert_eq!(impl_item, expected);
    }
}
