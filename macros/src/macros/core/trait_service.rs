use quote::ToTokens;
use syn::parse_quote;

use crate::configs::AllConfig;
use crate::utils::{
    entity_ident, entity_over_dim_ident, job_enum_ident, job_fn_ident, operon_ident,
    resolution_enum_ident, service_trait_ident,
};

/// Generates a trait for the service based on the provided AllConfig.
///
/// Example:
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// #[automatically_derived]
/// pub trait CookingService:
///     operon::service::OperonService<JobEnum = schema::JobEnum, ResolutionEnum = schema::ResolutionEnum>
/// {
///     async fn beta(&self, a: A,) -> Result<Vec<B>, Box<dyn std::error::Error + Send + Sync>>;
///     async fn gamma(&self, a: A,) -> Result<Vec<C>, Box<dyn std::error::Error + Send + Sync>>;
///     async fn delta(&self, a: A,b: B, c: C,) -> Result<D, Box<dyn std::error::Error + Send + Sync>>;
///     async fn epsilon(&self, b_j: Vec<B>, d_j: Vec<D>,) -> Result<E, Box<dyn std::error::Error + Send + Sync>>;
///     async fn zeta(&self, c_k: Vec<C>, e_k: Vec<E>,) -> Result<F, Box<dyn std::error::Error + Send + Sync>>;
/// }
/// ```
pub fn trait_service(all_configs: &AllConfig) -> syn::ItemTrait {
    let operon = operon_ident();
    let job_enum_ident = job_enum_ident();
    let res_enum_ident = resolution_enum_ident();
    let svc_ident = service_trait_ident(&all_configs.service_id);

    let (job_fn_strings, job_fns) = all_configs.jobs.values().map(|job| -> (String, syn::TraitItemFn) {
        let fn_name = job_fn_ident(&job.id);
        let args = job.from.iter().map(|arg| -> syn::FnArg {
            let arg_ident = entity_over_dim_ident(&arg.id, &arg.over);
            let entity_ident = entity_ident(&arg.id);
            let ty: syn::Type = arg.over.iter().fold(
                parse_quote! { #entity_ident },
                |acc, _| parse_quote! { Vec<#acc> },
            );

            parse_quote! { #arg_ident: #ty }
        });
        let entity_ident = entity_ident(&job.to);
        let return_ty: syn::Type = job.spawn_dim.as_ref().map_or_else(
            || parse_quote! { #entity_ident },
            |_| parse_quote! { Vec<#entity_ident> },
        );
        (
            format!(
                "async fn {fn_name}(&self, {}) -> Result<{}, Box<dyn std::error::Error + Send + Sync>>;",
                args.clone().map(|arg| arg.to_token_stream().to_string()).collect::<Vec<_>>().join(", "),
                return_ty.to_token_stream().to_string().replace(" ", "")
            ),
            parse_quote! {
                async fn #fn_name(&self, #(#args),*) -> Result<#return_ty, Box<dyn std::error::Error + Send + Sync>>;
            },
        )
    }).unzip::<_, _, Vec<_>, Vec<_>>();

    let doc_comment = format!(
        "# {svc_ident}\n\n\
        This trait defines the functions Operon will call \
        to execute jobs in the service you defined. \
        Implement the functions provided below to define \
        the behaviour of your service.\n\n\
        Notes:\n\n\
        * All functions are async methods and must return a `Result<_, Box<dyn std::error::Error + Send + Sync>>`.\n\
        * Repeated parameters are slices; repeated return types are vectors.\n\
        * The functions in the trait are intended to be stateless. \
          The struct may contain static metadata, but you would need to implement \
          internally-mutable thread-safe states if you really need stateful functions.\n\n\
        Please consult the following section for the exact function signatures.\n\n\
        ## Function Signatures\n\n\
        The functions were parsed as follows:\n\n\
        ```rust,ignore\n\
        use operon::async_trait::async_trait;\n\
        #[async_trait]\n\
        pub trait {svc_ident} {{\n\
        {job_fns}\n\
        }}\n\
        ```",
        job_fns = job_fn_strings
            .iter()
            .map(|fn_body| format!("    {fn_body}"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        #[doc = #doc_comment]
        pub trait #svc_ident: #operon::service::OperonService<JobEnum = schema::#job_enum_ident, ResolutionEnum = schema::#res_enum_ident> {
            #(#job_fns)*
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::simple_pipeline as all_config;

    #[rstest]
    fn test_trait_service(all_config: AllConfig) {
        let result = syn::ItemTrait {
            attrs: vec![],
            ..trait_service(&all_config)
        };
        assert_item_eq(&result, "core/service.rs");
    }
}
