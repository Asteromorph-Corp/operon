use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::{DimensionConfigMap, EntityConfigMap, JobConfig};
use crate::macros::spec::resolution_type::resolution_type;
use crate::macros::spec::spec::fn_all_upstream_jobs::fn_all_upstream_jobs;
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

/// Generates the implementation of the `JobSpec` trait for a given job.
#[allow(clippy::too_many_arguments)]
pub fn impl_job_spec(
    service_id: &syn::Ident,
    job: &JobConfig,
    all_upstream_jobs: &IndexSet<&JobConfig>,
    spawn_dim_repeating_jobs: &IndexSet<&JobConfig>,
    upstream_jobs: &IndexSet<&JobConfig>,
    downstream_jobs: &IndexSet<&JobConfig>,
    entities: &EntityConfigMap,
    dimensions: &DimensionConfigMap,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let spec_ident = spec_ident(&job.id);
    let peer_txs_ident = peer_txs_ident(&job.id);
    let n = job.dims.len();

    let svc_ident = service_trait_ident(service_id);
    let sto_ident = storage_trait_ident(service_id);

    let fn_all_upstream_jobs = fn_all_upstream_jobs(all_upstream_jobs);
    let fn_default_ticket = fn_default_ticket(job);
    let fn_check_consistency = fn_check_consistency(job);
    let fn_prepare_rebuild = fn_prepare_rebuild(job);
    let fn_run_job = fn_run_job(job, entities, dimensions);
    let fn_send_on_finish = fn_send_on_finish(job, spawn_dim_repeating_jobs, downstream_jobs);
    let fn_on_receive_job = fn_on_receive_job(job, upstream_jobs);
    let fn_on_receive_resolution = fn_on_receive_resolution(job, downstream_jobs);
    let fn_on_receive_explosion = fn_on_receive_explosion(job, upstream_jobs);
    let fn_pool_size = fn_pool_size(job);

    let resolution = resolution_type(job);

    parse_quote! {
        #[#operon::__private::async_trait::async_trait]
        #[automatically_derived]
        impl<Svc: #svc_ident, Sto: #sto_ident> #operon::__private::JobSpec<Svc, Sto> for #spec_ident {
            type Job = #operon::__private::Job<#n>;
            type Resolution = #resolution;
            type Ticket = #operon::__private::Ticket<#n>;
            type PeerEventSenders = #peer_txs_ident;
            #fn_all_upstream_jobs
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
