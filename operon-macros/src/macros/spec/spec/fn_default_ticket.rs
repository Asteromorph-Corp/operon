use syn::parse_quote;

use crate::configs::JobConfig;
use crate::operon_ident;

/// Generates the `default_ticket` function for the implementation of the trait `JobSpec`.
///
/// # Example
/// ```rust,ignore
/// fn default_ticket(&self) -> operon::schema::Ticket<1usize> {
///     operon::schema::Ticket::new(1usize)
/// }
/// ```
pub fn fn_default_ticket(job: &JobConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let n = job.dims.len();
    let initial_quota = job.from.len();

    parse_quote! {
        fn default_ticket(&self) -> #operon::schema::Ticket<#n> {
            #operon::schema::Ticket::new(#initial_quota)
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
    #[case::simple(job_beta(), "spec/spec/fn_default_ticket.rs")]
    fn test_fn_default_ticket(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let item = fn_default_ticket(&job);
        assert_item_eq(&item, fixture_path)
    }
}
