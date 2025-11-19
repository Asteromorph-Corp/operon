use syn::parse_quote;

use crate::configs::JobConfig;
use crate::operon_ident;
use crate::utils::job_metadata_ident;

pub fn job_metadata(job: &JobConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = job_metadata_ident(&job.id);

    let n = job.dims.len();
    let id = &job.id;
    let dims = &job.dims;
    let spawn_dim: syn::Expr = match &job.spawn_dim {
        Some(spawn_dim) => parse_quote! { Some(#spawn_dim) },
        None => parse_quote! { None },
    };

    parse_quote! {
        pub const fn #fn_name() -> #operon::schema_base::JobMetadata<#n> {
            #operon::schema_base::JobMetadata {
                id: #id,
                dims: [#(#dims),*],
                spawn_dim: #spawn_dim,
            }
        }
    }
}
