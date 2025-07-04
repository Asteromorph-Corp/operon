use syn::parse_quote;

use crate::{
    configs::JobConfig,
    utils::{job_field_ident, job_ident, mark_done_ident, operon_ident},
};

/// An helper struct to generate the SQL query for marking a ticket as done for a given job.
struct MarkDoneQuery<'a>(&'a JobConfig);

impl std::fmt::Display for MarkDoneQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UPDATE {{schema_prefix}}ticket_{} SET status = 'done' WHERE",
            self.0.id
        )?;
        for (i, dim) in self.0.dims.iter().enumerate() {
            if i != 0 {
                write!(f, " AND")?;
            }
            write!(f, " {dim} = ${}", i + 1)?;
        }
        write!(f, ";")
    }
}

/// Generates the `mark_done_*` function for a given job.
///
/// Example:
/// ```rust,ignore
/// pub async fn mark_done_beta(
///     client: operon::meta_storage::MetaClient<'_>,
///     job: &schema::BetaJob,
/// ) -> Result<(), operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     let stmt = format!("UPDATE {schema_prefix}ticket_beta SET status = 'done' WHERE i = $1 AND j = $2;");
///     client.execute(&stmt, &[&i64::try_from(job.i)?, &i64::try_from(job.j)?]).await?;
///     Ok(())
/// }
/// ```
pub(super) fn fn_mark_done(job: &JobConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = mark_done_ident(&job.id);
    let job_ident = job_ident(&job.id);
    let stmt = MarkDoneQuery(job).to_string();

    let dims = job.dims.iter().map(job_field_ident);

    parse_quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
            job: &schema::#job_ident,
        ) -> Result<(), #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(#stmt);
            client.execute(&stmt, &[#(&i64::try_from(job.#dims)?),*]).await?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::*;

    #[test]
    fn test_mark_done_query() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec!["a".to_string()],
            to: "b".to_string(),
            dims: vec!["i".to_string(), "j".to_string()],
        };
        let query = MarkDoneQuery(&job).to_string();
        assert_eq!(
            query,
            "UPDATE {schema_prefix}ticket_beta SET status = 'done' WHERE i = $1 AND j = $2;"
        );
    }

    #[test]
    fn test_fn_mark_done() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec!["a".to_string()],
            to: "b".to_string(),
            dims: vec!["i".to_string(), "j".to_string()],
        };
        let tokens = fn_mark_done(&job);
        let expected: syn::ItemFn = parse_quote! {
            pub async fn mark_done_beta(
                client: operon::meta_storage::MetaClient<'_>,
                job: &schema::BetaJob,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                let stmt = format!("UPDATE {schema_prefix}ticket_beta SET status = 'done' WHERE i = $1 AND j = $2;");
                client.execute(&stmt, &[&i64::try_from(job.i)?, &i64::try_from(job.j)?]).await?;
                Ok(())
            }
        };
        assert_eq!(
            tokens.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
