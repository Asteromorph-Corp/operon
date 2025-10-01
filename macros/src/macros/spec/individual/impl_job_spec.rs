use indexmap::IndexSet;
use syn::parse_quote;

use crate::JobConfig;
use crate::configs::{DimensionConfigMap, EntityConfigMap};
use crate::macros::spec::individual::fn_check_consistency::fn_check_consistency;
use crate::macros::spec::individual::fn_on_receive_job::fn_on_receive_job;
use crate::macros::spec::individual::fn_on_receive_resolution::fn_on_receive_resolution;
use crate::macros::spec::individual::fn_pool_size::fn_pool_size;
use crate::macros::spec::individual::fn_prepare_rebuild::fn_prepare_rebuild;
use crate::macros::spec::individual::fn_run_job::fn_run_job;
use crate::macros::spec::individual::fn_send_on_finish::fn_send_on_finish;
use crate::utils::{
    job_ident, operon_ident, peer_txs_ident, service_trait_ident, spawn_resolution, spec_ident,
    storage_trait_ident, ticket_ident,
};

/// Generates the implementation of the `JobSpec` trait for a given job.
pub fn impl_job_spec(
    service_id: &str,
    job: &JobConfig,
    resolution_receiving_jobs: &IndexSet<&JobConfig>,
    upstream_jobs: &IndexSet<&JobConfig>,
    downstream_jobs: &IndexSet<&JobConfig>,
    entities: &EntityConfigMap,
    dimensions: &DimensionConfigMap,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let spec_ident = spec_ident(&job.id);
    let job_ident = job_ident(&job.id);
    let spawn_dim_res: syn::Type = spawn_resolution(job.spawn_dim.as_ref());
    let ticket_ident = ticket_ident(&job.id);
    let peer_txs_ident = peer_txs_ident(&job.id);

    let svc_ident = service_trait_ident(service_id);
    let sto_ident = storage_trait_ident(service_id);

    let fn_check_consistency = fn_check_consistency(job);
    let fn_prepare_rebuild = fn_prepare_rebuild(job);
    let fn_run_job = fn_run_job(job, entities, dimensions);
    let fn_send_on_finish = fn_send_on_finish(job, resolution_receiving_jobs, downstream_jobs);
    let fn_on_receive_job = fn_on_receive_job(job, upstream_jobs);
    let fn_on_receive_resolution = fn_on_receive_resolution(job, dimensions);
    let fn_pool_size = fn_pool_size(job);

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl<Svc: #svc_ident, Sto: #sto_ident> #operon::scheduler::JobSpec<Svc, Sto> for #spec_ident {
            type Job = schema::#job_ident;
            type Resolution = #spawn_dim_res;
            type Ticket = schema::#ticket_ident;
            type PeerEventSenders = #peer_txs_ident;

            #fn_pool_size
            #fn_check_consistency
            #fn_prepare_rebuild
            #fn_run_job
            #fn_send_on_finish
            #fn_on_receive_job
            #fn_on_receive_resolution
        }
    }
}
