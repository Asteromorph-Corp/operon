use syn::parse_quote;

use crate::{
    JobConfig,
    utils::{job_ident, operon_ident, spawn_resolution, ticket_ident, variable_ident},
};

// TODO: Implement `get_dependency_quota` and `raise_dependency_count`
pub(super) fn impl_ticket(job: &JobConfig) -> syn::ItemImpl {
    let operon = operon_ident();
    let job_ident = job_ident(&job.id);
    let res_ident = spawn_resolution(job.spawn_dim.as_ref());
    let ticket_ident = ticket_ident(&job.id);
    let dim_fields = job
        .dims
        .iter()
        .map(|d| variable_ident(d))
        .collect::<Vec<_>>();

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::schema_base::Ticket for #ticket_ident {
            type Job = #job_ident;
            type Resolution = #res_ident;

            async fn get_dependency_quota(
                &self,
                _client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<Option<usize>, #operon::meta_storage::MetaStorageError> {
                todo!();
            }

            async fn raise_dependency_count(
                &mut self,
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), #operon::meta_storage::MetaStorageError> {
                todo!();
            }

            fn is_ready(&self) -> bool {
                self.is_resolved() && self.deps_done
            }

            fn is_resolved(&self) -> bool {
                #(self.#dim_fields.is_some())&&*
            }

            fn resolve(&self) -> Option<#job_ident> {
                if self.is_ready() {
                    Some(#job_ident {
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
    use syn::parse_quote;

    use super::*;

    #[test]
    #[ignore = "todo"] // TODO: Remove this once the `get_dependency_quota` and `raise_dependency_count` methods are implemented
    fn test_impl_ticket() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec!["a".to_string()],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
        };
        let item = impl_ticket(&job);
        let expected: syn::ItemImpl = parse_quote! {
            #[operon::async_trait::async_trait]
            #[automatically_derived]
            impl operon::schema_base::Ticket for BetaTicket {
                type Job = BetaJob;
                type Resolution = JResolution;

                async fn get_dependency_quota(
                    &self,
                    _client: operon::meta_storage::MetaClient<'_>,
                ) -> Result<Option<usize>, operon::meta_storage::MetaStorageError> {
                    panic!("Called get_dependency_quota on `beta` ticket");
                }

                async fn raise_dependency_count(
                    &mut self,
                    client: operon::meta_storage::MetaClient<'_>,
                ) -> Result<(), operon::meta_storage::MetaStorageError> {
                    panic!("Called raise_dependency_count on beta_i");
                }

                fn is_ready(&self) -> bool {
                    self.is_resolved() && self.deps_done
                }

                fn is_resolved(&self) -> bool {
                    self.i.is_some()
                }

                fn resolve(&self) -> Option<BetaJob> {
                    if self.is_ready() {
                        Some(BetaJob { i: self.i? })
                    } else {
                        None
                    }
                }
            }
        };
        assert_eq!(item, expected);
    }

    #[test]
    #[ignore = "todo"] // TODO: Remove this once the `get_dependency_quota` and `raise_dependency_count` methods are implemented
    fn test_impl_ticket_with_dependency() {
        let job = JobConfig {
            id: "epsilon".to_string(),
            from: vec!["b".to_string(), "d".to_string()],
            to: "e".to_string(),
            dims: vec!["i".to_string(), "k".to_string()],
            spawn_dim: None,
        };
        let item = impl_ticket(&job);
        let expected: syn::ItemImpl = parse_quote! {
            #[operon::async_trait::async_trait]
            #[automatically_derived]
            impl operon::schema_base::Ticket for EpsilonTicket {
                type Job = EpsilonJob;
                type Resolution = ();

                async fn get_dependency_quota(
                    &self,
                    client: operon::meta_storage::MetaClient<'_>,
                ) -> Result<Option<usize>, operon::meta_storage::MetaStorageError> {
                    let Some(i) = &self.i else {
                        return Ok(None);
                    };
                    let Some(j_resolution) = queries::get_resolution_j(conn, i).await? else {
                        return Ok(None);
                    };
                    let JResolution(resolved_j, _) = j_resolution;
                    Ok(Some(1 + resolved_j))
                }

                async fn raise_dependency_count(
                    &mut self,
                    client: operon::meta_storage::MetaClient<'_>,
                ) -> Result<(), operon::meta_storage::MetaStorageError> {
                    self.deps_count += 1;
                    if self.deps_quota.is_none() {
                        self.deps_quota = self.get_dependency_quota(client).await?;
                    }
                    self.deps_done = self.deps_quota.is_some_and(|quota| self.deps_count >= quota);
                    Ok(())
                }

                fn is_ready(&self) -> bool {
                    self.is_resolved() && self.deps_done
                }

                fn is_resolved(&self) -> bool {
                    self.i.is_some() && self.k.is_some()
                }

                fn resolve(&self) -> Option<EpsilonJob> {
                    if self.is_ready() {
                        Some(EpsilonJob {
                            i: self.i?,
                            k: self.k?,
                        })
                    } else {
                        None
                    }
                }
            }
        };
        assert_eq!(item, expected);
    }
}
