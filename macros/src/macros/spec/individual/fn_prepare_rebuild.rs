use syn::parse_quote;

use crate::{
    JobConfig,
    utils::{get_all_ident, get_resolution_ident, operon_ident, rebuilder_ident, variable_ident},
};

pub(super) fn fn_prepare_rebuild(job: &JobConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let rebuilder_ident = rebuilder_ident(&job.id);

    let get_all_fn_name = get_all_ident(&job.id);

    let resolve_fail_msg = format!("Failed to resolve a {} ticket", job.id);

    let resolution_expr: syn::Expr = match job.spawn_dim.as_ref() {
        Some(dim) => {
            let get_resolution_fn_name = get_resolution_ident(dim);
            let dim_vars = job
                .dims
                .iter()
                .map(|d| variable_ident(d))
                .collect::<Vec<_>>();

            let missing_resolution_msg = format!(
                "No resolution found for {}_{}",
                dim,
                "{},".repeat(job.dims.len()).trim_end_matches(",")
            );

            parse_quote! {
                queries::#get_resolution_fn_name(client, #(job.#dim_vars,)*)
                    .await?
                    .ok_or_else(|| {
                        #operon::scheduler::SchedulerError::Other(format!(
                            #missing_resolution_msg,
                            #(job.#dim_vars,)*
                        ))
                    })?
            }
        }
        None => parse_quote! { () },
    };

    parse_quote! {
        async fn prepare_rebuild(
            &self,
            storage: &Sto,
            client: #operon::meta_storage::MetaClient<'_>,
        ) -> Result<Box<dyn #operon::scheduler::JobRebuilder>, #operon::scheduler::SchedulerError>
        {
            let tickets =
                queries::#get_all_fn_name(client, #operon::schema_base::TicketStatus::Done).await?;
            let successes = #operon::futures::future::try_join_all(tickets.into_iter().map(
                |ticket| async move {
                    let job = #operon::schema_base::Ticket::resolve(&ticket).ok_or_else(|| {
                        #operon::scheduler::SchedulerError::Other(
                            #resolve_fail_msg.into()
                        )
                    })?;
                    let resolution = #resolution_expr;

                    Ok::<_, #operon::scheduler::SchedulerError>((job, resolution))
                }
            ))
            .await?;

            Ok(Box::new(#rebuilder_ident(successes)))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::JobArg;

    use super::*;

    #[test]
    fn test_fn_prepare_rebuild() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
        };
        let item = fn_prepare_rebuild(&job);
        let expected: syn::ImplItemFn = parse_quote! {
            async fn prepare_rebuild(
                &self,
                storage: &Sto,
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<Box<dyn operon::scheduler::JobRebuilder>, operon::scheduler::SchedulerError>
            {
                let tickets =
                    queries::get_all_beta(client, operon::schema_base::TicketStatus::Done).await?;
                let successes = operon::futures::future::try_join_all(tickets.into_iter().map(
                    |ticket| async move {
                        let job = operon::schema_base::Ticket::resolve(&ticket).ok_or_else(|| {
                            operon::scheduler::SchedulerError::Other(
                                "Failed to resolve a beta ticket".into()
                            )
                        })?;
                        let resolution = queries::get_resolution_j(client, job.i,).await?
                            .ok_or_else(|| {
                                operon::scheduler::SchedulerError::Other(format!(
                                    "No resolution found for j_{}",
                                    job.i,
                                ))
                            })?;

                        Ok::<_, operon::scheduler::SchedulerError>((job, resolution))
                    }
                ))
                .await?;

                Ok(Box::new(BetaRebuilder(successes)))
            }
        };

        assert_eq!(item, expected);
    }
}
