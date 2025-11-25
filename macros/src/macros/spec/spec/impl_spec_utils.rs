use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{
    dimension_metadata_ident, job_metadata_ident, operon_ident, service_trait_ident, spec_ident,
    storage_trait_ident,
};

/// Generates the implementation of the `JobSpec` trait for a given job.
pub fn impl_spec_utils(
    service_id: &str,
    job: &JobConfig,
    all_upstream_jobs: &IndexSet<&JobConfig>,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let spec_ident = spec_ident(&job.id);

    let svc_ident = service_trait_ident(service_id);
    let sto_ident = storage_trait_ident(service_id);

    let n = job.dims.len();
    let fn_job_meta = job_metadata_ident(&job.id);

    let all_upstream_job_ids = all_upstream_jobs.iter().map(|j| &j.id);
    let maybe_spawn_dim_meta: Option<syn::ImplItemFn> = job.spawn_dim.as_ref().map(|dim| {
        let fn_dim_meta = dimension_metadata_ident(dim);
        parse_quote! {
            pub const fn spawn_dim_meta(&self) -> #operon::schema::DimensionMetadata<#n> {
                metadata::#fn_dim_meta()
            }
        }
    });

    parse_quote! {
        impl #spec_ident {
            pub const fn job_meta(&self) -> #operon::schema::JobMetadata<#n> {
                metadata::#fn_job_meta()
            }
            #maybe_spawn_dim_meta

            pub fn all_upstream_jobs(&self) -> std::collections::HashSet<&'static str> {
                std::collections::HashSet::from_iter([
                    #(#all_upstream_job_ids,)*
                ])
            }


            pub fn into_handler<Svc: #svc_ident, Sto: #sto_ident>(self) -> Box<dyn #operon::scheduler::JobHandler<Svc, Sto>> {
                let job_meta = self.job_meta();
                let all_upstream_jobs = self.all_upstream_jobs();

                Box::new(#operon::scheduler::SpecWithMetadata::new(
                    self,
                    job_meta,
                    all_upstream_jobs,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::JobConfigMap;
    use crate::dependency_analysis::get_upstream_jobs;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_jobs, job_beta, job_epsilon, service_id};

    #[rstest]
    #[case::simple(job_beta(), "spec/spec/impl_spec_utils.simple.rs")]
    #[case::no_spawn_dim(job_epsilon(), "spec/spec/impl_spec_utils.no_spawn_dim.rs")]
    fn test_impl_spec_utils(
        service_id: &str,
        all_jobs: JobConfigMap,
        #[case] job: JobConfig,
        #[case] fixture_path: &str,
    ) {
        let all_upstream_jobs = get_upstream_jobs(&job, &all_jobs);
        let item = impl_spec_utils(service_id, &job, &all_upstream_jobs);
        assert_item_eq(&item, fixture_path)
    }
}
