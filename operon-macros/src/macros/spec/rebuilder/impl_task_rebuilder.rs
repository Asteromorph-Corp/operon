use indexmap::IndexSet;
use quote::quote;
use syn::parse_quote;

use crate::configs::{TaskConfig, TaskConfigMap};
use crate::dependency_analysis::get_direct_downstream_tasks;
use crate::utils::{operon_ident, rebuilder_ident, task_metadata_ident, to_lit_str};

/// Generates the implementation of the `TaskRebuilder` trait for a given task.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/spec/rebuilder/impl_task_rebuilder.rs") )]
/// ```
pub fn impl_task_rebuilder(
    task: &TaskConfig,
    spawn_dim_repeating_tasks: &IndexSet<&TaskConfig>,
    downstream_tasks: &IndexSet<&TaskConfig>,
    all_tasks: &TaskConfigMap,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let rebuilder_ident = rebuilder_ident(&task.id);

    let resolve_fail_msg = format!("Failed to resolve a {} ticket", task.id);
    let invalid_ticket_msg = format!(
        "The following {{}} {} ticket(s) were incorrectly marked as done: {{}}",
        task.id
    );

    let maybe_put_resolution = task.spawn_dim.is_some().then(|| -> syn::Stmt {
        parse_quote! { client.resolution(self.spawn_dim_meta).put(resolution).await?; }
    });

    // A task without a `spawn_dim` produces jobs whose `()` resolution nothing in the body reads.
    let resolution_pat: syn::Pat = match task.spawn_dim {
        Some(_) => parse_quote!(resolution),
        None => parse_quote!(_),
    };

    let explode_exprs = if let Some(spawn_dim) = &task.spawn_dim {
        spawn_dim_repeating_tasks
            .iter()
            .map(|repeating_task| {
                let Some(idx) = repeating_task.dims.iter().position(|d| d == spawn_dim) else {
                    panic!(
                        "{} not found in repeating task {}",
                        spawn_dim, repeating_task.id
                    );
                };
                let task_meta = task_metadata_ident(&repeating_task.id);

                let explode: syn::Stmt = parse_quote! {
                    let affected = client
                        .ticket(metadata::#task_meta())
                        .explode::<_, #idx>(self.spawn_dim_meta, resolution)
                        .await?;
                };
                let raise_quotas = get_direct_downstream_tasks(repeating_task, all_tasks)
                    .into_iter()
                    .flat_map(|downstream_task| {
                        downstream_task
                            .from
                            .iter()
                            .filter(|arg| {
                                arg.id == repeating_task.to && arg.over.contains(spawn_dim)
                            })
                            .map(|arg| -> syn::Stmt {
                                let repeating_task_meta = task_metadata_ident(&repeating_task.id);
                                let downstream_task_meta = task_metadata_ident(&downstream_task.id);
                                let over = arg.over.iter().map(to_lit_str);
                                parse_quote! {
                                    client.ticket(metadata::#downstream_task_meta())
                                        .raise_deps_quota(metadata::#repeating_task_meta(), ticket, &[#(#over),*], resolution.ub)
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
    } else if !spawn_dim_repeating_tasks.is_empty() {
        panic!("`spawn_dim` is `None`, but `spawn_dim_repeating_tasks` is not empty")
    } else {
        vec![]
    };

    let raise_dep_exprs = downstream_tasks
        .iter()
        .flat_map(|downstream_task| {
            let affected_args = downstream_task.from.iter().filter(|arg| arg.id == task.to);
            affected_args.map(|arg| -> syn::Stmt {
                let downstream_task_meta = task_metadata_ident(&downstream_task.id);
                let aggregate_dims = arg.over.iter().map(to_lit_str);

                parse_quote! {
                    client
                        .ticket(metadata::#downstream_task_meta())
                        .raise_deps_done(self.task_meta, job, &[#(#aggregate_dims),*])
                        .await?;
                }
            })
        })
        .collect::<Vec<_>>();

    parse_quote! {
        #[#operon::__private::async_trait::async_trait]
        #[automatically_derived]
        impl<
            Svc: #operon::OperonService,
            Sto: #operon::OperonStorage,
            MSto: #operon::__private::MetaBackend,
        > #operon::__private::TaskRebuilder<Svc, Sto, MSto> for #rebuilder_ident
        {
            async fn rebuild(
                &self,
                client: MSto::Client<'_>,
            ) -> Result<(), #operon::error::SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
                use #operon::__private::futures::{StreamExt, TryStreamExt};

                let ready_tickets = client
                    .ticket(self.task_meta)
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

                #operon::__private::futures::stream::iter(ready_data.into_iter().map(
                    |(job, #resolution_pat)| async move {
                        #maybe_put_resolution
                        client.ticket(self.task_meta).mark_done(job).await?;

                        #(#explode_exprs)*
                        #(#raise_dep_exprs)*

                        let (done, queued, waiting) = client.ticket(self.task_meta).get_status().await?;
                        (*self.progress.write().await).update(done, queued, waiting);

                        Ok::<_, #operon::error::SchedulerError<Svc::Error, Sto::Error, MSto::Error>>(())
                    }
                ))
                .buffer_unordered(#operon::__private::REBUILD_CONCURRENCY)
                .try_collect::<Vec<_>>()
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
    use crate::configs::TaskConfigMap;
    use crate::dependency_analysis::{get_direct_downstream_tasks, get_tasks_repeating_on};
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_tasks, task_beta};

    #[rstest]
    #[case::simple(task_beta(), "spec/rebuilder/impl_task_rebuilder.rs")]
    fn test_impl_task_rebuilder(
        all_tasks: TaskConfigMap,
        #[case] task: TaskConfig,
        #[case] fixture_path: &str,
    ) {
        let spawn_dim_repeating_tasks = task
            .spawn_dim
            .as_ref()
            .map(|dim| get_tasks_repeating_on(dim, &all_tasks))
            .unwrap_or_default();
        let downstream_tasks = get_direct_downstream_tasks(&task, &all_tasks);

        let item = impl_task_rebuilder(
            &task,
            &spawn_dim_repeating_tasks,
            &downstream_tasks,
            &all_tasks,
        );
        assert_item_eq(&item, fixture_path);
    }
}
