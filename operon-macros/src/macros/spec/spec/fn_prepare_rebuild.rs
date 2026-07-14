use quote::quote;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{operon_ident, rebuilder_ident};

/// Generates the `prepare_rebuild` function for the implementation of the trait `JobSpec`.
///
/// # Example
/// ```rust,ignore
/// async fn prepare_rebuild(
///     &self,
///     storage: &Sto,
///     client: MSto::Client<'_>,
/// ) -> Result<Box<dyn operon::__private::JobRebuilder<MSto>>, operon::error::SchedulerError<MSto::Error>> {
///     let tickets = client
///         .ticket(self.job_meta())
///         .get_all(operon::__private::TicketStatus::Done)
///         .await?;
///     let data =
///         operon::__private::futures::future::try_join_all(tickets.into_iter().map(|ticket| async move {
///             let job = ticket.resolve().ok_or_else(|| {
///                 operon::error::SchedulerError::Other("Failed to resolve a beta ticket".into())
///             })?;
///             let resolution = client
///                 .resolution(self.spawn_dim_meta())
///                 .get(job.coordinate)
///                 .await?
///                 .ok_or_else(|| {
///                     operon::error::SchedulerError::Other(format!(
///                         "No resolution found for j_{:?}",
///                         job.coordinate
///                     ))
///                 })?;
///
///             Ok::<_, operon::error::SchedulerError<MSto::Error>>((job, resolution))
///         }))
///         .await?;
///
///     Ok(Box::new(BetaRebuilder {
///         job_meta: self.job_meta(),
///         spawn_dim_meta: self.spawn_dim_meta(),
///         data,
///     }))
/// }
/// ```
pub(super) fn fn_prepare_rebuild(job: &JobConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let rebuilder_ident = rebuilder_ident(&job.id);

    let resolve_fail_msg = format!("Failed to resolve a {} ticket", job.id);

    let resolution_expr: syn::Expr = match job.spawn_dim.as_ref() {
        Some(spawn_dim) => {
            let missing_resolution_msg = format!("No resolution found for {spawn_dim}_{{:?}}");
            parse_quote! {
                client.resolution(self.spawn_dim_meta()).get(job.coordinate)
                    .await?
                    .ok_or_else(|| {
                        #operon::error::SchedulerError::Other(format!(
                            #missing_resolution_msg,
                            job.coordinate
                        ))
                    })?
            }
        }
        None => parse_quote! { () },
    };
    let maybe_spawn_dim_meta = job.spawn_dim.is_some().then(|| {
        quote! { spawn_dim_meta: self.spawn_dim_meta(), }
    });

    parse_quote! {
        async fn prepare_rebuild(
            &self,
            storage: &Sto,
            progress: #operon::__private::SharedProgress,
            client: MSto::Client<'_>,
        ) -> Result<Box<dyn #operon::__private::JobRebuilder<MSto>>, #operon::error::SchedulerError<MSto::Error>>
        {
            let tickets = client
                .ticket(self.job_meta())
                .get_all(#operon::__private::TicketStatus::Done)
                .await?;
            let data = #operon::__private::futures::future::try_join_all(tickets.into_iter().map(
                |ticket| async move {
                    let job = ticket.resolve().ok_or_else(|| {
                        #operon::error::SchedulerError::Other(
                            #resolve_fail_msg.into()
                        )
                    })?;
                    let resolution = #resolution_expr;

                    Ok::<_, #operon::error::SchedulerError<MSto::Error>>((job, resolution))
                }
            ))
            .await?;

            Ok(Box::new(#rebuilder_ident {
                job_meta: self.job_meta(),
                #maybe_spawn_dim_meta
                data,
                progress,
            }))
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
    #[case::simple(job_beta(), "spec/spec/fn_prepare_rebuild.rs")]
    fn test_fn_prepare_rebuild(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let item = fn_prepare_rebuild(&job);
        assert_item_eq(&item, fixture_path);
    }
}
