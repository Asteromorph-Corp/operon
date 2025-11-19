use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{get_all_ident, operon_ident, rebuilder_ident, variable_ident};

/// Generates the `prepare_rebuild` function for the implementation of the trait `JobSpec`.
///
/// Example:
/// ```rust,ignore
/// async fn prepare_rebuild(
///     &self,
///     storage: &Sto,
///     client: operon::meta_storage::MetaClient<'_>,
/// ) -> Result<Box<dyn operon::scheduler::JobRebuilder>, operon::scheduler::SchedulerError> {
///     let tickets = queries::get_all_beta(client, operon::schema_base::TicketStatus::Done).await?;
///     let successes =
///         operon::futures::future::try_join_all(tickets.into_iter().map(|ticket| async move {
///             let job = operon::schema_base::Ticket::resolve(&ticket).ok_or_else(|| {
///                 operon::scheduler::SchedulerError::Other("Failed to resolve a beta ticket".into())
///             })?;
///             let resolution = queries::get_resolution_j(client, job.i)
///                 .await?
///                 .ok_or_else(|| {
///                     operon::scheduler::SchedulerError::Other(format!(
///                         "No resolution found for j_{}",
///                         job.i,
///                     ))
///                 })?;
///
///             Ok::<_, operon::scheduler::SchedulerError>((job, resolution))
///         }))
///         .await?;
///
///     Ok(Box::new(BetaRebuilder(successes)))
/// }
/// ```
pub(super) fn fn_prepare_rebuild(job: &JobConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let rebuilder_ident = rebuilder_ident(&job.id);

    let get_all_fn_name = get_all_ident(&job.id);

    let resolve_fail_msg = format!("Failed to resolve a {} ticket", job.id);

    let dim_vars = job
        .dims
        .iter()
        .map(|d| variable_ident(d))
        .collect::<Vec<_>>();
    let resolution_expr: syn::Expr = match job.spawn_dim.as_ref() {
        Some(dim) => {
            let missing_resolution_msg = format!(
                "No resolution found for {}_{}",
                dim,
                "{},".repeat(job.dims.len()).trim_end_matches(",")
            );

            parse_quote! {
                client.resolution(self.spawn_dim_meta()).get([#(job.#dim_vars),*])
                    .await?
                    .ok_or_else(|| {
                        #operon::scheduler::SchedulerError::Other(format!(
                            #missing_resolution_msg,
                            #(job.#dim_vars,)*
                        ))
                    })?
            }
        }
        None => parse_quote! { () },
    };

    parse_quote! {
        async fn prepare_rebuild(
            &self,
            storage: &Sto,
            client: #operon::meta_storage::MetaClient<'_>,
        ) -> Result<Box<dyn #operon::scheduler::JobRebuilder>, #operon::scheduler::SchedulerError>
        {
            let tickets =
                queries::#get_all_fn_name(client, #operon::schema_base::TicketStatus::Done).await?;
            let successes = #operon::futures::future::try_join_all(tickets.into_iter().map(
                |ticket| async move {
                    let job = #operon::schema_base::Ticket::resolve(&ticket).ok_or_else(|| {
                        #operon::scheduler::SchedulerError::Other(
                            #resolve_fail_msg.into()
                        )
                    })?;
                    let resolution = #resolution_expr;

                    Ok::<_, #operon::scheduler::SchedulerError>((job, resolution))
                }
            ))
            .await?;

            Ok(Box::new(#rebuilder_ident(successes)))
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
    #[case::simple(job_beta(), "spec/fn_prepare_rebuild.rs")]
    fn test_fn_prepare_rebuild(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let item = fn_prepare_rebuild(&job);
        assert_item_eq(&item, fixture_path);
    }
}
