use syn::parse_quote;

use crate::configs::{EntityConfigMap, JobConfigMap};
use crate::macros::storage::batch_gets::batch_gets;
use crate::macros::storage::batch_puts::batch_puts;
use crate::macros::storage::single_ops::single_ops;
use crate::utils::{operon_ident, sql_storage_ident, storage_trait_ident};

pub(super) fn impl_service_storage(
    service_id: &syn::Ident,
    jobs: &JobConfigMap,
    entities: &EntityConfigMap,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let sql_storage_ident = sql_storage_ident(service_id);
    let storage_ident = storage_trait_ident(service_id);

    let single_ops = single_ops(entities);
    let batch_gets = batch_gets(jobs, entities);
    let batch_puts = batch_puts(jobs);

    parse_quote! {
        #[#operon::__private::async_trait::async_trait]
        impl #storage_ident for #sql_storage_ident
        {
            #(#single_ops)*
            #(#batch_gets)*
            #(#batch_puts)*
        }
    }
}
