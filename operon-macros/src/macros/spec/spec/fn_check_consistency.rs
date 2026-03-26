use quote::quote;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{clear_span, get_entity_ident, operon_ident};

/// Generates the `check_consistency` function for the implementation of the trait `JobSpec`.
///
/// # Example
/// ```rust,ignore
/// async fn check_consistency(
///     &self,
///     storage: &Sto,
///     client: operon::__private::MetaClient<'_>,
///     mode: operon::__private::CheckMode,
/// ) -> Result<bool, operon::error::SchedulerError> {
///     if mode == operon::__private::CheckMode::TrustAll {
///         return Ok(true);
///     }
///
///     let tickets = client
///         .ticket(self.job_meta())
///         .get_all(operon::__private::TicketStatus::Done)
///         .await?;
///     // Pull the "done" epsilon jobs from the metadata storage...
///     let Some(coordinates) = tickets
///         .iter()
///         .map(|ticket| ticket.resolve().map(|job| job.coordinate))
///         .collect::<Option<Vec<_>>>()
///     else {
///         operon::__private::tracing::info!("Some `epsilon` tickets are corrupt in the metadata storage.");
///         return Ok(false);
///     };
///
///     // ...and check if the data storage holds all the data for them.
///     let coordinates_to_check = match mode {
///         operon::__private::CheckMode::MetadataOnly => return Ok(true),
///         operon::__private::CheckMode::Exhaustive => {
///             coordinates
///         }
///         operon::__private::CheckMode::Quick => {
///             operon::__private::get_dop_coords(&coordinates)
///         }
///         _ => unreachable!(),
///     };
///
///     for coordinate in coordinates_to_check {
///         if storage.get_e(coordinate).await?.is_none() {
///             // TODO: improve error message coordinate display
///             operon::__private::tracing::info!("Data storage does not hold `E_{:?}`.", coordinate);
///             return Ok(false);
///         }
///     }
///
///     Ok(true)
/// }
/// ```
pub(super) fn fn_check_consistency(job: &JobConfig) -> syn::ImplItemFn {
    let operon = operon_ident();

    let field_vars = job.dims.iter().map(clear_span).collect::<Vec<_>>();
    let get_fn_name = get_entity_ident(&job.to);

    let corrupt_msg = format!(
        "Some `{}` tickets are corrupt in the metadata storage.",
        job.id
    );
    let missing_entity_msg = format!("Data storage does not hold `{}_{{:?}}`.", job.to);

    let check_res_and_entity = match job.spawn_dim.as_ref() {
        Some(spawn_dim) => {
            let spawn_dim = clear_span(spawn_dim);
            let missing_res_msg = format!(
                "No `{}` resolution found for `{}_{{:?}}` in the metadata storage.",
                spawn_dim, job.id,
            );

            quote! {
                let mut tags = Vec::new();
                for coordinate in coordinates {
                    let Some(res) = client.resolution(self.spawn_dim_meta()).get(coordinate).await?
                    else {
                        #operon::__private::tracing::info!(#missing_res_msg, coordinate);
                        return Ok(false);
                    };
                    for #spawn_dim in 0..(res.ub) {
                        tags.push((coordinate, #spawn_dim));
                    }
                }

                let tags_to_check = match mode {
                    #operon::__private::CheckMode::MetadataOnly => return Ok(true),
                    #operon::__private::CheckMode::Exhaustive => {
                        tags
                    }
                    #operon::__private::CheckMode::Quick => {
                        #operon::__private::get_dop_tags(&tags)
                    }
                    _ => unreachable!(),
                };

                for ([#(#field_vars),*], #spawn_dim) in tags_to_check {
                    if storage.#get_fn_name([#(#field_vars,)* #spawn_dim]).await?.is_none() {
                        #operon::__private::tracing::info!(#missing_entity_msg, [#(#field_vars,)* #spawn_dim]);
                        return Ok(false);
                    }
                }
            }
        }
        None => {
            quote! {
                let coordinates_to_check = match mode {
                    #operon::__private::CheckMode::MetadataOnly => return Ok(true),
                    #operon::__private::CheckMode::Exhaustive => {
                        coordinates
                    }
                    #operon::__private::CheckMode::Quick => {
                        #operon::__private::get_dop_coords(&coordinates)
                    }
                    _ => unreachable!(),
                };

                for coordinate in coordinates_to_check {
                    if storage.#get_fn_name(coordinate).await?.is_none() {
                        #operon::__private::tracing::info!(#missing_entity_msg, coordinate);
                        return Ok(false);
                    }
                }
            }
        }
    };

    parse_quote! {
        async fn check_consistency(
            &self,
            storage: &Sto,
            client: #operon::__private::MetaClient<'_>,
            mode: #operon::__private::CheckMode,
        ) -> Result<bool, #operon::error::SchedulerError> {
            if mode == #operon::__private::CheckMode::TrustAll {
                return Ok(true);
            }

            let tickets = client
                .ticket(self.job_meta())
                .get_all(#operon::__private::TicketStatus::Done)
                .await?;
            let Some(coordinates) = tickets
                .iter()
                .map(|ticket| ticket.resolve().map(|job| job.coordinate))
                .collect::<Option<Vec<_>>>() else {
                    #operon::__private::tracing::info!(#corrupt_msg);
                    return Ok(false);
                };

            #check_res_and_entity

            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{job_beta, job_epsilon};

    #[rstest]
    #[case::simple(job_beta(), "spec/spec/fn_check_consistency.simple.rs")]
    #[case::no_spawn_dim(job_epsilon(), "spec/spec/fn_check_consistency.no_spawn_dim.rs")]
    fn test_fn_check_consistency(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let item = fn_check_consistency(&job);
        assert_item_eq(&item, fixture_path)
    }
}
