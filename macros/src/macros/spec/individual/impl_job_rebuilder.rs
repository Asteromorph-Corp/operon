use indexmap::IndexSet;
use syn::parse_quote;

use crate::{
    JobConfig,
    configs::DimensionId,
    utils::{
        explode_ident, mark_done_ident, operon_ident, put_resolution_ident, raise_dep_ident,
        rebuilder_ident, resolution_ident, ticket_ident, variable_ident,
    },
};

/// Generates the implementation of the `JobRebuilder` trait for a given job.
///
/// Example:
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// #[automatically_derived]
/// impl operon::scheduler::JobRebuilder for BetaRebuilder {
///     async fn explode(
///         &self,
///         client: operon::meta_storage::MetaClient<'_>,
///         primary_ub: usize,
///     ) -> Result<(), operon::scheduler::SchedulerError> {
///         queries::explode_beta_i(client, &schema::IResolution(primary_ub)).await?;
///         Ok(())
///     }
///
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
    primary_dimension: &DimensionId,
    spawn_dim_repeating_jobs: &IndexSet<&JobConfig>,
    downstream_jobs: &IndexSet<&JobConfig>,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let rebuilder_ident = rebuilder_ident(&job.id);
    let primary_res_ident = resolution_ident(primary_dimension);
    let ticket_ident = ticket_ident(&job.id);
    let job_id = &job.id;

    let explode_fn_name = explode_ident(&job.id, primary_dimension);

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
            parse_quote! { queries::#raise_dep_fn_name(client, #(&#args,)*).await?; }
        })
        .collect::<Vec<_>>();

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::scheduler::JobRebuilder for #rebuilder_ident {
            async fn explode(
                &self,
                client: #operon::meta_storage::MetaClient<'_>,
                primary_ub: usize,
            ) -> Result<(), #operon::scheduler::SchedulerError> {
                queries::#explode_fn_name(client, &schema::#primary_res_ident(primary_ub)).await?;
                Ok(())
            }

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
    use crate::JobArg;

    use super::*;

    #[test]
    fn test_impl_job_rebuilder() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
        };
        let delta = JobConfig {
            id: "delta".to_string(),
            from: vec![
                JobArg {
                    id: "a".to_string(),
                    over: vec![],
                },
                JobArg {
                    id: "b".to_string(),
                    over: vec![],
                },
                JobArg {
                    id: "c".to_string(),
                    over: vec![],
                },
            ],
            to: "d".to_string(),
            dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
            spawn_dim: None,
        };
        let epsilon = JobConfig {
            id: "epsilon".to_string(),
            from: vec![
                JobArg {
                    id: "b".to_string(),
                    over: vec!["j".to_string()],
                },
                JobArg {
                    id: "d".to_string(),
                    over: vec!["j".to_string()],
                },
            ],
            to: "e".to_string(),
            dims: vec!["i".to_string(), "k".to_string()],
            spawn_dim: None,
        };

        let primary_dimension = "i".to_string();
        let spawn_dim_repeating_jobs = IndexSet::from_iter([&delta]);
        let downstream_jobs = IndexSet::from_iter([&delta, &epsilon]);

        let item = impl_job_rebuilder(
            &job,
            &primary_dimension,
            &spawn_dim_repeating_jobs,
            &downstream_jobs,
        );

        let expected: syn::ItemImpl = parse_quote! {
            #[operon::async_trait::async_trait]
            #[automatically_derived]
            impl operon::scheduler::JobRebuilder for BetaRebuilder {
                async fn explode(
                    &self,
                    client: operon::meta_storage::MetaClient<'_>,
                    primary_ub: usize,
                ) -> Result<(), operon::scheduler::SchedulerError> {
                    queries::explode_beta_i(client, &schema::IResolution(primary_ub)).await?;
                    Ok(())
                }

                async fn rebuild(
                    &self,
                    client: operon::meta_storage::MetaClient<'_>,
                    ui_state: &operon::tokio::sync::RwLock<operon::ui::UiState>,
                ) -> Result<(), operon::scheduler::SchedulerError> {
                    for (job, resolution) in &self.0 {
                        queries::put_resolution_j(client, resolution).await?;
                        queries::mark_done_beta(client, job).await?;

                        queries::explode_delta_j(client, resolution).await?;
                        queries::raise_dep_delta(
                            client,
                            &operon::schema_base::TicketDepCount::some(job.i),
                            &operon::schema_base::TicketDepCount::none(),
                            &operon::schema_base::TicketDepCount::none(),
                        )
                        .await?;
                        queries::raise_dep_epsilon(
                            client,
                            &operon::schema_base::TicketDepCount::some(job.i),
                            &operon::schema_base::TicketDepCount::none(),
                        )
                        .await?;

                        let mut ui_state = ui_state.write().await;
                        let (done, queued, waiting) = <schema::BetaTicket as operon::schema_base::TicketSql>::get_status(client).await?;
                        let state = if queued + waiting == 0 {
                            operon::operon::RunningState::Finished
                        } else {
                            operon::operon::RunningState::Running
                        };
                        ui_state.update_ui_state(operon::ui::UiStateUpdate::ProgressUpdate(
                            "beta".to_string(),
                            (done, queued, waiting, state, false),
                        ))?;
                    }

                    Ok(())
                }
            }
        };

        assert_eq!(item, expected);
    }
}
