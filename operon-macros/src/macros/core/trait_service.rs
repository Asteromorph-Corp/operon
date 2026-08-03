use indoc::formatdoc;
use syn::parse_quote;

use crate::configs::{AllConfig, TaskConfig};
use crate::macros::core::DocumentedFn;
use crate::utils::{
    entity_over_dim_ident, job_enum_ident, operon_ident, resolution_enum_ident,
    service_trait_ident_spanned, ticket_enum_ident, to_type,
};

fn format_definition(task: &TaskConfig) -> String {
    let output_agg = task
        .spawn_dim
        .as_ref()
        .map(|d| format!("<{d}>"))
        .unwrap_or_default();
    let inputs = task
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
    let repeat = if task.dims.is_empty() {
        String::new()
    } else {
        let dims = task.dims.iter().map(|d| d.to_string()).collect::<Vec<_>>();
        format!(" for {}", dims.join(", "))
    };

    format!(
        "{}{} = {}({}){}",
        task.to, output_agg, task.id, inputs, repeat
    )
}

fn format_signature(task: &TaskConfig) -> String {
    let args = std::iter::once("&self".to_owned())
        .chain(task.from.iter().map(|arg| {
            let arg_ident = entity_over_dim_ident(&arg.id, &arg.over);
            let arg_ty =
                (0..arg.over.len()).fold(arg.id.to_string(), |acc, _| format!("Vec<{acc}>"));
            format!("{arg_ident}: {arg_ty}")
        }))
        .collect::<Vec<_>>()
        .join(", ");
    let return_ty = task
        .spawn_dim
        .as_ref()
        .map_or_else(|| task.to.to_string(), |_| format!("Vec<{}>", task.to));

    format!(
        "async fn {}({}) -> Result<{}, Self::Error>",
        task.id, args, return_ty
    )
}

/// Generates a trait for the service based on the provided AllConfig.
///
/// # Example
/// ```rust,ignore
/// #[operon::__private::async_trait::async_trait]
/// pub trait CookingService:
///     operon::OperonService<
///         JobEnum = schema::JobEnum,
///         ResolutionEnum = schema::ResolutionEnum,
///         TicketEnum = schema::TicketEnum,
///     >
/// {
///     async fn alpha(&self) -> Result<Vec<A>, Self::Error>;
///     async fn beta(&self, a: A) -> Result<Vec<B>, Self::Error>;
///     async fn gamma(&self, a: A) -> Result<Vec<C>, Self::Error>;
///     async fn delta(&self, a: A, b: B, c: C) -> Result<D, Self::Error>;
///     async fn epsilon(&self, b_j: Vec<B>, d_j: Vec<D>) -> Result<E, Self::Error>;
///     async fn zeta(&self, c_k: Vec<C>, e_k: Vec<E>) -> Result<F, Self::Error>;
/// }
/// ```
pub fn trait_service(all_configs: &AllConfig) -> syn::ItemTrait {
    let operon = operon_ident();
    let job_enum_ident = job_enum_ident();
    let res_enum_ident = resolution_enum_ident();
    let ticket_enum_ident = ticket_enum_ident();
    let svc_ident = service_trait_ident_spanned(&all_configs.service_id);

    let (task_sigs, task_fns) = all_configs
        .tasks
        .values()
        .map(|task| -> DocumentedFn {
            let fn_name = &task.id;
            let args = task
                .from
                .iter()
                .map(|arg| -> syn::FnArg {
                    let arg_ident = entity_over_dim_ident(&arg.id, &arg.over);
                    let arg_ty: syn::Type = (0..arg.over.len())
                        .fold(to_type(&arg.id), |acc, _| parse_quote! { Vec<#acc> });
                    parse_quote! { #arg_ident: #arg_ty }
                })
                .collect::<Vec<_>>();
            let return_ty: syn::Type = task.spawn_dim.as_ref().map_or_else(
                || to_type(&task.to),
                |_| {
                    let unit_ty = to_type(&task.to);
                    parse_quote! { Vec<#unit_ty> }
                },
            );

            let def = format_definition(task);
            let sig = format_signature(task);
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
                    #[allow(clippy::too_many_arguments)]
                    async fn #fn_name(&self, #(#args),*) -> Result<#return_ty, Self::Error>;
                },
            )
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let doc_comment = formatdoc! {"
        Generated trait containing task methods that should be implemented for use with Operon.

        Each task method returns `Self::Error`, the service's error type.
        `#[derive(OperonService)]` defaults it to `operon::error::UserError`.
        Select a concrete type with `#[operon(error = MyError)]` on the derive.

        # Methods
        ```rust,ignore
        {}
        ```",
        task_sigs.join("\n")
    };

    parse_quote! {
        #[#operon::__private::async_trait::async_trait]
        #[doc = #doc_comment]
        pub trait #svc_ident: #operon::OperonService<
            JobEnum = schema::#job_enum_ident,
            ResolutionEnum = schema::#res_enum_ident,
            TicketEnum = schema::#ticket_enum_ident
        > {
            #(#task_fns)*
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
        let mut result = trait_service(&all_config);
        result.attrs.retain(|attr| attr.path().is_ident("doc"));
        assert_item_eq(&result, "core/service.rs");
    }
}
