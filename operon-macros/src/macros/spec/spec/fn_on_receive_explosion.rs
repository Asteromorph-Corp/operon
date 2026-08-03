use indexmap::IndexSet;
use quote::quote;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{
    operon_ident, task_metadata_ident, ticket_enum_ident, to_lit_str, to_pascal_case,
};

/// Generates the `on_receive_explosion` function for the implementation of the trait `TaskSpec`.
///
/// # Example
/// ```rust,ignore
/// #[allow(unused_variables, clippy::match_single_binding)]
/// async fn on_receive_explosion(
///     &self,
///     client: MSto::Client<'_>,
///     resolution: schema::ResolutionEnum,
/// ) -> Result<Vec<Self::Ticket>, operon::error::SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
///     match resolution {
///         schema::ResolutionEnum::J(res) => Ok(client
///             .ticket(self.task_meta())
///             .raise_deps_quota(metadata::dimension_j_meta(), res)
///             .await?),
///         _ => Err(operon::error::SchedulerError::InvalidPeerEventReceived(
///             "explosion",
///             "epsilon",
///         )),
///     }
/// }
/// ```
pub(super) fn fn_on_receive_explosion(
    job: &JobConfig,
    upstream_jobs: &IndexSet<&JobConfig>,
) -> syn::ImplItemFn {
    let operon = operon_ident();
    let ticket_enum_ident = ticket_enum_ident();
    let job_id = to_lit_str(&job.id);

    let arms = upstream_jobs.into_iter().map(|upstream_job| -> syn::Arm {
        let variant_ident = to_pascal_case(&upstream_job.id);
        let task_meta = task_metadata_ident(&upstream_job.id);

        let raise_quotas = job
            .from
            .iter()
            .filter(|arg| arg.id == upstream_job.to)
            .map(|arg| {
                let over = arg.over.iter().map(to_lit_str);
                quote! {
                    let aggregate_dims = [#(#over),*];
                    if aggregate_dims.contains(&explosion.dim) {
                        let tickets = client.ticket(self.task_meta())
                            .raise_deps_quota(metadata::#task_meta(), ticket, &aggregate_dims, explosion.ub)
                            .await?;
                        out.extend(tickets);
                    }
                }
            });

        parse_quote! {
            schema::#ticket_enum_ident::#variant_ident(ticket) => {
                let mut out = Vec::new();
                #(#raise_quotas)*
                Ok(out)
            },
        }
    });

    parse_quote! {
        #[allow(unused_variables, clippy::match_single_binding)]
        async fn on_receive_explosion(
            &self,
            client: MSto::Client<'_>,
            explosion: #operon::__private::TicketExplosion<schema::#ticket_enum_ident>,
        ) -> Result<Vec<Self::Ticket>, #operon::error::SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
            match explosion.ticket {
                #(#arms)*
                _ => Err(#operon::error::SchedulerError::InvalidPeerEventReceived("explosion", #job_id)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use rstest::rstest;

    use super::*;
    use crate::configs::JobConfigMap;
    use crate::dependency_analysis::get_direct_upstream_jobs;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::all_jobs;

    #[rstest]
    #[case::simple(format_ident!("epsilon"), "spec/spec/fn_on_receive_explosion.rs")]
    fn test_fn_on_receive_explosion(
        all_jobs: JobConfigMap,
        #[case] job_id: syn::Ident,
        #[case] fixture_path: &str,
    ) {
        let job = all_jobs.get(&job_id).unwrap();
        let upstream_jobs = get_direct_upstream_jobs(job, &all_jobs);
        let item = fn_on_receive_explosion(job, &upstream_jobs);
        assert_item_eq(&item, fixture_path);
    }
}
