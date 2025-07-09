use syn::parse_quote;

use crate::{
    JobConfig,
    configs::JobConfigMap,
    macros::spec::individual::fn_check_consistency::fn_check_consistency,
    utils::{
        job_enum_ident, job_ident, operon_ident, peer_txs_ident, resolution_enum_ident,
        service_trait_ident, spawn_resolution, spec_ident, storage_trait_ident, ticket_ident,
    },
};

// TODO: Implement the actual logic for the methods in this trait.
pub fn impl_job_spec(service_id: &str, job: &JobConfig, _jobs: &JobConfigMap) -> syn::ItemImpl {
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

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl<Svc: #svc_ident, Sto: #sto_ident> #operon::scheduler::JobSpec<Svc, Sto> for #spec_ident {
            type Job = schema::#job_ident;
            type Resolution = #spawn_dim_res;
            type Ticket = schema::#ticket_ident;
            type PeerEventSenders = #peer_txs_ident;

            #fn_check_consistency

            async fn prepare_rebuild(
                &self,
                storage: &Sto,
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<Box<dyn #operon::scheduler::JobRebuilder>, #operon::scheduler::SchedulerError> {
                todo!();
            }

            async fn run_job(
                &self,
                service: &Svc,
                storage: &Sto,
                client: #operon::meta_storage::MetaClient<'_>,
                job: &Self::Job,
            ) -> Result<Self::Resolution, #operon::scheduler::SchedulerError> {
                todo!();
            }

            async fn send_event(
                &self,
                peer_txs: &Self::PeerEventSenders,
                job: Self::Job,
                resolution: Self::Resolution,
            ) -> Result<(), #operon::scheduler::SchedulerError> {
                todo!();
            }

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
