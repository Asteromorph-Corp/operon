use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{job_ident, mark_done_ident, operon_ident, variable_ident};

/// An helper struct to generate the SQL query for marking a ticket as done for a given job.
struct MarkDoneQuery<'a>(&'a JobConfig);

impl std::fmt::Display for MarkDoneQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UPDATE {{schema_prefix}}ticket_{} SET status = 'done'",
            self.0.id
        )?;
        for (idx, dim) in self.0.dims.iter().enumerate() {
            if idx == 0 {
                write!(f, " WHERE")?;
            } else {
                write!(f, " AND")?;
            }
            write!(f, " {dim} = ${}", idx + 1)?;
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
///     let stmt = format!("UPDATE {schema_prefix}ticket_beta SET status = 'done' WHERE i = $1;");
///     client.execute(&stmt, &[&i64::try_from(job.i)?]).await?;
///     Ok(())
/// }
/// ```
pub(super) fn fn_mark_done(job: &JobConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = mark_done_ident(&job.id);
    let job_ident = job_ident(&job.id);
    let stmt = MarkDoneQuery(job).to_string();

    let dims = job.dims.iter().map(|d| variable_ident(d));

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
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{job_beta, job_epsilon};

    #[rstest]
    #[case::simple(
        job_beta(),
        "UPDATE {schema_prefix}ticket_beta SET status = 'done' WHERE i = $1;"
    )]
    fn test_mark_done_query(#[case] job: JobConfig, #[case] expected: &str) {
        let stmt = MarkDoneQuery(&job).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(job_beta(), "queries/ticket/mark_done.simple.rs")]
    #[case::multipe_dims(job_epsilon(), "queries/ticket/mark_done.multiple_dims.rs")]
    fn test_fn_mark_done(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let result = fn_mark_done(&job);
        assert_item_eq(&result, fixture_path);
    }
}
