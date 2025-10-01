use syn::parse_quote;

use crate::JobConfig;
use crate::utils::{
    clear_ticket_ident, get_all_ident, init_ticket_ident, job_ident, operon_ident,
    put_ticket_ident, ticket_ident, variable_ident,
};

/// Generates the implementation of the `TicketSql` trait for a given job's ticket.
///
/// Example:
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// #[automatically_derived]
/// impl operon::schema_base::TicketSql for BetaTicket {
///     fn to_sql_insert_params(
///         &self,
///     ) -> Result<
///         Vec<Box<dyn operon::postgres_types::ToSql + Send + Sync>>,
///         operon::meta_storage::MetaStorageError,
///     > {
///         let i = self.i.to_sql()?;
///         let resolved = operon::schema_base::Ticket::is_resolved(self);
///         let deps_count = i64::try_from(self.deps_count)?;
///         let deps_quota = self.deps_quota.map(i64::try_from).transpose()?;
///         let deps_done = self.deps_done;
///         let status = self.status;
///
///         Ok(vec![
///             Box::new(i),
///             Box::new(resolved),
///             Box::new(deps_count),
///             Box::new(deps_quota),
///             Box::new(deps_done),
///             Box::new(status),
///         ])
///     }
///
///     fn to_sql_copy_params(&self) -> Result<String, operon::meta_storage::MetaStorageError> {
///         Ok(format!(
///             "{},{},{},{},{},{}\n",
///             self.i.to_sql()?,
///             operon::schema_base::Ticket::is_resolved(self),
///             self.deps_count,
///             self.deps_quota.map_or(String::new(), |q| q.to_string()),
///             self.deps_done,
///             self.status,
///         ))
///     }
///
///     fn from_sql_row(
///         row: &operon::tokio_postgres::Row,
///     ) -> Result<Self, operon::meta_storage::MetaStorageError> {
///         let i = operon::schema_base::TicketDepCount::from_sql(row.get(stringify!(i)))?;
///         // let resolved: bool = row.get("resolved");
///         let deps_count = usize::try_from(row.get::<_, i64>("deps_count"))?;
///         let deps_quota = row
///             .get::<_, Option<i64>>("deps_quota")
///             .map(usize::try_from)
///             .transpose()?;
///         let deps_done: bool = row.get("deps_done");
///         let status: operon::schema_base::TicketStatus = row.get("status");
///
///         Ok(BetaTicket {
///             i,
///             // resolved,
///             deps_count,
///             deps_quota,
///             deps_done,
///             status,
///         })
///     }
///
///     async fn init_table(
///         client: operon::meta_storage::MetaClient<'_>,
///     ) -> Result<(), operon::meta_storage::MetaStorageError> {
///         queries::init_ticket_beta(client).await
///     }
///
///     async fn clear_table(
///         client: operon::meta_storage::MetaClient<'_>,
///     ) -> Result<(), operon::meta_storage::MetaStorageError> {
///         queries::clear_ticket_beta(client).await
///     }
///
///     async fn put(
///         &self,
///         client: operon::meta_storage::MetaClient<'_>,
///     ) -> Result<(), operon::meta_storage::MetaStorageError> {
///         queries::put_ticket_beta(client, self).await
///     }
///
///     async fn get_all(
///         client: operon::meta_storage::MetaClient<'_>,
///         status: operon::schema_base::TicketStatus,
///     ) -> Result<Vec<Self>, operon::meta_storage::MetaStorageError> {
///         queries::get_all_beta(client, status).await
///     }
///
///     async fn get_status(
///         client: operon::meta_storage::MetaClient<'_>,
///     ) -> Result<(i64, i64, i64), operon::meta_storage::MetaStorageError> {
///         operon::meta_storage::get_ticket_summary::<BetaJob>(client).await
///     }
/// }
pub(super) fn impl_ticket_sql(job: &JobConfig) -> syn::ItemImpl {
    let operon = operon_ident();
    let ticket_ident = ticket_ident(&job.id);
    let job_ident = job_ident(&job.id);
    let dim_fields = job
        .dims
        .iter()
        .map(|d| variable_ident(d))
        .collect::<Vec<_>>();

    let copy_template = {
        let mut s = std::iter::repeat_n("{}", job.dims.len() + 5)
            .collect::<Vec<_>>()
            .join(",");
        s.push('\n');
        s
    };

    let init_fn_name = init_ticket_ident(&job.id);
    let clear_fn_name = clear_ticket_ident(&job.id);
    let put_fn_name = put_ticket_ident(&job.id);
    let get_all_fn_name = get_all_ident(&job.id);

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::schema_base::TicketSql for #ticket_ident {
            fn to_sql_insert_params(
                &self,
            ) -> Result<
                Vec<Box<dyn #operon::postgres_types::ToSql + Send + Sync>>,
                #operon::meta_storage::MetaStorageError,
            > {
                #(let #dim_fields = self.#dim_fields.to_sql()?;)*
                let resolved = #operon::schema_base::Ticket::is_resolved(self);
                let deps_count = i64::try_from(self.deps_count)?;
                let deps_quota = self.deps_quota.map(i64::try_from).transpose()?;
                let deps_done = self.deps_done;
                let status = self.status;

                Ok(vec![
                    #(Box::new(#dim_fields),)*
                    Box::new(resolved),
                    Box::new(deps_count),
                    Box::new(deps_quota),
                    Box::new(deps_done),
                    Box::new(status),
                ])
            }

            fn to_sql_copy_params(&self) -> Result<String, #operon::meta_storage::MetaStorageError> {
                Ok(format!(
                    #copy_template,
                    #(self.#dim_fields.to_sql()?,)*
                    #operon::schema_base::Ticket::is_resolved(self),
                    self.deps_count,
                    self.deps_quota
                        .map_or(String::new(), |q| q.to_string()),
                    self.deps_done,
                    self.status,
                ))
            }

            fn from_sql_row(
                row: &#operon::tokio_postgres::Row,
            ) -> Result<Self, #operon::meta_storage::MetaStorageError> {
                #(let #dim_fields = #operon::schema_base::TicketDepCount::from_sql(row.get(stringify!(#dim_fields)))?;)*
                let deps_count = usize::try_from(row.get::<_, i64>("deps_count"))?;
                let deps_quota = row
                    .get::<_, Option<i64>>("deps_quota")
                    .map(usize::try_from)
                    .transpose()?;
                let deps_done: bool = row.get("deps_done");
                let status: #operon::schema_base::TicketStatus = row.get("status");

                Ok(#ticket_ident {
                    #(#dim_fields,)*
                    deps_count,
                    deps_quota,
                    deps_done,
                    status,
                })
            }

            async fn init_table(
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), #operon::meta_storage::MetaStorageError> {
                queries::#init_fn_name(client).await
            }

            async fn clear_table(
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), #operon::meta_storage::MetaStorageError> {
                queries::#clear_fn_name(client).await
            }

            async fn put(
                &self,
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), #operon::meta_storage::MetaStorageError> {
                queries::#put_fn_name(client, self).await
            }

            async fn get_all(
                client: #operon::meta_storage::MetaClient<'_>,
                status: #operon::schema_base::TicketStatus,
            ) -> Result<Vec<Self>, #operon::meta_storage::MetaStorageError> {
                queries::#get_all_fn_name(client, status).await
            }

            async fn get_status(
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<(i64, i64, i64), #operon::meta_storage::MetaStorageError> {
                #operon::meta_storage::get_ticket_summary::<#job_ident>(client).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;
    use crate::configs::JobArg;

    #[test]
    fn test_impl_ticket_sql() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
            pool_size: 8,
        };
        let item = impl_ticket_sql(&job);
        let expected: syn::ItemImpl = parse_quote! {
            #[operon::async_trait::async_trait]
            #[automatically_derived]
            impl operon::schema_base::TicketSql for BetaTicket {
                fn to_sql_insert_params(
                    &self,
                ) -> Result<
                    Vec<Box<dyn operon::postgres_types::ToSql + Send + Sync>>,
                    operon::meta_storage::MetaStorageError,
                > {
                    let i = self.i.to_sql()?;
                    let resolved = operon::schema_base::Ticket::is_resolved(self);
                    let deps_count = i64::try_from(self.deps_count)?;
                    let deps_quota = self.deps_quota.map(i64::try_from).transpose()?;
                    let deps_done = self.deps_done;
                    let status = self.status;

                    Ok(vec![
                        Box::new(i),
                        Box::new(resolved),
                        Box::new(deps_count),
                        Box::new(deps_quota),
                        Box::new(deps_done),
                        Box::new(status),
                    ])
                }

                fn to_sql_copy_params(&self) -> Result<String, operon::meta_storage::MetaStorageError> {
                    Ok(format!(
                        "{},{},{},{},{},{}\n",
                        self.i.to_sql()?,
                        operon::schema_base::Ticket::is_resolved(self),
                        self.deps_count,
                        self.deps_quota.map_or(String::new(), |q| q.to_string()),
                        self.deps_done,
                        self.status,
                    ))
                }

                fn from_sql_row(
                    row: &operon::tokio_postgres::Row,
                ) -> Result<Self, operon::meta_storage::MetaStorageError> {
                    let i = operon::schema_base::TicketDepCount::from_sql(row.get(stringify!(i)))?;
                    // let resolved: bool = row.get("resolved");
                    let deps_count = usize::try_from(row.get::<_, i64>("deps_count"))?;
                    let deps_quota = row
                        .get::<_, Option<i64>>("deps_quota")
                        .map(usize::try_from)
                        .transpose()?;
                    let deps_done: bool = row.get("deps_done");
                    let status: operon::schema_base::TicketStatus = row.get("status");

                    Ok(BetaTicket {
                        i,
                        // resolved,
                        deps_count,
                        deps_quota,
                        deps_done,
                        status,
                    })
                }

                async fn init_table(
                    client: operon::meta_storage::MetaClient<'_>,
                ) -> Result<(), operon::meta_storage::MetaStorageError> {
                    queries::init_ticket_beta(client).await
                }

                async fn clear_table(
                    client: operon::meta_storage::MetaClient<'_>,
                ) -> Result<(), operon::meta_storage::MetaStorageError> {
                    queries::clear_ticket_beta(client).await
                }

                async fn put(
                    &self,
                    client: operon::meta_storage::MetaClient<'_>,
                ) -> Result<(), operon::meta_storage::MetaStorageError> {
                    queries::put_ticket_beta(client, self).await
                }

                async fn get_all(
                    client: operon::meta_storage::MetaClient<'_>,
                    status: operon::schema_base::TicketStatus,
                ) -> Result<Vec<Self>, operon::meta_storage::MetaStorageError> {
                    queries::get_all_beta(client, status).await
                }

                async fn get_status(
                    client: operon::meta_storage::MetaClient<'_>,
                ) -> Result<(i64, i64, i64), operon::meta_storage::MetaStorageError> {
                    operon::meta_storage::get_ticket_summary::<BetaJob>(client).await
                }
            }
        };

        assert_eq!(item, expected);
    }
}
