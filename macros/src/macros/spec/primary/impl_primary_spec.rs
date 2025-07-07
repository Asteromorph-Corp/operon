use syn::parse_quote;

use crate::{
    configs::{DimensionId, EntityId},
    utils::{
        get_entity_ident, operon_ident, resolution_ident, service_trait_ident, storage_trait_ident,
    },
};

/// Generates the `impl PrimarySpec` for the primary entity and dimension.
///
/// Example:
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// impl<Svc, Sto> operon::scheduler::PrimarySpec<Svc, Sto> for AlphaSpec
/// where
///     Svc: MyOperonService<JobEnum = schema::MyJobEnum, ResolutionEnum = schema::ResolutionEnum>,
///     Sto: MyOperonStorage,
/// {
///     type Resolution = schema::IResolution;
///
///     async fn check_consistency(
///         &self,
///         storage: &Sto,
///         client: operon::meta_storage::MetaClient<'_>,
///         primary_ub: usize,
///     ) -> Result<bool, operon::scheduler::SchedulerError> {
///         // Pull the primary resolution from the metadata storage..
///         let Some(schema::IResolution(i_ub)) =
///             <schema::IResolution as operon::schema_base::ResolutionSql>::get(client, ())
///                 .await?
///         else {
///             // This is technically unreachable, because we check this same value
///             // in `check_recovery_state`.
///             operon::log::info!("No primary resolution found in the metadata storage.");
///             return Ok(false);
///         };
///         // ...and check if the data storage holds all the data for it.
///         for i in 0..i_ub.max(primary_ub) {
///             if storage.get_a(i).await?.is_none() {
///                 operon::log::info!("Data storage does not hold `A_{i}`.");
///                 return Ok(false);
///             }
///         }
///         // Additionally check if the primary resolution agrees with the given upper bound.
///         if i_ub != primary_ub {
///             operon::log::warn!(
///                 "Previous run's upper bound `{i_ub}` is different from the current run's upper bound `{primary_ub}`. \n\
///                 If you overwrote the primary data, consider running `run --fresh` to overwrite the existing data, \
///                 otherwise the resulting data may be inconsistent. \n\
///                 If you want to keep the existing data, and intendedly set the upper bound to `{primary_ub}`, \
///                 you may ignore this warning."
///             );
///         }
///
///         return Ok(true);
///     }
/// }
/// ```
pub fn impl_primary_spec(
    service_id: &str,
    primary_entity: &EntityId,
    primary_dimension: &DimensionId,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let res_ident = resolution_ident(primary_dimension);
    let svc_trait = service_trait_ident(service_id);
    let sto_trait = storage_trait_ident(service_id);
    let get_fn_name = get_entity_ident(primary_entity);

    parse_quote! {
        #[#operon::async_trait::async_trait]
        impl<Svc: #svc_trait, Sto: #sto_trait> #operon::scheduler::PrimarySpec<Svc, Sto> for PrimarySpec
        {
            type Resolution = schema::#res_ident;

            async fn check_consistency(
                &self,
                storage: &Sto,
                client: #operon::meta_storage::MetaClient<'_>,
                primary_ub: usize,
            ) -> Result<bool, #operon::scheduler::SchedulerError> {
                // Pull the primary resolution from the metadata storage..
                let Some(schema::#res_ident(i_ub)) = Self::Resolution::get(client, ()).await? else {
                    // This is technically unreachable, because we check this same value
                    // in `check_recovery_state`.
                    #operon::log::info!("No primary resolution found in the metadata storage.");
                    return Ok(false);
                };
                // ...and check if the data storage holds all the data for it.
                for i in 0..i_ub.max(primary_ub) {
                    if storage.#get_fn_name(i).await?.is_none() {
                        #operon::log::info!("Data storage does not hold `A_{i}`.");
                        return Ok(false);
                    }
                }
                // Additionally check if the primary resolution agrees with the given upper bound.
                if i_ub != primary_ub {
                    #operon::log::warn!(
                        "Previous run's upper bound `{i_ub}` is different from the current run's upper bound `{primary_ub}`. \n\
                        If you overwrote the primary data, consider running `run --fresh` to overwrite the existing data, \
                        otherwise the resulting data may be inconsistent. \n\
                        If you want to keep the existing data, and intendedly set the upper bound to `{primary_ub}`, \
                        you may ignore this warning."
                    );
                }

                return Ok(true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impl_primary_spec() {
        let service_id = "MyOperon";
        let primary_entity = EntityId::from("A");
        let primary_dimension = DimensionId::from("i");

        let item = impl_primary_spec(service_id, &primary_entity, &primary_dimension);
        let expected: syn::ItemImpl = parse_quote! {
            #[operon::async_trait::async_trait]
            impl<Svc: MyOperonService, Sto: MyOperonStorage> operon::scheduler::PrimarySpec<Svc, Sto> for PrimarySpec
            {
                type Resolution = schema::IResolution;

                async fn check_consistency(
                    &self,
                    storage: &Sto,
                    client: operon::meta_storage::MetaClient<'_>,
                    primary_ub: usize,
                ) -> Result<bool, operon::scheduler::SchedulerError> {
                    // Pull the primary resolution from the metadata storage..
                    let Some(schema::IResolution(i_ub)) = Self::Resolution::get(client, ()).await? else {
                        // This is technically unreachable, because we check this same value
                        // in `check_recovery_state`.
                        operon::log::info!("No primary resolution found in the metadata storage.");
                        return Ok(false);
                    };
                    // ...and check if the data storage holds all the data for it.
                    for i in 0..i_ub.max(primary_ub) {
                        if storage.get_a(i).await?.is_none() {
                            operon::log::info!("Data storage does not hold `A_{i}`.");
                            return Ok(false);
                        }
                    }
                    // Additionally check if the primary resolution agrees with the given upper bound.
                    if i_ub != primary_ub {
                        operon::log::warn!(
                            "Previous run's upper bound `{i_ub}` is different from the current run's upper bound `{primary_ub}`. \n\
                        If you overwrote the primary data, consider running `run --fresh` to overwrite the existing data, \
                        otherwise the resulting data may be inconsistent. \n\
                        If you want to keep the existing data, and intendedly set the upper bound to `{primary_ub}`, \
                        you may ignore this warning."
                        );
                    }

                    return Ok(true);
                }
            }
        };

        assert_eq!(item, expected);
    }
}
