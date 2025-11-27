use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::{JobConfig, JobConfigMap};
use crate::dependency_analysis::get_direct_downstream_jobs;
use crate::utils::{job_metadata_ident, operon_ident, rebuilder_ident};

/// Generates the implementation of the `JobRebuilder` trait for a given job.
///
/// # Example
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// #[automatically_derived]
/// impl operon::scheduler::JobRebuilder for BetaRebuilder {
///     async fn rebuild(
///         &self,
///         client: operon::meta_storage::MetaClient<'_>,
///         ui_state: &operon::tokio::sync::RwLock<operon::ui::UiState>,
///     ) -> Result<(), operon::scheduler::SchedulerError> {
///         for (job, resolution) in self.data.iter().cloned() {
///             client
///                 .resolution(self.spawn_dim_meta)
///                 .put(resolution)
///                 .await?;
///             client.ticket(self.job_meta).mark_done(job).await?;
///
///             client
///                 .ticket(metadata::job_delta_meta())
///                 .explode::<_, 1usize>(self.spawn_dim_meta, resolution)
///                 .await?;
///             client
///                 .ticket(metadata::job_epsilon_meta())
///                 .raise_deps_quota(self.spawn_dim_meta, resolution)
///                 .await?;
///             client
///                 .ticket(metadata::job_delta_meta())
///                 .raise_deps_done(self.job_meta, job, &[])
///                 .await?;
///             client
///                 .ticket(metadata::job_epsilon_meta())
///                 .raise_deps_done(self.job_meta, job, &["j"])
///                 .await?;
///
///             let mut ui_state = ui_state.write().await;
///             let (done, queued, waiting) = client.ticket(self.job_meta).get_status().await?;
///             let state = if queued + waiting == 0 {
///                 operon::operon::RunningState::Finished
///             } else {
///                 operon::operon::RunningState::Running
///             };
///             ui_state.update_ui_state(operon::ui::UiStateUpdate::ProgressUpdate(
///                 "beta".to_string(),
///                 (done, queued, waiting, state, false),
///             ))?;
///         }
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
    let job_id = &job.id;

    let maybe_put_resolution = job.spawn_dim.is_some().then(|| -> syn::Stmt {
        parse_quote! { client.resolution(self.spawn_dim_meta).put(resolution).await?; }
    });

    let explode_exprs = if let Some(spawn_dim) = &job.spawn_dim {
        let explode = spawn_dim_repeating_jobs
            .iter()
            .map(|repeating_job| -> syn::Stmt {
                let Some(idx) = repeating_job.dims.iter().position(|d| d == spawn_dim) else {
                    panic!(
                        "{} not found in repeating job {}",
                        spawn_dim, repeating_job.id
                    );
                };
                let job_meta = job_metadata_ident(&repeating_job.id);

                parse_quote! {
                    client
                        .ticket(metadata::#job_meta())
                        .explode::<_, #idx>(self.spawn_dim_meta, resolution)
                        .await?;
                }
            });
        let raise_quotas = spawn_dim_repeating_jobs.iter().flat_map(|repeating_job| {
            get_direct_downstream_jobs(repeating_job, all_jobs)
                .into_iter()
                .flat_map(|downstream_job| {
                    let downstream_job_meta = job_metadata_ident(&downstream_job.id);
                    let cnt = downstream_job
                        .from
                        .iter()
                        .filter(|arg| arg.id == repeating_job.to && arg.over.contains(spawn_dim))
                        .count();
                    let stmt: syn::Stmt = parse_quote! {
                        client.ticket(metadata::#downstream_job_meta())
                            .raise_deps_quota(self.spawn_dim_meta, resolution)
                            .await?;
                    };
                    std::iter::repeat_n(stmt, cnt)
                })
        });
        explode.chain(raise_quotas).collect()
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
                let aggregate_dims = &arg.over;

                parse_quote! {
                    client.ticket(metadata::#downstream_job_meta()).raise_deps_done(self.job_meta, job, &[#(#aggregate_dims),*]).await?;
                }
            })
        })
        .collect::<Vec<_>>();

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::scheduler::JobRebuilder for #rebuilder_ident {
            async fn rebuild(
                &self,
                client: #operon::meta_storage::MetaClient<'_>,
                ui_state: &#operon::tokio::sync::RwLock<#operon::ui::UiState>,
            ) -> Result<(), #operon::scheduler::SchedulerError> {
                for (job, resolution) in self.data.iter().cloned() {
                    #maybe_put_resolution
                    client.ticket(self.job_meta).mark_done(job).await?;

                    #(#explode_exprs)*
                    #(#raise_dep_exprs)*

                    // FIXME: I would rather not do this, but every other way of doing this would require massive update of the UI logic
                    let mut ui_state = ui_state.write().await;
                    let (done, queued, waiting) = client.ticket(self.job_meta).get_status().await?;
                    let state = if queued + waiting == 0 {
                        #operon::operon::RunningState::Finished
                    } else {
                        #operon::operon::RunningState::Running
                    };
                    ui_state.update_ui_state(#operon::ui::UiStateUpdate::ProgressUpdate(
                        #job_id.to_string(),
                        (done, queued, waiting, state, false),
                    ))?;
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
