use syn::parse_quote;

use crate::{
    JobConfig,
    utils::{job_ident, mark_done_ident, operon_ident},
};

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
    use quote::ToTokens;
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_impl_job_sql() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec!["a".to_string()],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
        };
        let item = impl_job_sql(&job);
        let expected: syn::ItemImpl = parse_quote! {
            #[operon::async_trait::async_trait]
            #[automatically_derived]
            impl operon::schema_base::JobSql for BetaJob {
                async fn mark_done(
                    &self,
                    client: operon::meta_storage::MetaClient<'_>,
                ) -> Result<(), operon::meta_storage::MetaStorageError> {
                    queries::mark_done_beta(client, self).await
                }
            }
        };
        assert_eq!(
            item.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
