use indexmap::IndexSet;
use quote::format_ident;
use syn::parse_quote;

use crate::configs::TaskConfig;
use crate::utils::{job_enum_ident, operon_ident, task_metadata_ident, to_lit_str, to_pascal_case};

/// Generates the `on_receive_job` function for a task.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/spec/spec/fn_on_receive_job.rs"))]
/// ```
pub(super) fn fn_on_receive_job(
    task: &TaskConfig,
    upstream_tasks: &IndexSet<&TaskConfig>,
) -> syn::ImplItemFn {
    let operon = operon_ident();
    let job_enum_ident = job_enum_ident();
    let task_id = to_lit_str(&task.id);

    let job_arms = upstream_tasks.iter().map(|upstream_task| -> syn::Arm {
        let upstream_task_meta = task_metadata_ident(&upstream_task.id);
        let variant_ident = to_pascal_case(&format_ident!("{}", upstream_task.id));
        let affected_args = task.from.iter().filter(|arg| arg.id == upstream_task.to);

        let raise_deps_done = affected_args.map(|arg| -> syn::Expr {
            let aggregate_dims = arg.over.iter().map(to_lit_str);
            parse_quote! {
                client
                    .ticket(self.task_meta())
                    .raise_deps_done(metadata::#upstream_task_meta(), job, &[#(#aggregate_dims),*])
                    .await?
            }
        });

        parse_quote! {
            schema::#job_enum_ident::#variant_ident(job) => Ok(
                [
                    #(#raise_deps_done,)*
                ].concat()
            ),
        }
    });

    parse_quote! {
        #[allow(unused_variables, clippy::match_single_binding)]
        async fn on_receive_job(
            &self,
            client: MSto::Client<'_>,
            job: schema::#job_enum_ident,
        ) -> Result<Vec<Self::Ticket>, #operon::error::SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
            match job {
                #(#job_arms)*
                _ => Err(#operon::error::SchedulerError::InvalidPeerEventReceived("job", #task_id)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::TaskConfigMap;
    use crate::dependency_analysis::get_direct_upstream_tasks;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::all_tasks;

    #[rstest]
    #[case::simple(format_ident!("epsilon"), "spec/spec/fn_on_receive_job.rs")]
    fn test_fn_on_receive_job(
        all_tasks: TaskConfigMap,
        #[case] task_id: syn::Ident,
        #[case] fixture_path: &str,
    ) {
        let task = all_tasks.get(&task_id).unwrap();
        let upstream_tasks = get_direct_upstream_tasks(task, &all_tasks);
        let item = fn_on_receive_job(task, &upstream_tasks);
        assert_item_eq(&item, fixture_path);
    }
}
