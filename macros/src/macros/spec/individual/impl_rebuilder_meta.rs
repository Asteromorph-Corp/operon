use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{dimension_metadata_ident, operon_ident, rebuilder_ident};

/// Generates the implementation of the `JobSpec` trait for a given job.
pub fn impl_rebuilder_meta(job: &JobConfig) -> syn::ItemImpl {
    let operon = operon_ident();
    let rebuilder_ident = rebuilder_ident(&job.id);

    let maybe_spawn_dim_meta: Option<syn::ImplItemFn> = job.spawn_dim.as_ref().map(|dim| {
        let n = job.dims.len();
        let fn_dim_meta = dimension_metadata_ident(dim);
        parse_quote! {
            pub const fn spawn_dim_meta(&self) -> #operon::schema_base::DimensionMetadata<#n> {
                metadata::#fn_dim_meta()
            }
        }
    });

    parse_quote! {
        impl #rebuilder_ident {
            #maybe_spawn_dim_meta
        }
    }
}
