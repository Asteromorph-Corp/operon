use syn::parse_quote;

use crate::configs::{EntityConfigMap, JobConfigMap};
use crate::macros::storage::batch_gets::batch_gets;
use crate::macros::storage::batch_puts::batch_puts;
use crate::macros::storage::generic_constraints;
use crate::macros::storage::single_ops::single_ops;
use crate::utils::{operon_ident, storage_trait_ident};

pub(super) fn impl_service_storage(
    service_id: &str,
    jobs: &JobConfigMap,
    entities: &EntityConfigMap,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let sql_storage_ident = crate::utils::sql_storage_ident(service_id);
    let storage_ident = storage_trait_ident(service_id);

    let generics = entities
        .values()
        .map(|entity| &entity.generic)
        .collect::<Vec<_>>();
    let generic_constraints = generic_constraints(entities);
    let single_ops = single_ops(entities);
    let batch_gets = batch_gets(jobs, entities);
    let batch_puts = batch_puts(jobs);

    parse_quote! {
        #[#operon::async_trait::async_trait]
        impl<#(#generics),*> #storage_ident for #sql_storage_ident<#(#generics),*>
        where
            #(#generic_constraints,)*
        {
            #(#single_ops)*
            #(#batch_gets)*
            #(#batch_puts)*
        }
    }
}
