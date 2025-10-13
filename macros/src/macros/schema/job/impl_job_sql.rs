use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{job_ident, mark_done_ident, operon_ident};

/// Generates an implementation of the `JobSql` trait for the given job.
///
/// Example:
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// #[automatically_derived]
/// impl operon::schema_base::JobSql for BetaJob {
///     async fn mark_done(
///        &self,
///        client: operon::meta_storage::MetaClient<'_>,
///     ) -> Result<(), operon::meta_storage::MetaStorageError> {
///         queries::mark_done_beta(client, self).await
///     }
/// }
/// ```
pub(super) fn impl_job_sql(job: &JobConfig) -> syn::ItemImpl {
    let operon = operon_ident();
    let job_ident = job_ident(&job.id);

    let mark_done_fn_name = mark_done_ident(&job.id);

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::schema_base::JobSql for #job_ident {
            async fn mark_done(
                &self,
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::#mark_done_fn_name(client, self).await
            }
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
    #[case::simple(job_beta(), "schema/job/impl_job_sql.rs")]
    fn test_impl_job_sql(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let result = impl_job_sql(&job);
        assert_item_eq(&result, fixture_path);
    }
}
