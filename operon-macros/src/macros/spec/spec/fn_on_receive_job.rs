use indexmap::IndexSet;
use quote::format_ident;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{job_enum_ident, job_metadata_ident, operon_ident, to_lit_str, to_pascal_case};

/// Generates the `on_receive_job` function for the implementation of the trait `JobSpec`.
///
/// # Example
/// ```rust,ignore
/// #[allow(unused_variables, clippy::match_single_binding)]
/// async fn on_receive_job(
///     &self,
///     client: MSto::Client<'_>,
///     job: schema::JobEnum,
/// ) -> Result<Vec<Self::Ticket>, operon::error::SchedulerError<MSto::Error>> {
///     match job {
///         schema::JobEnum::Beta(job) => Ok([client
///             .ticket(self.job_meta())
///             .raise_deps_done(metadata::job_beta_meta(), job, &["j"])
///             .await?]
///         .concat()),
///         schema::JobEnum::Delta(job) => Ok([client
///             .ticket(self.job_meta())
///             .raise_deps_done(metadata::job_delta_meta(), job, &["j"])
///             .await?]
///         .concat()),
///         _ => Err(operon::error::SchedulerError::InvalidPeerEventReceived(
///             "job", "epsilon",
///         )),
///     }
/// }
/// ```
pub(super) fn fn_on_receive_job(
    job: &JobConfig,
    upstream_jobs: &IndexSet<&JobConfig>,
) -> syn::ImplItemFn {
    let operon = operon_ident();
    let job_enum_ident = job_enum_ident();
    let job_id = to_lit_str(&job.id);

    let job_arms = upstream_jobs.iter().map(|upstream_job| -> syn::Arm {
        let upstream_job_meta = job_metadata_ident(&upstream_job.id);
        let variant_ident = to_pascal_case(&format_ident!("{}", upstream_job.id));
        let affected_args = job.from.iter().filter(|arg| arg.id == upstream_job.to);

        let raise_deps_done = affected_args.map(|arg| -> syn::Expr {
            let aggregate_dims = arg.over.iter().map(to_lit_str);
            parse_quote! {
                client
                    .ticket(self.job_meta())
                    .raise_deps_done(metadata::#upstream_job_meta(), job, &[#(#aggregate_dims),*])
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
        ) -> Result<Vec<Self::Ticket>, #operon::error::SchedulerError<MSto::Error>> {
            match job {
                #(#job_arms)*
                _ => Err(#operon::error::SchedulerError::InvalidPeerEventReceived("job", #job_id)),
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
    #[case::simple(format_ident!("epsilon"), "spec/spec/fn_on_receive_job.rs")]
    fn test_fn_on_receive_job(
        all_jobs: JobConfigMap,
        #[case] job_id: syn::Ident,
        #[case] fixture_path: &str,
    ) {
        let job = all_jobs.get(&job_id).unwrap();
        let upstream_jobs = get_direct_upstream_jobs(job, &all_jobs);
        let item = fn_on_receive_job(job, &upstream_jobs);
        assert_item_eq(&item, fixture_path);
    }
}
