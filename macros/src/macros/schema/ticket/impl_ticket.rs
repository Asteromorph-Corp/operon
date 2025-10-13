use syn::parse_quote;

use crate::configs::{DimensionConfigMap, EntityId};
use crate::macros::schema::ticket::fn_get_dependency_quota::fn_get_dependency_quota;
use crate::utils::{job_ident, operon_ident, spawn_resolution, ticket_ident, variable_ident};
use crate::{JobConfig, JobConfigMap};

/// Generates the implementation of the `Ticket` trait for a given job's ticket.
pub(super) fn impl_ticket(
    job: &JobConfig,
    primary_entity: &EntityId,
    dimensions: &DimensionConfigMap,
    jobs: &JobConfigMap,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let job_ident = job_ident(&job.id);
    let res_ident = spawn_resolution(job.spawn_dim.as_ref());
    let ticket_ident = ticket_ident(&job.id);

    let dim_fields = job
        .dims
        .iter()
        .map(|d| variable_ident(d))
        .collect::<Vec<_>>();

    let fn_new: Option<syn::ImplItemFn> = job
        .from
        .iter()
        .all(|arg| arg.id == *primary_entity)
        .then(|| {
            parse_quote! {
                fn new() -> Self {
                    Self {
                        deps_quota: Some(0),
                        deps_done: true,
                        ..Default::default()
                    }
                }
            }
        });

    let fn_get_dependency_quota = fn_get_dependency_quota(job, primary_entity, dimensions, jobs);

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::schema_base::Ticket for #ticket_ident {
            type Job = schema::#job_ident;
            type Resolution = #res_ident;

            #fn_new
            #fn_get_dependency_quota

            async fn resolve_dependency_quota(
                self,
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<Self, #operon::meta_storage::MetaStorageError> {
                let mut ticket = self;
                if ticket.deps_quota.is_none() {
                    ticket.deps_quota = ticket.get_dependency_quota(client).await?;
                }
                ticket.deps_done = ticket.deps_quota.is_some_and(|quota| ticket.deps_count >= quota);
                if ticket.is_ready() {
                    ticket.status = operon::schema_base::TicketStatus::Queued;
                }
                Ok(ticket)
            }

            async fn raise_dependency_count(
                self,
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<Self, #operon::meta_storage::MetaStorageError> {
                let mut ticket = self;
                ticket.deps_count += 1;
                ticket.deps_done = ticket.deps_quota.is_some_and(|quota| ticket.deps_count >= quota);
                if ticket.is_ready() {
                    ticket.status = operon::schema_base::TicketStatus::Queued;
                }
                Ok(ticket)
            }

            fn is_ready(&self) -> bool {
                self.is_resolved() && self.deps_done
            }

            fn is_resolved(&self) -> bool {
                #(self.#dim_fields.is_some())&&*
            }

            fn resolve(&self) -> Option<schema::#job_ident> {
                if self.is_ready() {
                    Some(schema::#job_ident {
                        #(#dim_fields: self.#dim_fields.0?,)*
                    })
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_dimensions, all_jobs, primary_entity};

    #[rstest]
    #[case("beta", "schema/ticket/impl_ticket.rs")]
    fn test_impl_ticket(
        primary_entity: EntityId,
        all_jobs: JobConfigMap,
        all_dimensions: DimensionConfigMap,
        #[case] job_id: &str,
        #[case] fixture_path: &str,
    ) {
        let job = all_jobs.get(job_id).unwrap();
        let item = impl_ticket(job, &primary_entity, &all_dimensions, &all_jobs);
        assert_item_eq(&item, fixture_path);
    }
}
