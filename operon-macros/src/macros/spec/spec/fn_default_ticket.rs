use syn::parse_quote;

use crate::configs::TaskConfig;
use crate::operon_ident;

/// Generates the `default_ticket` function for the implementation of the trait `TaskSpec`.
///
/// # Example
/// ```rust,ignore
/// fn default_ticket(&self) -> operon::__private::Ticket<1usize> {
///     operon::__private::Ticket::new(1usize)
/// }
/// ```
pub fn fn_default_ticket(task: &TaskConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let n = task.dims.len();
    let initial_quota = task.from.len();

    parse_quote! {
        fn default_ticket(&self) -> #operon::__private::Ticket<#n> {
            #operon::__private::Ticket::new(#initial_quota)
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::task_beta;

    #[rstest]
    #[case::simple(task_beta(), "spec/spec/fn_default_ticket.rs")]
    fn test_fn_default_ticket(#[case] task: TaskConfig, #[case] fixture_path: &str) {
        let item = fn_default_ticket(&task);
        assert_item_eq(&item, fixture_path)
    }
}
