use indexmap::IndexSet;
use quote::format_ident;
use syn::parse_quote;

use crate::configs::TaskConfig;
use crate::utils::{operon_ident, resolution_enum_ident, sender_ident, to_pascal_case};

/// Generates the `send_on_finish` function for the implementation of the trait `TaskSpec`.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/spec/spec/fn_send_on_finish.rs") )]
/// ```
pub(super) fn fn_send_on_finish(
    task: &TaskConfig,
    spawn_dim_repeating_tasks: &IndexSet<&TaskConfig>,
    downstream_tasks: &IndexSet<&TaskConfig>,
) -> syn::ImplItemFn {
    let operon = operon_ident();
    let resolution_enum_ident = resolution_enum_ident();

    let send_resolutions = spawn_dim_repeating_tasks
        .iter()
        .map(|repeating_task| -> syn::Expr {
            // TODO: remove unwrap
            let resolution_variant_ident = to_pascal_case(task.spawn_dim.as_ref().unwrap());
            let sender_ident = sender_ident(&repeating_task.id);
            let ok_msg = format!(
                "`{}` sent peer event to `{}`: {{resolution:?}}",
                task.id, repeating_task.id
            );
            let err_msg = format!(
                "`{}`'s peer channel closed before handling `{}`'s {{resolution:?}}",
                repeating_task.id, task.id
            );

            parse_quote! {
                match peer_txs
                    .#sender_ident
                    .send(#operon::__private::PeerEvent::Resolution(schema::#resolution_enum_ident::#resolution_variant_ident(resolution)))
                    .await
                {
                    Ok(_) => #operon::__private::tracing::trace!(#ok_msg),
                    Err(_) => #operon::__private::tracing::trace!(#err_msg),
                }
            }
        });

    let send_jobs = downstream_tasks.iter().map(|downstream_task| -> syn::Expr {
        let job_variant_ident = to_pascal_case(&format_ident!("{}", task.id));
        let sender_ident = sender_ident(&downstream_task.id);
        let ok_msg = format!(
            "`{}` sent peer event to `{}`: {{job:?}}",
            task.id, downstream_task.id
        );
        let err_msg = format!(
            "`{}`'s peer channel closed before handling `{}`'s {{job:?}}",
            downstream_task.id, task.id
        );

        parse_quote! {
            match peer_txs
                .#sender_ident
                .send(#operon::__private::PeerEvent::Job(schema::JobEnum::#job_variant_ident(job)))
                .await
            {
                Ok(_) => #operon::__private::tracing::trace!(#ok_msg),
                Err(_) => #operon::__private::tracing::trace!(#err_msg),
            }
        }
    });

    parse_quote! {
        async fn send_on_finish(
            &self,
            peer_txs: &Self::PeerEventSenders,
            job: Self::Job,
            resolution: Self::Resolution,
        ) -> Result<(), #operon::error::SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
            #(#send_resolutions)*
            #(#send_jobs)*
            Ok(())
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
    #[case::simple(task_beta(), "spec/spec/fn_send_on_finish.rs")]
    fn test_fn_send_on_finish(
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

        let item = fn_send_on_finish(&task, &spawn_dim_repeating_tasks, &downstream_tasks);
        assert_item_eq(&item, fixture_path);
    }
}
