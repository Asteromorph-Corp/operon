use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{
    dimension_metadata_ident, job_metadata_ident, operon_ident, service_trait_ident, spec_ident,
    storage_trait_ident,
};

/// Generates the implementation of the `JobSpec` trait for a given job.
pub fn impl_meta(service_id: &str, job: &JobConfig) -> syn::ItemImpl {
    let operon = operon_ident();
    let spec_ident = spec_ident(&job.id);

    let svc_ident = service_trait_ident(service_id);
    let sto_ident = storage_trait_ident(service_id);

    let n = job.dims.len();
    let fn_job_meta = job_metadata_ident(&job.id);

    let maybe_spawn_dim_meta: Option<syn::ImplItemFn> = job.spawn_dim.as_ref().map(|dim| {
        let fn_dim_meta = dimension_metadata_ident(dim);
        parse_quote! {
            pub const fn spawn_dim_meta(&self) -> #operon::schema_base::DimensionMetadata<#n> {
                metadata::#fn_dim_meta()
            }
        }
    });

    parse_quote! {
        impl #spec_ident {
            pub const fn job_meta(&self) -> #operon::schema_base::JobMetadata<#n> {
                metadata::#fn_job_meta()
            }
            #maybe_spawn_dim_meta


            pub fn into_handler<Svc: #svc_ident, Sto: #sto_ident>(self) -> Box<dyn #operon::scheduler::JobHandler<Svc, Sto>> {
                let job_meta = self.job_meta();
                Box::new(
                    #operon::scheduler::SpecWithMetadata::new(self, job_meta)
                )
            }
        }
    }
}
