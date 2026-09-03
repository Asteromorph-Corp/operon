use syn::parse_quote;

use crate::configs::TaskConfig;
use crate::utils::{
    dimension_metadata_ident, operon_ident, service_trait_ident, spec_ident, storage_trait_ident,
    task_metadata_ident,
};

/// Generates an implementation of utility functions for a task spec.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/spec/spec/impl_spec_utils.simple.rs") )]
/// ```
pub fn impl_spec_utils(service_id: &syn::Ident, task: &TaskConfig) -> syn::ItemImpl {
    let operon = operon_ident();
    let spec_ident = spec_ident(&task.id);

    let svc_ident = service_trait_ident(service_id);
    let sto_ident = storage_trait_ident(service_id);

    let n = task.dims.len();
    let fn_task_meta = task_metadata_ident(&task.id);

    let maybe_spawn_dim_meta: Option<syn::ImplItemFn> = task.spawn_dim.as_ref().map(|dim| {
        let fn_dim_meta = dimension_metadata_ident(dim);
        parse_quote! {
            pub const fn spawn_dim_meta(&self) -> #operon::__private::DimensionMetadata<#n> {
                metadata::#fn_dim_meta()
            }
        }
    });

    parse_quote! {
        impl #spec_ident {
            pub const fn task_meta(&self) -> #operon::__private::TaskMetadata<#n> {
                metadata::#fn_task_meta()
            }
            #maybe_spawn_dim_meta

            pub fn into_handler<Svc: #svc_ident, Sto: #sto_ident, MSto: #operon::__private::MetaBackend>(self) -> Box<dyn #operon::__private::TaskHandler<Svc, Sto, MSto>> {
                let task_meta = self.task_meta();
                Box::new(#operon::__private::SpecWithMetadata::new(self, task_meta))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{service_id, task_beta, task_epsilon};

    #[rstest]
    #[case::simple(task_beta(), "spec/spec/impl_spec_utils.simple.rs")]
    #[case::no_spawn_dim(task_epsilon(), "spec/spec/impl_spec_utils.no_spawn_dim.rs")]
    fn test_impl_spec_utils(
        service_id: syn::Ident,
        #[case] task: TaskConfig,
        #[case] fixture_path: &str,
    ) {
        let item = impl_spec_utils(&service_id, &task);
        assert_item_eq(&item, fixture_path)
    }
}
