use syn::parse_quote;

use crate::{
    JobConfig, JobConfigMap,
    configs::{DimensionConfigMap, EntityId},
    macros::schema::ticket::fn_get_dependency_quota::fn_get_dependency_quota,
    utils::{job_ident, operon_ident, spawn_resolution, ticket_ident, variable_ident},
};

/// Generates the implementation of the `Ticket` trait for a given job's ticket.
pub(super) fn impl_ticket(
    job: &JobConfig,
    primary_entity: &EntityId,
    dimensions: &DimensionConfigMap,
    jobs: &JobConfigMap,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let job_ident = job_ident(&job.id);
    let res_ident = spawn_resolution(job.spawn_dim.as_ref());
    let ticket_ident = ticket_ident(&job.id);

    let dim_fields = job
        .dims
        .iter()
        .map(|d| variable_ident(d))
        .collect::<Vec<_>>();

    let fn_get_dependency_quota = fn_get_dependency_quota(job, primary_entity, dimensions, jobs);

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::schema_base::Ticket for #ticket_ident {
            type Job = schema::#job_ident;
            type Resolution = #res_ident;

            #fn_get_dependency_quota

            async fn raise_dependency_count(
                self,
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<Self, #operon::meta_storage::MetaStorageError> {
                let mut ticket = self;
                ticket.deps_count += 1;
                if ticket.deps_quota.is_none() {
                    ticket.deps_quota = ticket.get_dependency_quota(client).await?;
                }
                ticket.deps_done = ticket.deps_quota.is_some_and(|quota| ticket.deps_count >= quota);
                if ticket.is_ready() {
                    ticket.status = operon::schema_base::TicketStatus::Queued;
                }
                Ok(ticket)
            }

            fn is_ready(&self) -> bool {
                self.is_resolved() && self.deps_done
            }

            fn is_resolved(&self) -> bool {
                #(self.#dim_fields.is_some())&&*
            }

            fn resolve(&self) -> Option<schema::#job_ident> {
                if self.is_ready() {
                    Some(schema::#job_ident {
                        #(#dim_fields: self.#dim_fields.0?,)*
                    })
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::parse_quote;

    use crate::{
        DimensionConfig,
        configs::{DimensionId, JobArg},
    };

    use super::*;

    #[test]
    fn test_impl_ticket() {
        let jobs = JobConfigMap::from_iter([(
            "beta".to_string(),
            JobConfig {
                id: "beta".to_string(),
                from: vec![JobArg {
                    id: "a".to_string(),
                    over: vec![],
                }],
                to: "b".to_string(),
                dims: vec!["i".to_string()],
                spawn_dim: Some("j".to_string()),
            },
        )]);
        let primary_entity = DimensionId::from("a");
        let dimensions = DimensionConfigMap::from_iter([(
            "i".to_string(),
            DimensionConfig {
                id: "i".to_string(),
                depends_on: vec![],
            },
        )]);
        let beta = jobs.get("beta").unwrap();
        let item = impl_ticket(beta, &primary_entity, &dimensions, &jobs);
        let fn_get_dependency_quota =
            fn_get_dependency_quota(beta, &primary_entity, &dimensions, &jobs);
        let expected: syn::ItemImpl = parse_quote! {
            #[operon::async_trait::async_trait]
            #[automatically_derived]
            impl operon::schema_base::Ticket for BetaTicket {
                type Job = schema::BetaJob;
                type Resolution = schema::JResolution;

                #fn_get_dependency_quota

                async fn raise_dependency_count(
                    self,
                    client: operon::meta_storage::MetaClient<'_>,
                ) -> Result<Self, operon::meta_storage::MetaStorageError> {
                    let mut ticket = self;
                    ticket.deps_count += 1;
                    if ticket.deps_quota.is_none() {
                        ticket.deps_quota = ticket.get_dependency_quota(client).await?;
                    }
                    ticket.deps_done = ticket.deps_quota.is_some_and(|quota| ticket.deps_count >= quota);
                    if ticket.is_ready() {
                        ticket.status = operon::schema_base::TicketStatus::Queued;
                    }
                    Ok(ticket)
                }

                fn is_ready(&self) -> bool {
                    self.is_resolved() && self.deps_done
                }

                fn is_resolved(&self) -> bool {
                    self.i.is_some()
                }

                fn resolve(&self) -> Option<schema::BetaJob> {
                    if self.is_ready() {
                        Some(schema::BetaJob { i: self.i.0?, })
                    } else {
                        None
                    }
                }
            }
        };
        assert_eq!(
            item.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
