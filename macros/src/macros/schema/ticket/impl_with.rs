use syn::parse_quote;

use crate::JobConfig;
use crate::configs::DimensionId;
use crate::utils::{dimension_ident, operon_ident, ticket_ident, variable_ident, with_ident};

fn fn_with(job: &JobConfig, dim: &DimensionId) -> syn::ImplItemFn {
    let operon = operon_ident();
    let fn_name = with_ident(dim);
    let ticket_ident = ticket_ident(&job.id);
    let dim_ident = dimension_ident(dim);
    let arg = variable_ident(dim);

    parse_quote! {
        pub fn #fn_name(self, #arg: #dim_ident) -> Self {
            let mut new = #ticket_ident {
                #arg: #arg.into(),
                ..self
            };

            if #operon::schema_base::Ticket::is_ready(&new) {
                new.status = #operon::schema_base::TicketStatus::Queued;
            }

            new
        }
    }
}

/// Generates the impl block with `with_*` methods for a ticket.
pub(super) fn impl_with_fns(job: &JobConfig) -> syn::ItemImpl {
    let ticket_ident = ticket_ident(&job.id);
    let with_fns = job
        .dims
        .iter()
        .map(|dim| fn_with(job, dim))
        .collect::<Vec<_>>();

    parse_quote! {
        impl #ticket_ident {
            #(#with_fns)*
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{job_beta, job_delta};

    #[rstest]
    #[case(job_beta(), DimensionId::from("i"), "schema/ticket/fn_with.rs")]
    fn test_fn_with(#[case] job: JobConfig, #[case] dim: DimensionId, #[case] fixture_path: &str) {
        let result = fn_with(&job, &dim);
        assert_item_eq(&result, fixture_path);
    }

    #[rstest]
    #[case(job_beta(), "schema/ticket/impl_with_fns.simple.rs")]
    #[case(job_delta(), "schema/ticket/impl_with_fns.with_dependency.rs")]
    fn test_impl_with_fns(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let result = impl_with_fns(&job);
        assert_item_eq(&result, fixture_path);
    }
}
