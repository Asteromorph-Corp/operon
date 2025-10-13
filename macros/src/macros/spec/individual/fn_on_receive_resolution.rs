use syn::parse_quote;

use crate::configs::{DimensionConfigMap, JobConfig};
use crate::dependency_analysis::get_quota_required_dims;
use crate::utils::{
    explode_ident, operon_ident, resolution_enum_ident, resolve_dep_ident, variant_ident,
};

/// Generates the `on_receive_resolution` function for the implementation of the trait `JobSpec`.
///
/// Example:
/// ```rust, ignore
/// #[allow(unused_variables, clippy::match_single_binding)]
/// async fn on_receive_resolution(
///     &self,
///     client: operon::meta_storage::MetaClient<'_>,
///     resolution: schema::ResolutionEnum,
/// ) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
///     match resolution {
///         schema::ResolutionEnum::I(res) => Ok(
///             queries::explode_delta_i(client, &res).await?
///         ),
///         schema::ResolutionEnum::J(res) => Ok(
///             queries::explode_delta_j(client, &res).await?
///         ),
///         schema::ResolutionEnum::K(res) => Ok(
///             queries::explode_delta_k(client, &res).await?
///         ),
///         _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived("resolution", "delta")),
///     }
/// }
/// ```
pub(super) fn fn_on_receive_resolution(
    job: &JobConfig,
    all_dims: &DimensionConfigMap,
) -> syn::ImplItemFn {
    let operon = operon_ident();
    let res_enum_ident = resolution_enum_ident();
    let job_id = &job.id;

    let explode_arms = job.dims.iter().map(|dim| -> syn::Arm {
        let variant_ident = variant_ident(dim);
        let explode_fn_name = explode_ident(job_id, dim);
        parse_quote! {
            schema::#res_enum_ident::#variant_ident(res) => Ok(
                queries::#explode_fn_name(client, &res).await?
            ),
        }
    });
    let resolve_arms = get_quota_required_dims(job, all_dims)
        .into_iter()
        .map(|dim| -> syn::Arm {
            let variant_ident = variant_ident(&dim.id);
            let resolve_dep_fn_name = resolve_dep_ident(&job.id, &dim.id);
            parse_quote! {
                schema::#res_enum_ident::#variant_ident(res) => {
                    let new_ready = queries::#resolve_dep_fn_name(client, &res).await?;
                    #operon::log::info!("{:#?}, {:#?}", res, new_ready);
                    Ok(new_ready)
                },
            }
        });

    parse_quote! {
        #[allow(unused_variables, clippy::match_single_binding)]
        async fn on_receive_resolution(
            &self,
            client: #operon::meta_storage::MetaClient<'_>,
            resolution: schema::#res_enum_ident,
        ) -> Result<Vec<Self::Ticket>, #operon::scheduler::SchedulerError> {
            match resolution {
                #(#explode_arms)*
                #(#resolve_arms)*
                _ => Err(#operon::scheduler::SchedulerError::InvalidPeerEventReceived("resolution", #job_id)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_dimensions, job_delta};

    #[rstest]
    #[case::simple(job_delta(), "spec/fn_on_receive_resolution.rs")]
    fn test_fn_on_receive_resolution(
        all_dimensions: DimensionConfigMap,
        #[case] job: JobConfig,
        #[case] fixture_path: &str,
    ) {
        let item = fn_on_receive_resolution(&job, &all_dimensions);
        assert_item_eq(&item, fixture_path);
    }
}
