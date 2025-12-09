use indoc::formatdoc;
use syn::parse_quote;

use crate::configs::{AllConfig, JobConfig};
use crate::utils::{
    entity_over_dim_ident, job_enum_ident, operon_ident, resolution_enum_ident,
    service_trait_ident, to_type,
};

fn format_definition(job: &JobConfig) -> String {
    let output_agg = job
        .spawn_dim
        .as_ref()
        .map(|d| format!("<{d}>"))
        .unwrap_or_default();
    let inputs = job
        .from
        .iter()
        .map(|arg| {
            let agg = if arg.over.is_empty() {
                String::new()
            } else {
                let dims = arg.over.iter().map(|d| d.to_string()).collect::<Vec<_>>();
                format!("<{}>", dims.join(", "))
            };
            format!("{}{}", arg.id, agg)
        })
        .collect::<Vec<_>>()
        .join(", ");
    let repeat = if job.dims.is_empty() {
        String::new()
    } else {
        let dims = job.dims.iter().map(|d| d.to_string()).collect::<Vec<_>>();
        format!(" for {}", dims.join(", "))
    };

    format!(
        "{}{} = {}({}){}",
        job.to, output_agg, job.id, inputs, repeat
    )
}

fn format_signature(job: &JobConfig) -> String {
    let operon = operon_ident();
    let args = job
        .from
        .iter()
        .map(|arg| {
            let arg_ident = entity_over_dim_ident(&arg.id, &arg.over);
            let arg_ty =
                (0..arg.over.len()).fold(arg.id.to_string(), |acc, _| format!("Vec<{acc}>"));
            format!("{arg_ident}: {arg_ty}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let return_ty = job
        .spawn_dim
        .as_ref()
        .map_or_else(|| job.to.to_string(), |_| format!("Vec<{}>", job.to));

    format!(
        "async fn {}({}) -> Result<{}, {}::operon::UserError>",
        job.id, args, return_ty, operon
    )
}

/// Generates a trait for the service based on the provided AllConfig.
///
/// # Example
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// #[automatically_derived]
/// pub trait CookingService:
///     operon::service::OperonService<JobEnum = schema::JobEnum, ResolutionEnum = schema::ResolutionEnum>
/// {
///     async fn alpha(&self) -> Result<Vec<A>, operon::operon::UserError>;
///     async fn beta(&self, a: A) -> Result<Vec<B>, operon::operon::UserError>;
///     async fn gamma(&self, a: A) -> Result<Vec<C>, operon::operon::UserError>;
///     async fn delta(&self, a: A, b: B, c: C) -> Result<D, operon::operon::UserError>;
///     async fn epsilon(&self, b_j: Vec<B>, d_j: Vec<D>) -> Result<E, operon::operon::UserError>;
///     async fn zeta(&self, c_k: Vec<C>, e_k: Vec<E>) -> Result<F, operon::operon::UserError>;
/// }
/// ```
pub fn trait_service(all_configs: &AllConfig) -> syn::ItemTrait {
    let operon = operon_ident();
    let job_enum_ident = job_enum_ident();
    let res_enum_ident = resolution_enum_ident();
    let svc_ident = service_trait_ident(&all_configs.service_id);

    let (job_sigs, job_fns) = all_configs.jobs.values().map(|job| -> (String, syn::TraitItemFn) {
        let fn_name = &job.id;
        let args = job.from.iter().map(|arg| -> syn::FnArg {
            let arg_ident = entity_over_dim_ident(&arg.id, &arg.over);
            let arg_ty: syn::Type = (0..arg.over.len()).fold(
                to_type(&arg.id),
                |acc, _| parse_quote! { Vec<#acc> },
            );
            parse_quote! { #arg_ident: #arg_ty }
        }).collect::<Vec<_>>();
        let return_ty: syn::Type = job.spawn_dim.as_ref().map_or_else(
            || to_type(&job.to),
            |_| {
                let unit_ty = to_type(&job.to);
                parse_quote! { Vec<#unit_ty> }
            },
        );

        let def = format_definition(job);
        let sig = format_signature(job);
        let doc = formatdoc! {"
            ```rust,ignore
            {sig}
            ```
            Corresponds to the task:
            ```rust,ignore
            {def}
            ```",
        };

        (
            sig,
            parse_quote! {
                #[doc = #doc]
                async fn #fn_name(&self, #(#args),*) -> Result<#return_ty, #operon::operon::UserError>;
            },
        )
    }).unzip::<_, _, Vec<_>, Vec<_>>();

    let doc_comment = formatdoc! {"
        Generated trait containing task methods that should be implemented for use with Operon.

        # Methods
        ```rust,ignore
        {}
        ```",
        job_sigs.join("\n")
    };

    parse_quote! {
        #[#operon::async_trait::async_trait]
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
