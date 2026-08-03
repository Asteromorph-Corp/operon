use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::{DimensionConfigMap, EntityConfigMap, TaskConfig};
use crate::macros::spec::resolution_type::resolution_type;
use crate::macros::spec::spec::fn_all_upstream_tasks::fn_all_upstream_tasks;
use crate::macros::spec::spec::fn_check_consistency::fn_check_consistency;
use crate::macros::spec::spec::fn_default_ticket::fn_default_ticket;
use crate::macros::spec::spec::fn_on_receive_explosion::fn_on_receive_explosion;
use crate::macros::spec::spec::fn_on_receive_job::fn_on_receive_job;
use crate::macros::spec::spec::fn_on_receive_resolution::fn_on_receive_resolution;
use crate::macros::spec::spec::fn_pool_size::fn_pool_size;
use crate::macros::spec::spec::fn_prepare_rebuild::fn_prepare_rebuild;
use crate::macros::spec::spec::fn_run_job::fn_run_job;
use crate::macros::spec::spec::fn_send_on_finish::fn_send_on_finish;
use crate::utils::{
    operon_ident, peer_txs_ident, service_trait_ident, spec_ident, storage_trait_ident,
};

/// Generates the implementation of the `TaskSpec` trait for a given task.
#[allow(clippy::too_many_arguments)]
pub fn impl_task_spec(
    service_id: &syn::Ident,
    task: &TaskConfig,
    all_upstream_tasks: &IndexSet<&TaskConfig>,
    spawn_dim_repeating_tasks: &IndexSet<&TaskConfig>,
    upstream_tasks: &IndexSet<&TaskConfig>,
    downstream_tasks: &IndexSet<&TaskConfig>,
    entities: &EntityConfigMap,
    dimensions: &DimensionConfigMap,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let spec_ident = spec_ident(&task.id);
    let peer_txs_ident = peer_txs_ident(&task.id);
    let n = task.dims.len();

    let svc_ident = service_trait_ident(service_id);
    let sto_ident = storage_trait_ident(service_id);

    let fn_all_upstream_tasks = fn_all_upstream_tasks(all_upstream_tasks);
    let fn_default_ticket = fn_default_ticket(task);
    let fn_check_consistency = fn_check_consistency(task);
    let fn_prepare_rebuild = fn_prepare_rebuild(task);
    let fn_run_job = fn_run_job(task, entities, dimensions);
    let fn_send_on_finish = fn_send_on_finish(task, spawn_dim_repeating_tasks, downstream_tasks);
    let fn_on_receive_job = fn_on_receive_job(task, upstream_tasks);
    let fn_on_receive_resolution = fn_on_receive_resolution(task, downstream_tasks);
    let fn_on_receive_explosion = fn_on_receive_explosion(task, upstream_tasks);
    let fn_pool_size = fn_pool_size(task);

    let resolution = resolution_type(task);

    parse_quote! {
        #[#operon::__private::async_trait::async_trait]
        #[automatically_derived]
        impl<Svc: #svc_ident, Sto: #sto_ident, MSto: #operon::__private::MetaBackend>
            #operon::__private::TaskSpec<Svc, Sto, MSto> for #spec_ident
        {
            type Job = #operon::__private::Job<#n>;
            type Resolution = #resolution;
            type Ticket = #operon::__private::Ticket<#n>;
            type PeerEventSenders = #peer_txs_ident;
            #fn_all_upstream_tasks
            #fn_default_ticket
            #fn_pool_size
            #fn_check_consistency
            #fn_prepare_rebuild
            #fn_run_job
            #fn_send_on_finish
            #fn_on_receive_job
            #fn_on_receive_resolution
            #fn_on_receive_explosion
        }
    }
}
