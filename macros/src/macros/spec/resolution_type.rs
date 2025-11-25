use syn::parse_quote;

use crate::configs::JobConfig;
use crate::operon_ident;

pub fn resolution_type(job: &JobConfig) -> syn::Type {
    let operon = operon_ident();
    if job.spawn_dim.is_some() {
        let n = job.dims.len();
        parse_quote! { #operon::schema::Resolution<#n> }
    } else {
        parse_quote! { () }
    }
}
