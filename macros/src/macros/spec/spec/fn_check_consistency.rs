use quote::quote;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{get_entity_ident, operon_ident, variable_ident};

/// Generates the `check_consistency` function for the implementation of the trait `JobSpec`.
///
/// Example:
/// ```rust,ignore
/// async fn check_consistency(
///     &self,
///     storage: &Sto,
///     client: operon::meta_storage::MetaClient<'_>,
/// ) -> Result<bool, operon::scheduler::SchedulerError> {
///     // Pull the "done" beta jobs from the metadata storage...
///     let Some(jobs) = queries::get_all_beta(
///         client,
///         operon::schema::TicketStatus::Done,
///     )
///     .await?
///     .iter()
///     .map(|t| operon::schema::Ticket::resolve(t))
///     .collect::<Option<Vec<_>>>() else {
///         operon::log::info!("Some `beta` tickets are corrupt in the metadata storage.");
///         return Ok(false);
///     };
///     // ...and map them with the dimensions they spawned...
///     let mut tags = Vec::new();
///     for job in jobs {
///         let Some(res) = queries::get_resolution_j(client, job.i,).await? else {
///             operon::log::info!(
///                 "No `j` resolution found for `beta_{}` in the metadata storage.",
///                 job.i,
///             );
///             return Ok(false);
///         };
///         for j in 0..(res.0) {
///             tags.push((job.i, j,));
///         }
///     }
///     // ...and check if the data storage holds all the data for them.
///     for (i, j,) in tags {
///         if storage.get_b(i, j,).await?.is_none() {
///             operon::log::info!("Data storage does not hold `b_{},{}`.", i, j,);
///             return Ok(false);
///         }
///     }
///
///     Ok(true)
/// }
/// ```
pub(super) fn fn_check_consistency(job: &JobConfig) -> syn::ImplItemFn {
    let operon = operon_ident();

    let field_vars = job
        .dims
        .iter()
        .map(|d| variable_ident(d))
        .collect::<Vec<_>>();
    let get_fn_name = get_entity_ident(&job.to);

    let corrupt_msg = format!(
        "Some `{}` tickets are corrupt in the metadata storage.",
        job.id
    );
    let missing_entity_msg = format!("Data storage does not hold `{}_{{:?}}`.", job.to);

    let check_res_and_entity = match job.spawn_dim.as_ref() {
        Some(dim) => {
            // let res_ident = resolution_ident(dim);
            let dim_var = variable_ident(dim);
            let missing_res_msg = format!(
                "No `{}` resolution found for `{}_{{:?}}` in the metadata storage.",
                dim, job.id,
            );

            quote! {
                let mut tags = Vec::new();
                for job in jobs {
                    let Some(res) = client.resolution(self.spawn_dim_meta()).get(job.coordinate).await?
                    else {
                        #operon::log::info!(#missing_res_msg, job.coordinate);
                        return Ok(false);
                    };
                    for #dim_var in 0..(res.ub) {
                        tags.push((job.coordinate, #dim_var));
                    }
                }
                for ([#(#field_vars),*], #dim_var) in tags {
                    if storage.#get_fn_name(#(#field_vars,)* #dim_var,).await?.is_none() {
                        #operon::log::info!(#missing_entity_msg, [#(#field_vars,)* #dim_var]);
                        return Ok(false);
                    }
                }
            }
        }
        None => {
            quote! {
                for job in jobs {
                    let [#(#field_vars),*] = job.coordinate;

                    if storage.#get_fn_name(#(#field_vars),*).await?.is_none() {
                        #operon::log::info!(#missing_entity_msg, [#(#field_vars),*]);
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
            client: #operon::meta_storage::MetaClient<'_>,
        ) -> Result<bool, #operon::scheduler::SchedulerError> {
            let tickets = client
                .ticket(self.job_meta())
                .get_all(#operon::schema::TicketStatus::Done)
                .await?;
            let Some(jobs) = tickets
                .iter()
                .map(|ticket| ticket.resolve())
                .collect::<Option<Vec<_>>>() else {
                    #operon::log::info!(#corrupt_msg);
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
