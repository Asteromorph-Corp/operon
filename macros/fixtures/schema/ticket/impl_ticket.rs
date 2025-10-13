#[operon::async_trait::async_trait]
#[automatically_derived]
impl operon::schema_base::Ticket for BetaTicket {
    type Job = schema::BetaJob;
    type Resolution = schema::JResolution;

    fn new() -> Self {
        Self {
            deps_quota: Some(0),
            deps_done: true,
            ..Default::default()
        }
    }

    #[allow(unused_variables)]
    async fn get_dependency_quota(
        &self,
        client: operon::meta_storage::MetaClient<'_>,
    ) -> Result<Option<usize>, operon::meta_storage::MetaStorageError> {
        Ok(Some(0usize))
    }

    async fn resolve_dependency_quota(
        self,
        client: operon::meta_storage::MetaClient<'_>,
    ) -> Result<Self, operon::meta_storage::MetaStorageError> {
        let mut ticket = self;
        if ticket.deps_quota.is_none() {
            ticket.deps_quota = ticket.get_dependency_quota(client).await?;
        }
        ticket.deps_done = ticket
            .deps_quota
            .is_some_and(|quota| ticket.deps_count >= quota);
        if ticket.is_ready() {
            ticket.status = operon::schema_base::TicketStatus::Queued;
        }
        Ok(ticket)
    }

    async fn raise_dependency_count(
        self,
        client: operon::meta_storage::MetaClient<'_>,
    ) -> Result<Self, operon::meta_storage::MetaStorageError> {
        let mut ticket = self;
        ticket.deps_count += 1;
        ticket.deps_done = ticket
            .deps_quota
            .is_some_and(|quota| ticket.deps_count >= quota);
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
            Some(schema::BetaJob { i: self.i.0? })
        } else {
            None
        }
    }
}
