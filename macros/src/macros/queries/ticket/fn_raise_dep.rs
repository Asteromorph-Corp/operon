use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{job_metadata_ident, operon_ident, raise_dep_ident, variable_ident};

/// A helper struct to generate the SQL query for popping tickets to be raised.
struct RaiseDepPopQuery<'a>(&'a JobConfig);

impl std::fmt::Display for RaiseDepPopQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let job_id = &self.0.id;

        writeln!(f, "DELETE FROM {{schema_prefix}}ticket_{job_id}")?;
        writeln!(f, "{{where_clause}}")?;
        write!(f, "RETURNING *;")
    }
}

/// A helper struct to generate the SQL query for copying exploded result into the database.
struct RaiseDepCopyInQuery<'a>(&'a JobConfig);

impl std::fmt::Display for RaiseDepCopyInQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let job_id = &self.0.id;

        writeln!(f, "COPY {{schema_prefix}}ticket_{job_id} (")?;
        write!(f, "    ")?;
        for dim in &self.0.dims {
            write!(f, "{dim}, ")?;
        }
        writeln!(f, "deps_done, deps_quota, status")?;
        writeln!(f, ")")?;
        write!(f, "FROM STDIN WITH (FORMAT csv);")
    }
}

/// Generates the `raise_dep_*` function for a given job.
///
/// Example:
/// ```rust,ignore
/// pub async fn raise_dep_beta(
///     client: operon::meta_storage::MetaClient<'_>,
///     i: &operon::schema_base::OptionCoordinate,
/// ) -> Result<Vec<schema::BetaTicket>, operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     let params = [("i", i),]
///         .into_iter()
///         .filter_map(|(name, param): (&str, #operon::schema_base::OptionCoordinate)| param.0.map(|p| (name, p)))
///         .map(|(name, param)| i64::try_from(param).map(|p| (name, p)))
///         .collect::<Result<Vec<_>, _>>()?;
///
///     let where_clause = if params.is_empty() {
///         String::new()
///     } else {
///         format!(
///             "WHERE {}",
///             params
///                 .iter()
///                 .enumerate()
///                 .map(|(i, (name, _))| format!("{} = ${}", name, i + 1))
///                 .collect::<Vec<_>>()
///                 .join(" AND "),
///         )
///     };
///     let params = params.into_iter().map(|(_, param)| param).collect::<Vec<_>>();
///     let pop_stmt = format!("DELETE FROM {schema_prefix}ticket_beta\n{where_clause}\nRETURNING *;");
///
///     let rows = client.query(
///         &pop_stmt,
///         &params.iter().map(|p| p as &(dyn operon::postgres_types::ToSql + Sync)).collect::<Vec<_>>()
///     ).await?;
///     let tickets = rows
///         .iter()
///         .map(<schema::BetaTicket as operon::schema_base::TicketSql>::from_sql_row)
///         .collect::<Result<Vec<_>, _>>()?;
///     let new_tickets = operon::futures::future::try_join_all(
///         tickets
///             .into_iter()
///             .map(|ticket| operon::schema_base::Ticket::raise_dependency_count(ticket, client))
///     )
///     .await?;
///
///     let copy_stmt = format!("COPY {schema_prefix}ticket_beta (\n    i, resolved, deps_count, deps_quota, deps_done, status\n)\nFROM STDIN WITH (FORMAT csv);");
///     let sink = client.copy_in::<_, operon::bytes::Bytes>(&copy_stmt).await?;
///     let mut sink = Box::pin(sink);
///     for ticket in &new_tickets {
///         operon::futures::SinkExt::feed(
///             &mut sink,
///             operon::schema_base::TicketSql::to_sql_copy_params(ticket)?.into(),
///         )
///         .await?;
///     }
///     operon::futures::SinkExt::close(&mut sink).await?;
///
///     let ready_tickets = new_tickets
///         .into_iter()
///         .filter(operon::schema_base::Ticket::is_ready)
///         .collect::<Vec<_>>();
///     Ok(ready_tickets)
/// }
/// ```
pub(super) fn fn_raise_dep(job: &JobConfig) -> syn::ItemFn {
    // TODO: only define raise_dep for a valid combination of jobs.
    let operon = operon_ident();
    let fn_name = raise_dep_ident(&job.id);
    let n = job.dims.len();
    let job_meta = job_metadata_ident(&job.id);
    let pop_query = RaiseDepPopQuery(job).to_string();
    let copy_query = RaiseDepCopyInQuery(job).to_string();

    let args = job.dims.iter().map(|dim| -> syn::FnArg {
        let arg = variable_ident(dim);
        parse_quote! {
            #arg: #operon::schema_base::OptionCoordinate
        }
    });
    let params = job.dims.iter().map(|dim| -> syn::Expr {
        let arg = variable_ident(dim);
        parse_quote! {
            (#dim, #arg)
        }
    });

    parse_quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
            #(#args,)*
        ) -> Result<Vec<#operon::schema_base::Ticket<#n>>, #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let params = [
                #(#params,)*
            ]
            .into_iter()
            .filter_map(|(name, param): (&str, #operon::schema_base::OptionCoordinate)| param.0.map(|p| (name, p)))
            .map(|(name, param)| i64::try_from(param).map(|p| (name, p)))
            .collect::<Result<Vec<_>, _>>()?;

            let where_clause = if params.is_empty() {
                String::new()
            } else {
                let where_clause = params
                    .iter()
                    .enumerate()
                    .map(|(i, (name, _))| format!("{} = ${}", name, i + 1))
                    .collect::<Vec<_>>()
                    .join(" AND ");
                format!("WHERE {where_clause}")
            };
            let params = params.into_iter().map(|(_, param)| param).collect::<Vec<_>>();
            let pop_stmt = format!(#pop_query);

            let rows = client.query(
                &pop_stmt,
                &params.iter().map(|p| p as &(dyn #operon::postgres_types::ToSql + Sync)).collect::<Vec<_>>()
            ).await?;
            let tickets = rows
                .iter()
                .map(|row| #operon::schema_base::Ticket::from_sql_row(metadata::#job_meta(), row))
                .collect::<Result<Vec<_>, _>>()?;
            let new_tickets = tickets
                .into_iter()
                .map(|ticket| ticket.raise_deps_done())
                .collect::<Vec<_>>();

            let copy_stmt = format!(#copy_query);
            let sink = client.copy_in::<_, #operon::bytes::Bytes>(&copy_stmt).await?;
            let mut sink = Box::pin(sink);
            for ticket in &new_tickets {
                #operon::futures::SinkExt::feed(
                    &mut sink,
                    ticket.to_copy_string()?.into(),
                )
                .await?;
            }
            #operon::futures::SinkExt::close(&mut sink).await?;

            let ready_tickets = new_tickets
                .into_iter()
                .filter(|ticket| ticket.is_ready())
                .collect::<Vec<_>>();
            Ok(ready_tickets)
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::job_beta;

    #[rstest]
    #[case::simple(job_beta(), "queries/ticket/raise_dep.rs")]
    fn test_fn_raise_dep(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let item = fn_raise_dep(&job);
        assert_item_eq(&item, fixture_path);
    }
}
