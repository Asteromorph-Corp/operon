use indexmap::IndexSet;
use syn::parse_quote;

use crate::{
    JobConfig,
    utils::{job_enum_ident, operon_ident, raise_dep_ident, variable_ident, variant_ident},
};

pub(super) fn fn_on_receive_job(
    job: &JobConfig,
    upstream_jobs: &IndexSet<&JobConfig>,
) -> syn::ImplItemFn {
    let operon = operon_ident();
    let job_enum_ident = job_enum_ident();
    let job_id = &job.id;

    let raise_dep_fn_name = raise_dep_ident(&job.id);

    let job_arms = upstream_jobs.iter().map(|upstream_job| -> syn::Arm {
        let variant_ident = variant_ident(&upstream_job.id);

        let args = job.dims.iter().map(|d| -> syn::Expr {
            if upstream_job.dims.contains(d) {
                let field_ident = variable_ident(d);
                parse_quote! { #operon::schema_base::TicketDepCount::some(job.#field_ident) }
            } else {
                parse_quote! { #operon::schema_base::TicketDepCount::none() }
            }
        });

        parse_quote! {
            schema::#job_enum_ident::#variant_ident(job) => Ok(
                queries::#raise_dep_fn_name(client, #(&#args,)*).await?
            ),
        }
    });

    parse_quote! {
        #[allow(unused_variables, clippy::match_single_binding)]
        async fn on_receive_job(
            &self,
            client: #operon::meta_storage::MetaClient<'_>,
            job: schema::#job_enum_ident,
        ) -> Result<Vec<Self::Ticket>, #operon::scheduler::SchedulerError> {
            match job {
                #(#job_arms)*
                _ => Err(#operon::scheduler::SchedulerError::InvalidPeerEventReceived("job", #job_id)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::JobArg;

    use super::*;

    #[test]
    fn test_fn_on_receive_job() {
        let job = JobConfig {
            id: "epsilon".to_string(),
            from: vec![
                JobArg {
                    id: "b".to_string(),
                    over: vec!["j".to_string()],
                },
                JobArg {
                    id: "d".to_string(),
                    over: vec!["j".to_string()],
                },
            ],
            to: "e".to_string(),
            dims: vec!["i".to_string(), "k".to_string()],
            spawn_dim: None,
        };
        let beta = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
        };
        let delta = JobConfig {
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
        };

        let upstream_jobs = IndexSet::from_iter(vec![&beta, &delta]);

        let item = fn_on_receive_job(&job, &upstream_jobs);
        let expected: syn::ImplItemFn = parse_quote! {
            #[allow(unused_variables, clippy::match_single_binding)]
            async fn on_receive_job(
                &self,
                client: operon::meta_storage::MetaClient<'_>,
                job: schema::JobEnum,
            ) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
                match job {
                    schema::JobEnum::Beta(job) => Ok(
                        queries::raise_dep_epsilon(
                            client,
                            &operon::schema_base::TicketDepCount::some(job.i),
                            &operon::schema_base::TicketDepCount::none(),
                        )
                        .await?
                    ),
                    schema::JobEnum::Delta(job) => Ok(
                        queries::raise_dep_epsilon(
                            client,
                            &operon::schema_base::TicketDepCount::some(job.i),
                            &operon::schema_base::TicketDepCount::some(job.k),
                        )
                        .await?
                    ),
                    _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived("job", "epsilon")),
                }
            }
        };
        assert_eq!(item, expected);
    }
}
