use syn::parse_quote;

use crate::{
    JobConfig,
    configs::DimensionId,
    utils::{dimension_ident, operon_ident, ticket_ident, variable_ident, with_ident},
};

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
    use crate::configs::JobArg;

    use super::*;

    #[test]
    fn test_fn_with() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
            pool_size: 8,
        };
        let dim = DimensionId::from("i");
        let item = fn_with(&job, &dim);
        let expected: syn::ImplItemFn = parse_quote! {
            pub fn with_i(self, i: IDim) -> Self {
                let mut new = BetaTicket {
                    i: i.into(),
                    ..self
                };
                if operon::schema_base::Ticket::is_ready(&new) {
                    new.status = operon::schema_base::TicketStatus::Queued;
                }
                new
            }
        };
        assert_eq!(item, expected);
    }

    #[test]
    fn test_impl_with_fns() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
            pool_size: 8,
        };
        let item = impl_with_fns(&job);
        let expected: syn::ItemImpl = parse_quote! {
            impl BetaTicket {
                pub fn with_i(self, i: IDim) -> Self {
                    let mut new = BetaTicket {
                        i: i.into(),
                        ..self
                    };

                    if operon::schema_base::Ticket::is_ready(&new) {
                        new.status = operon::schema_base::TicketStatus::Queued;
                    }

                    new
                }
            }
        };
        assert_eq!(item, expected);
    }
}
