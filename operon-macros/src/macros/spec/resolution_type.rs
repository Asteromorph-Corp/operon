use syn::parse_quote;

use crate::configs::TaskConfig;
use crate::operon_ident;

pub(super) fn resolution_type(task: &TaskConfig) -> syn::Type {
    let operon = operon_ident();
    if task.spawn_dim.is_some() {
        let n = task.dims.len();
        parse_quote! { #operon::__private::Resolution<#n> }
    } else {
        parse_quote! { () }
    }
}
