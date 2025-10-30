use syn::parse_quote;

use crate::configs::{EntityId, JobConfig};
use crate::utils::{job_ident, operon_ident, spawn_resolution, ticket_ident, variable_ident};

/// Generates the implementation of the `Ticket` trait for a given job's ticket.
pub(super) fn impl_ticket(job: &JobConfig, primary_entity: &EntityId) -> syn::ItemImpl {
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
                        deps_quota: 0,
                        deps_done: true,
                        ..Default::default()
                    }
                }
            }
        });

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::schema_base::Ticket for #ticket_ident {
            type Job = schema::#job_ident;
            type Resolution = #res_ident;

            #fn_new

            fn update_deps_done(mut self) -> Self {
                self.deps_done = self.deps_count >= self.deps_quota;
                if self.is_ready() {
                    self.status = operon::schema_base::TicketStatus::Queued;
                }
                self
            }

            fn raise_dependency_count(mut self) -> Self {
                self.deps_count += 1;
                self.update_deps_done()
            }

            fn raise_dependency_quota(mut self, quota: usize) -> Self {
                self.deps_quota += quota;
                self.update_deps_done()
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
    use crate::test_utils::simple_pipeline::{job_beta, primary_entity};

    #[rstest]
    #[case(job_beta(), "schema/ticket/impl_ticket.rs")]
    fn test_impl_ticket(
        primary_entity: EntityId,
        #[case] job: JobConfig,
        #[case] fixture_path: &str,
    ) {
        let item = impl_ticket(&job, &primary_entity);
        assert_item_eq(&item, fixture_path);
    }
}
