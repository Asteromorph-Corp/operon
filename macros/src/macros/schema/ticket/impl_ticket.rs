use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{job_ident, operon_ident, spawn_resolution, ticket_ident, variable_ident};

/// Generates the implementation of the `Ticket` trait for a given job's ticket.
pub(super) fn impl_ticket(job: &JobConfig) -> syn::ItemImpl {
    let operon = operon_ident();
    let job_ident = job_ident(&job.id);
    let res_ident = spawn_resolution(job.spawn_dim.as_ref());
    let ticket_ident = ticket_ident(&job.id);

    let dim_fields = job
        .dims
        .iter()
        .map(|d| variable_ident(d))
        .collect::<Vec<_>>();

    let initial_quota = job.from.len();
    let initial_done = initial_quota == 0;

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::schema_base::Ticket for #ticket_ident {
            type Job = schema::#job_ident;
            type Resolution = #res_ident;

            fn new() -> Self {
                Self {
                    deps_count: 0,
                    deps_quota: #initial_quota,
                    deps_done: #initial_done,
                    ..Default::default()
                }
            }

            fn update_deps_done(mut self) -> Self {
                self.deps_done = self.deps_count >= self.deps_quota;
                if self.is_ready() {
                    self.status = #operon::schema_base::TicketStatus::Queued;
                }
                self
            }

            fn raise_dependency_count(mut self) -> Self {
                self.deps_count += 1;
                self.update_deps_done()
            }

            fn raise_dependency_quota(mut self, explosion_ub: usize) -> Self {
                self.deps_quota = self.deps_quota + explosion_ub - 1;
                self.update_deps_done()
            }

            fn is_ready(&self) -> bool {
                self.is_resolved() && self.deps_done
            }

            fn is_resolved(&self) -> bool {
                #(self.#dim_fields.is_some())&&*
            }

            fn resolve(&self) -> Option<schema::#job_ident> {
                if !self.is_ready() {
                    return None;
                }

                let job = schema::#job_ident {
                    #(#dim_fields: self.#dim_fields.0?,)*
                };
                Some(job)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::job_beta;

    #[rstest]
    #[case(job_beta(), "schema/ticket/impl_ticket.rs")]
    fn test_impl_ticket(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let item = impl_ticket(&job);
        assert_item_eq(&item, fixture_path);
    }
}
