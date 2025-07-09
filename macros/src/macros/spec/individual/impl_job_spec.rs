use indexmap::IndexSet;
use syn::parse_quote;

use crate::{
    JobConfig,
    macros::spec::individual::{
        fn_check_consistency::fn_check_consistency, fn_prepare_rebuild::fn_prepare_rebuild,
        fn_send_on_finish::fn_send_on_finish,
    },
    utils::{
        job_enum_ident, job_ident, operon_ident, peer_txs_ident, resolution_enum_ident,
        service_trait_ident, spawn_resolution, spec_ident, storage_trait_ident, ticket_ident,
    },
};

// TODO: Implement the actual logic for the methods in this trait.
pub fn impl_job_spec(
    service_id: &str,
    job: &JobConfig,
    spawn_dim_repeating_jobs: &IndexSet<&JobConfig>,
    downstream_jobs: &IndexSet<&JobConfig>,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let spec_ident = spec_ident(&job.id);
    let job_ident = job_ident(&job.id);
    let spawn_dim_res: syn::Type = spawn_resolution(job.spawn_dim.as_ref());
    let ticket_ident = ticket_ident(&job.id);
    let peer_txs_ident = peer_txs_ident(&job.id);
    let job_enum_ident = job_enum_ident();
    let res_enum_ident = resolution_enum_ident();

    let svc_ident = service_trait_ident(service_id);
    let sto_ident = storage_trait_ident(service_id);

    let fn_check_consistency = fn_check_consistency(job);
    let fn_prepare_rebuild = fn_prepare_rebuild(job);
    let fn_send_on_finish = fn_send_on_finish(job, spawn_dim_repeating_jobs, downstream_jobs);

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl<Svc: #svc_ident, Sto: #sto_ident> #operon::scheduler::JobSpec<Svc, Sto> for #spec_ident {
            type Job = schema::#job_ident;
            type Resolution = #spawn_dim_res;
            type Ticket = schema::#ticket_ident;
            type PeerEventSenders = #peer_txs_ident;

            #fn_check_consistency
            #fn_prepare_rebuild

            async fn run_job(
                &self,
                service: &Svc,
                storage: &Sto,
                client: #operon::meta_storage::MetaClient<'_>,
                job: &Self::Job,
            ) -> Result<Self::Resolution, #operon::scheduler::SchedulerError> {
                todo!();
            }

            #fn_send_on_finish

            async fn on_job_ready_tickets(
                &self,
                client: #operon::meta_storage::MetaClient<'_>,
                job: schema::#job_enum_ident,
            ) -> Result<Vec<Self::Ticket>, #operon::scheduler::SchedulerError> {
                todo!();
            }

            async fn on_resolution_ready_tickets(
                &self,
                client: #operon::meta_storage::MetaClient<'_>,
                resolution: schema::#res_enum_ident,
            ) -> Result<Vec<Self::Ticket>, #operon::scheduler::SchedulerError> {
                todo!();
            }
        }
    }
}
