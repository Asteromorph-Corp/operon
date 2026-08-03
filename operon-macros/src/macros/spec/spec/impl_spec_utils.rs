use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{
    dimension_metadata_ident, job_metadata_ident, operon_ident, service_trait_ident, spec_ident,
    storage_trait_ident,
};

/// Generates the implementation of utility function for a spec struct of given job.
///
/// # Example
/// ```rust,ignore
/// impl BetaSpec {
///     pub const fn job_meta(&self) -> operon::__private::JobMetadata<1usize> {
///         metadata::job_beta_meta()
///     }
///
///     pub const fn spawn_dim_meta(&self) -> operon::__private::DimensionMetadata<1usize> {
///         metadata::dimension_j_meta()
///     }
///
///     pub fn all_upstream_tasks(&self) -> std::collections::HashSet<&'static str> {
///         std::collections::HashSet::from_iter(["alpha", "beta"])
///     }
///
///     pub fn into_handler<Svc: CookingService, Sto: CookingStorage, MSto: operon::__private::MetaBackend>(
///         self,
///     ) -> Box<dyn operon::__private::TaskHandler<Svc, Sto, MSto>> {
///         let job_meta = self.job_meta();
///         let all_upstream_tasks = self.all_upstream_tasks();
///         Box::new(operon::__private::SpecWithMetadata::new(
///             self,
///             job_meta,
///             all_upstream_tasks,
///         ))
///     }
/// }
/// ```
pub fn impl_spec_utils(service_id: &syn::Ident, job: &JobConfig) -> syn::ItemImpl {
    let operon = operon_ident();
    let spec_ident = spec_ident(&job.id);

    let svc_ident = service_trait_ident(service_id);
    let sto_ident = storage_trait_ident(service_id);

    let n = job.dims.len();
    let fn_job_meta = job_metadata_ident(&job.id);

    let maybe_spawn_dim_meta: Option<syn::ImplItemFn> = job.spawn_dim.as_ref().map(|dim| {
        let fn_dim_meta = dimension_metadata_ident(dim);
        parse_quote! {
            pub const fn spawn_dim_meta(&self) -> #operon::__private::DimensionMetadata<#n> {
                metadata::#fn_dim_meta()
            }
        }
    });

    parse_quote! {
        impl #spec_ident {
            pub const fn job_meta(&self) -> #operon::__private::JobMetadata<#n> {
                metadata::#fn_job_meta()
            }
            #maybe_spawn_dim_meta

            pub fn into_handler<Svc: #svc_ident, Sto: #sto_ident, MSto: #operon::__private::MetaBackend>(self) -> Box<dyn #operon::__private::TaskHandler<Svc, Sto, MSto>> {
                let job_meta = self.job_meta();
                Box::new(#operon::__private::SpecWithMetadata::new(self, job_meta))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{job_beta, job_epsilon, service_id};

    #[rstest]
    #[case::simple(job_beta(), "spec/spec/impl_spec_utils.simple.rs")]
    #[case::no_spawn_dim(job_epsilon(), "spec/spec/impl_spec_utils.no_spawn_dim.rs")]
    fn test_impl_spec_utils(
        service_id: syn::Ident,
        #[case] job: JobConfig,
        #[case] fixture_path: &str,
    ) {
        let item = impl_spec_utils(&service_id, &job);
        assert_item_eq(&item, fixture_path)
    }
}
