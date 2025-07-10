use syn::parse_quote;

use crate::{
    DimensionConfig, JobConfig,
    utils::{
        explode_ident, operon_ident, resolution_ident, ticket_ident, variable_ident, with_ident,
    },
};

/// A helper struct to generate the SQL query for popping tickets to be exploded.
struct ExplodePopQuery<'a>(&'a JobConfig, &'a DimensionConfig);

impl std::fmt::Display for ExplodePopQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let job_id = &self.0.id;

        writeln!(f, "DELETE FROM {{schema_prefix}}ticket_{job_id}")?;
        for (i, dep) in self.1.depends_on.iter().enumerate() {
            if i == 0 {
                write!(f, "WHERE")?;
            } else {
                write!(f, "    AND")?;
            }
            writeln!(f, " {} = ${}", dep, i + 1)?;
        }
        write!(f, "RETURNING *;")
    }
}

/// A helper struct to generate the SQL query for copying exploded result into the database.
struct ExplodeCopyInQuery<'a>(&'a JobConfig);

impl std::fmt::Display for ExplodeCopyInQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let job_id = &self.0.id;

        writeln!(f, "COPY {{schema_prefix}}ticket_{job_id} (")?;
        write!(f, "    ")?;
        for dim in &self.0.dims {
            write!(f, "{dim}, ")?;
        }
        writeln!(f, "resolved, deps_count, deps_quota, deps_done, status")?;
        writeln!(f, ")")?;
        write!(f, "FROM STDIN WITH (FORMAT csv);")
    }
}

pub(super) fn fn_explode(job: &JobConfig, dim: &DimensionConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = explode_ident(&job.id, &dim.id);
    let ticket_ident = ticket_ident(&job.id);
    let res_ident = resolution_ident(&dim.id);
    let field_ident = variable_ident(&dim.id);
    let with_fn_name = with_ident(&dim.id);
    let pop_query = ExplodePopQuery(job, dim).to_string();
    let copy_query = ExplodeCopyInQuery(job).to_string();

    let indices = (1..=dim.depends_on.len()).map(syn::Index::from);

    let err_msg = format!(
        "Called `explode({})` on `{}`, but `{}` was resolved",
        dim.id, job.id, dim.id
    );

    parse_quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
            resolution: &schema::#res_ident,
        ) -> Result<Vec<schema::#ticket_ident>, #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let pop_stmt = format!(#pop_query);

            let rows = client.query(&pop_stmt, &[#(&i64::try_from(resolution.#indices)?,)*]).await?;
            let tickets = rows
                .iter()
                .map(<schema::#ticket_ident as #operon::schema_base::TicketSql>::from_sql_row)
                .collect::<Result<Vec<_>, _>>()?;

            if tickets.iter().any(|ticket| ticket.#field_ident.is_some()) {
                return Err(#operon::meta_storage::MetaStorageError::InvalidResolution(
                    #err_msg.into(),
                ));
            }
            let new_tickets = tickets
                .iter()
                .flat_map(|ticket| (0..resolution.0).map(|ub| ticket.clone().#with_fn_name(ub)))
                .collect::<Vec<_>>();

            let copy_stmt = format!(#copy_query);
            let sink = client.copy_in::<_, #operon::bytes::Bytes>(&copy_stmt).await?;
            let mut sink = Box::pin(sink);
            for ticket in &new_tickets {
                #operon::futures::SinkExt::feed(
                    &mut sink,
                    #operon::schema_base::TicketSql::to_sql_copy_params(ticket)?.into(),
                )
                .await?;
            }
            #operon::futures::SinkExt::close(&mut sink).await?;

            let ready_tickets = new_tickets
                .into_iter()
                .filter(#operon::schema_base::Ticket::is_ready)
                .collect::<Vec<_>>();
            Ok(ready_tickets)
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use crate::configs::JobArg;

    use super::*;

    #[test]
    fn test_explode_pop_query() {
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
        let i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };
        let query = ExplodePopQuery(&beta, &i).to_string();
        let expected = indoc! { "
            DELETE FROM {schema_prefix}ticket_beta
            RETURNING *;"};
        assert_eq!(query, expected);

        let epsilon = JobConfig {
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
        let k = DimensionConfig {
            id: "k".to_string(),
            depends_on: vec!["i".to_string()],
        };

        let query = ExplodePopQuery(&epsilon, &k).to_string();
        let expected = indoc! { "
            DELETE FROM {schema_prefix}ticket_epsilon
            WHERE i = $1
            RETURNING *;"};
        assert_eq!(query, expected);
    }

    #[test]
    fn test_explode_copy_in_query() {
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

        let query = ExplodeCopyInQuery(&beta).to_string();
        let expected = indoc! { "
            COPY {schema_prefix}ticket_beta (
                i, resolved, deps_count, deps_quota, deps_done, status
            )
            FROM STDIN WITH (FORMAT csv);"
        };
        assert_eq!(query, expected);
    }

    #[test]
    fn test_explode_fn() {
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
        let dim = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let pop_stmt = indoc! {"
            DELETE FROM {schema_prefix}ticket_beta
            RETURNING *;"
        };
        let copy_stmt = indoc! {"
            COPY {schema_prefix}ticket_beta (
                i, resolved, deps_count, deps_quota, deps_done, status
            )
            FROM STDIN WITH (FORMAT csv);"
        };

        let item = fn_explode(&job, &dim);
        let expected: syn::ItemFn = parse_quote! {
            pub async fn explode_beta_i(
                client: operon::meta_storage::MetaClient<'_>,
                resolution: &schema::IResolution,
            ) -> Result<Vec<schema::BetaTicket>, operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                // Statement to select all tickets that match the *parent dimensions* in the resolution.
                // (In this case, there are none.)
                let pop_stmt = format!(#pop_stmt);

                let rows = client.query(&pop_stmt, &[]).await?;
                let tickets = rows
                    .iter()
                    .map(<schema::BetaTicket as operon::schema_base::TicketSql>::from_sql_row)
                    .collect::<Result<Vec<_>, _>>()?;

                if tickets.iter().any(|ticket| ticket.i.is_some()) {
                    return Err(operon::meta_storage::MetaStorageError::InvalidResolution(
                        "Called `explode(i)` on `beta`, but `i` was resolved".into(),
                    ));
                }
                let new_tickets = tickets
                    .iter()
                    .flat_map(|ticket| (0..resolution.0).map(|ub| ticket.clone().with_i(ub)))
                    .collect::<Vec<_>>();

                let copy_stmt = format!(#copy_stmt);
                let sink = client
                    .copy_in::<_, operon::bytes::Bytes>(&copy_stmt)
                    .await?;
                let mut sink = Box::pin(sink);
                for ticket in &new_tickets {
                    operon::futures::SinkExt::feed(
                        &mut sink,
                        operon::schema_base::TicketSql::to_sql_copy_params(ticket)?.into(),
                    )
                    .await?;
                }
                operon::futures::SinkExt::close(&mut sink).await?;

                let ready_tickets = new_tickets
                    .into_iter()
                    .filter(operon::schema_base::Ticket::is_ready)
                    .collect::<Vec<_>>();
                Ok(ready_tickets)
            }
        };

        assert_eq!(item, expected);
    }
}
