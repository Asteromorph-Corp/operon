use syn::parse_quote;

use crate::configs::JobConfig;
use crate::operon_ident;

pub fn fn_default_ticket(job: &JobConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let n = job.dims.len();
    let initial_quota = job.from.len();

    parse_quote! {
        fn default_ticket(&self) -> #operon::schema_base::Ticket<#n> {
            #operon::schema_base::Ticket::new(#initial_quota)
        }
    }
}
