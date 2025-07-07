use syn::parse_quote;

use crate::{
    JobConfig,
    utils::{operon_ident, rebuilder_ident},
};

// TODO: Implement the actual logic for the methods in this trait.
pub fn impl_job_rebuilder(job: &JobConfig) -> syn::ItemImpl {
    let operon = operon_ident();
    let rebuilder_ident = rebuilder_ident(&job.id);

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::scheduler::JobRebuilder for #rebuilder_ident {
            async fn explode(
                &self,
                client: #operon::meta_storage::MetaClient<'_>,
                primary_ub: usize,
            ) -> Result<(), #operon::scheduler::SchedulerError> {
                todo!();
            }

            async fn rebuild(
                &self,
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), #operon::scheduler::SchedulerError> {
                todo!();
            }
        }
    }
}
