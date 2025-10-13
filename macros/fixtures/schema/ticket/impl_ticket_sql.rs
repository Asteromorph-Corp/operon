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
