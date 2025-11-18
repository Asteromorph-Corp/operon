use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{
    explode_ident, mark_done_ident, operon_ident, put_resolution_ident, raise_dep_ident,
    rebuilder_ident, ticket_ident, variable_ident,
};

/// Generates the implementation of the `JobRebuilder` trait for a given job.
///
/// Example:
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// #[automatically_derived]
/// impl operon::scheduler::JobRebuilder for BetaRebuilder {
///     async fn rebuild(
///         &self,
///         client: operon::meta_storage::MetaClient<'_>,
///     ) -> Result<(), operon::scheduler::SchedulerError> {
///         for (job, resolution) in &self.0 {
///             queries::put_resolution_j(client, resolution).await?;
///             queries::mark_done_beta(client, job).await?;
///
///             queries::explode_delta_j(client, resolution).await?;
///             queries::raise_dep_delta(
///                 client,
///                 &operon::schema_base::TicketDepCount::some(job.i),
///                 &operon::schema_base::TicketDepCount::none(),
///                 &operon::schema_base::TicketDepCount::none(),
///             )
///             .await?;
///             queries::raise_dep_epsilon(
///                 client,
///                 &operon::schema_base::TicketDepCount::some(job.i),
///                 &operon::schema_base::TicketDepCount::none(),
///             )
///             .await?;
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
) -> syn::ItemImpl {
    let operon = operon_ident();
    let rebuilder_ident = rebuilder_ident(&job.id);
    let ticket_ident = ticket_ident(&job.id);
    let job_id = &job.id;

    let maybe_put_resolution = job.spawn_dim.as_ref().map(|dim| -> syn::Stmt {
        let put_resolution_fn_name = put_resolution_ident(dim);
        parse_quote! { queries::#put_resolution_fn_name(client, resolution).await?; }
    });
    let mark_done_fn_name = mark_done_ident(&job.id);

    let explode_exprs = if !spawn_dim_repeating_jobs.is_empty() {
        let spawn_dim = job
            .spawn_dim
            .as_ref()
            .expect("No spawn dimension provided, but repeating jobs are specified");
        spawn_dim_repeating_jobs
            .iter()
            .map(|repeating_job| -> syn::Stmt {
                let explode_fn_name = explode_ident(&repeating_job.id, spawn_dim);
                parse_quote! { queries::#explode_fn_name(client, resolution).await?; }
            })
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    let raise_dep_exprs = downstream_jobs
        .iter()
        .map(|downstream_job| -> syn::Stmt {
            let raise_dep_fn_name = raise_dep_ident(&downstream_job.id);
            let args = downstream_job.dims.iter().map(|d| -> syn::Expr {
                if job.dims.contains(d) {
                    let field_ident = variable_ident(d);
                    parse_quote! { #operon::schema_base::TicketDepCount::some(job.#field_ident) }
                } else {
                    parse_quote! { #operon::schema_base::TicketDepCount::none() }
                }
            });
            parse_quote! { queries::#raise_dep_fn_name(client, #(#args,)*).await?; }
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
                for (job, resolution) in &self.0 {
                    #maybe_put_resolution
                    queries::#mark_done_fn_name(client, job).await?;

                    #(#explode_exprs)*
                    #(#raise_dep_exprs)*

                    // FIXME: I would rather not do this, but every other way of doing this would require massive update of the UI logic
                    let mut ui_state = ui_state.write().await;
                    let (done, queued, waiting) = <schema::#ticket_ident as #operon::schema_base::TicketSql>::get_status(client).await?;
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
    #[case::simple(job_beta(), "spec/impl_job_rebuilder.rs")]
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

        let item = impl_job_rebuilder(&job, &spawn_dim_repeating_jobs, &downstream_jobs);
        assert_item_eq(&item, fixture_path);
    }
}
