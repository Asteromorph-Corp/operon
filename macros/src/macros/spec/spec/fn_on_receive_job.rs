use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{job_enum_ident, job_metadata_ident, operon_ident, variant_ident};

/// Generates the `on_receive_job` function for the implementation of the trait `JobSpec`.
///
/// Example:
/// ```rust,ignore
/// #[allow(unused_variables, clippy::match_single_binding)]
/// async fn on_receive_job(
///     &self,
///     client: operon::meta_storage::MetaClient<'_>,
///     job: schema::JobEnum,
/// ) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
///     match job {
///         schema::JobEnum::Beta(job) => Ok(
///             queries::raise_dep_epsilon(
///                 client,
///                 &operon::schema::OptionCoordinate::some(job.i),
///                 &operon::schema::OptionCoordinate::none(),
///             )
///             .await?
///         ),
///         schema::JobEnum::Delta(job) => Ok(
///             queries::raise_dep_epsilon(
///                 client,
///                 &operon::schema::OptionCoordinate::some(job.i),
///                 &operon::schema::OptionCoordinate::some(job.k),
///             )
///             .await?
///         ),
///         _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived("job", "epsilon")),
///     }
/// }
/// ```
pub(super) fn fn_on_receive_job(
    job: &JobConfig,
    upstream_jobs: &IndexSet<&JobConfig>,
) -> syn::ImplItemFn {
    let operon = operon_ident();
    let job_enum_ident = job_enum_ident();
    let job_id = &job.id;

    let job_arms = upstream_jobs.iter().map(|upstream_job| -> syn::Arm {
        let upstream_job_meta = job_metadata_ident(&upstream_job.id);
        let variant_ident = variant_ident(&upstream_job.id);
        let affected_args = job.from.iter().filter(|arg| arg.id == upstream_job.to);

        let raise_deps_done = affected_args.map(|arg| -> syn::Expr {
            let aggregate_dims = &arg.over;
            parse_quote! {
                client.ticket(self.job_meta()).raise_deps_done(metadata::#upstream_job_meta(), job, &[#(#aggregate_dims),*]).await?
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
            client: #operon::meta_storage::MetaClient<'_>,
            job: schema::#job_enum_ident,
        ) -> Result<Vec<Self::Ticket>, #operon::scheduler::SchedulerError> {
            match job {
                #(#job_arms)*
                _ => Err(#operon::scheduler::SchedulerError::InvalidPeerEventReceived("job", #job_id)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::JobConfigMap;
    use crate::dependency_analysis::get_direct_upstream_jobs;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::all_jobs;

    #[rstest]
    #[case::simple("epsilon", "spec/spec/fn_on_receive_job.rs")]
    fn test_fn_on_receive_job(
        all_jobs: JobConfigMap,
        #[case] job_id: &str,
        #[case] fixture_path: &str,
    ) {
        let job = all_jobs.get(job_id).unwrap();
        let upstream_jobs = get_direct_upstream_jobs(job, &all_jobs);
        let item = fn_on_receive_job(job, &upstream_jobs);
        assert_item_eq(&item, fixture_path);
    }
}
