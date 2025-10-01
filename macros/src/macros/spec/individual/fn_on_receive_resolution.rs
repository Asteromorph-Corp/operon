use syn::parse_quote;

use crate::JobConfig;
use crate::utils::{explode_ident, operon_ident, resolution_enum_ident, variant_ident};

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
pub(super) fn fn_on_receive_resolution(job: &JobConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let res_enum_ident = resolution_enum_ident();
    let job_id = &job.id;

    let resolution_arms = job.dims.iter().map(|dim| -> syn::Arm {
        let variant_ident = variant_ident(dim);
        let explode_fn_name = explode_ident(job_id, dim);
        parse_quote! {
            schema::#res_enum_ident::#variant_ident(res) => Ok(
                queries::#explode_fn_name(client, &res).await?
            ),
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
                #(#resolution_arms)*
                _ => Err(#operon::scheduler::SchedulerError::InvalidPeerEventReceived("resolution", #job_id)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JobArg;

    #[test]
    fn test_fn_on_receive_resolution() {
        let job = JobConfig {
            id: "delta".to_string(),
            from: vec![
                JobArg {
                    id: "a".to_string(),
                    over: vec![],
                },
                JobArg {
                    id: "b".to_string(),
                    over: vec![],
                },
                JobArg {
                    id: "c".to_string(),
                    over: vec![],
                },
            ],
            to: "d".to_string(),
            dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
            spawn_dim: None,
            pool_size: 4,
        };

        let item = fn_on_receive_resolution(&job);
        let expected: syn::ImplItemFn = parse_quote! {
            #[allow(unused_variables, clippy::match_single_binding)]
            async fn on_receive_resolution(
                &self,
                client: operon::meta_storage::MetaClient<'_>,
                resolution: schema::ResolutionEnum,
            ) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
                match resolution {
                    schema::ResolutionEnum::I(res) => Ok(
                        queries::explode_delta_i(client, &res).await?
                    ),
                    schema::ResolutionEnum::J(res) => Ok(
                        queries::explode_delta_j(client, &res).await?
                    ),
                    schema::ResolutionEnum::K(res) => Ok(
                        queries::explode_delta_k(client, &res).await?
                    ),
                    _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived("resolution", "delta")),
                }
            }
        };

        assert_eq!(item, expected);
    }
}
