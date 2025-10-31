use quote::quote;
use syn::parse_quote;

use crate::configs::{DimensionId, JobConfigMap};
use crate::macros::schema::ticket::impl_ticket::impl_ticket;
use crate::macros::schema::ticket::impl_ticket_sql::impl_ticket_sql;
use crate::macros::schema::ticket::impl_with::impl_with_fns;
use crate::macros::schema::ticket::ticket_definition::ticket_definition;

/// Generates the `mod ticket` module with all ticket-related items.
pub fn mod_ticket(jobs: &JobConfigMap, primary_entity: &DimensionId) -> syn::ItemMod {
    let jobs = jobs.values().map(|job| {
        let def = ticket_definition(job);
        let impl_with_fns = impl_with_fns(job);
        let impl_ticket = impl_ticket(job, primary_entity);
        let impl_ticket_sql = impl_ticket_sql(job);

        quote! {
            #def
            #impl_with_fns
            #impl_ticket
            #impl_ticket_sql
        }
    });

    parse_quote! {
        mod ticket {
            use super::*;

            #(#jobs)*
        }
    }
}
