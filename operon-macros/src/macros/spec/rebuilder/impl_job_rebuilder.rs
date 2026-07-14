use indexmap::IndexSet;
use quote::quote;
use syn::parse_quote;

use crate::configs::{JobConfig, JobConfigMap};
use crate::dependency_analysis::get_direct_downstream_jobs;
use crate::utils::{job_metadata_ident, operon_ident, rebuilder_ident, to_lit_str};

/// Generates the implementation of the `JobRebuilder` trait for a given job.
///
/// # Example
/// ```rust,ignore
/// #[operon::__private::async_trait::async_trait]
/// #[automatically_derived]
/// impl operon::__private::JobRebuilder for BetaRebuilder {
///     async fn rebuild(
///         &self,
///         client: operon::__private::MetaClient<'_>,
///     ) -> Result<(), operon::error::SchedulerError> {
///         let ready_tickets = client
///             .ticket(self.job_meta)
///             .get_all(operon::__private::TicketStatus::Queued)
///             .await?
///             .into_iter()
///             .map(|ticket| match ticket.resolve() {
///                 Some(job) => Ok(job.coordinate),
///                 None => Err(operon::error::SchedulerError::Other(
///                     "Failed to resolve a beta ticket".into(),
///                 )),
///             })
///             .collect::<Result<std::collections::HashSet<_>, _>>()?;
///
///         let (ready_data, invalid_data): (Vec<_>, Vec<_>) = self
///             .data
///             .iter()
///             .cloned()
///             .partition(|(job, _)| ready_tickets.contains(&job.coordinate));
///         let invalid_tickets = invalid_data
///             .into_iter()
///             .map(|(job, _)| job)
///             .collect::<Vec<_>>();
///
///         operon::__private::futures::future::try_join_all(ready_data.into_iter().map(
///             |(job, resolution)| async move {
///                 client
///                     .resolution(self.spawn_dim_meta)
///                     .put(resolution)
///                     .await?;
///                 client.ticket(self.job_meta).mark_done(job).await?;
///
///                 let affected = client
///                     .ticket(metadata::job_delta_meta())
///                     .explode::<_, 1usize>(self.spawn_dim_meta, resolution)
///                     .await?;
///                 for ticket in affected {
///                     client
///                         .ticket(metadata::job_epsilon_meta())
///                         .raise_deps_quota(
///                             metadata::job_delta_meta(),
///                             ticket,
///                             &["j"],
///                             resolution.ub,
///                         )
///                         .await?;
///                 }
///                 client
///                     .ticket(metadata::job_delta_meta())
///                     .raise_deps_done(self.job_meta, job, &[])
///                     .await?;
///                 client
///                     .ticket(metadata::job_epsilon_meta())
///                     .raise_deps_done(self.job_meta, job, &["j"])
///                     .await?;
///
///                 let (done, queued, waiting) = client.ticket(self.job_meta).get_status().await?;
///                 (*self.progress.write().await).update(done, queued, waiting);
///
///                 Ok::<_, operon::error::SchedulerError>(())
///             },
///         ))
///         .await?;
///
///         // ... warn about `invalid_tickets` ...
///
///         Ok(())
///     }
/// }
/// ```
pub fn impl_job_rebuilder(
    job: &JobConfig,
    spawn_dim_repeating_jobs: &IndexSet<&JobConfig>,
    downstream_jobs: &IndexSet<&JobConfig>,
    all_jobs: &JobConfigMap,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let rebuilder_ident = rebuilder_ident(&job.id);

    let resolve_fail_msg = format!("Failed to resolve a {} ticket", job.id);
    let invalid_ticket_msg = format!(
        "The following {{}} {} ticket(s) were incorrectly marked as done: {{}}",
        job.id
    );

    let maybe_put_resolution = job.spawn_dim.is_some().then(|| -> syn::Stmt {
        parse_quote! { client.resolution(self.spawn_dim_meta).put(resolution).await?; }
    });

    // Jobs without a `spawn_dim` carry a `()` resolution that nothing in the body reads.
    let resolution_pat: syn::Pat = match job.spawn_dim {
        Some(_) => parse_quote!(resolution),
        None => parse_quote!(_),
    };

    let explode_exprs = if let Some(spawn_dim) = &job.spawn_dim {
        spawn_dim_repeating_jobs
            .iter()
            .map(|repeating_job| {
                let Some(idx) = repeating_job.dims.iter().position(|d| d == spawn_dim) else {
                    panic!(
                        "{} not found in repeating job {}",
                        spawn_dim, repeating_job.id
                    );
                };
                let job_meta = job_metadata_ident(&repeating_job.id);

                let explode: syn::Stmt = parse_quote! {
                    let affected = client
                        .ticket(metadata::#job_meta())
                        .explode::<_, #idx>(self.spawn_dim_meta, resolution)
                        .await?;
                };
                let raise_quotas = get_direct_downstream_jobs(repeating_job, all_jobs)
                    .into_iter()
                    .flat_map(|downstream_job| {
                        downstream_job
                            .from
                            .iter()
                            .filter(|arg| {
                                arg.id == repeating_job.to && arg.over.contains(spawn_dim)
                            })
                            .map(|arg| -> syn::Stmt {
                                let repeating_job_meta = job_metadata_ident(&repeating_job.id);
                                let downstream_job_meta = job_metadata_ident(&downstream_job.id);
                                let over = arg.over.iter().map(to_lit_str);
                                parse_quote! {
                                    client.ticket(metadata::#downstream_job_meta())
                                        .raise_deps_quota(metadata::#repeating_job_meta(), ticket, &[#(#over),*], resolution.ub)
                                        .await?;
                                }
                            })
                    });

                quote! {
                    #explode
                    for ticket in affected {
                        #(#raise_quotas)*
                    }
                }
            })
            .collect::<Vec<_>>()
    } else if !spawn_dim_repeating_jobs.is_empty() {
        panic!("`spawn_dim` is `None`, but `spawn_dim_repeating_jobs` is not empty")
    } else {
        vec![]
    };

    let raise_dep_exprs = downstream_jobs
        .iter()
        .flat_map(|downstream_job| {
            let affected_args = downstream_job.from.iter().filter(|arg| arg.id == job.to);
            affected_args.map(|arg| -> syn::Stmt {
                let downstream_job_meta = job_metadata_ident(&downstream_job.id);
                let aggregate_dims = arg.over.iter().map(to_lit_str);

                parse_quote! {
                    client
                        .ticket(metadata::#downstream_job_meta())
                        .raise_deps_done(self.job_meta, job, &[#(#aggregate_dims),*])
                        .await?;
                }
            })
        })
        .collect::<Vec<_>>();

    parse_quote! {
        #[#operon::__private::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::__private::JobRebuilder for #rebuilder_ident {
            async fn rebuild(
                &self,
                client: #operon::__private::MetaClient<'_>,
            ) -> Result<(), #operon::error::SchedulerError> {
                let ready_tickets = client
                    .ticket(self.job_meta)
                    .get_all(#operon::__private::TicketStatus::Queued)
                    .await?
                    .into_iter()
                    .map(|ticket| match ticket.resolve() {
                        Some(job) => Ok(job.coordinate),
                        None => Err(#operon::error::SchedulerError::Other(
                            #resolve_fail_msg.into(),
                        ))
                    })
                    .collect::<Result<std::collections::HashSet<_>, _>>()?;

                let (ready_data, invalid_data): (Vec<_>, Vec<_>) = self
                    .data
                    .iter()
                    .cloned()
                    .partition(|(job, _)| ready_tickets.contains(&job.coordinate));
                let invalid_tickets = invalid_data
                    .into_iter()
                    .map(|(job, _)| job)
                    .collect::<Vec<_>>();

                #operon::__private::futures::future::try_join_all(ready_data.into_iter().map(
                    |(job, #resolution_pat)| async move {
                        #maybe_put_resolution
                        client.ticket(self.job_meta).mark_done(job).await?;

                        #(#explode_exprs)*
                        #(#raise_dep_exprs)*

                        let (done, queued, waiting) = client.ticket(self.job_meta).get_status().await?;
                        (*self.progress.write().await).update(done, queued, waiting);

                        Ok::<_, #operon::error::SchedulerError>(())
                    }
                ))
                .await?;

                if !invalid_tickets.is_empty() {
                    let count = invalid_tickets.len();
                    let ticket_display = if count <= 3 {
                        format!("{:?}", invalid_tickets)
                    } else {
                        format!(
                            "{:?}, {:?}, and {} more",
                            invalid_tickets[0], invalid_tickets[1],
                            count - 2
                        )
                    };
                    #operon::__private::tracing::warn!(#invalid_ticket_msg, count, ticket_display);
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::JobConfigMap;
    use crate::dependency_analysis::{get_direct_downstream_jobs, get_jobs_repeating_on};
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_jobs, job_beta};

    #[rstest]
    #[case::simple(job_beta(), "spec/rebuilder/impl_job_rebuilder.rs")]
    fn test_impl_job_rebuilder(
        all_jobs: JobConfigMap,
        #[case] job: JobConfig,
        #[case] fixture_path: &str,
    ) {
        let spawn_dim_repeating_jobs = job
            .spawn_dim
            .as_ref()
            .map(|dim| get_jobs_repeating_on(dim, &all_jobs))
            .unwrap_or_default();
        let downstream_jobs = get_direct_downstream_jobs(&job, &all_jobs);

        let item = impl_job_rebuilder(&job, &spawn_dim_repeating_jobs, &downstream_jobs, &all_jobs);
        assert_item_eq(&item, fixture_path);
    }
}
