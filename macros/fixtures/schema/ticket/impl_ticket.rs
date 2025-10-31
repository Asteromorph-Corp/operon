#[operon::async_trait::async_trait]
#[automatically_derived]
impl operon::schema_base::Ticket for BetaTicket {
    type Job = schema::BetaJob;
    type Resolution = schema::JResolution;

    fn new() -> Self {
        Self {
            deps_quota: 0usize,
            deps_done: true,
            ..Default::default()
        }
    }

    fn update_deps_done(mut self) -> Self {
        self.deps_done = self.deps_count >= self.deps_quota;
        if self.is_ready() {
            self.status = operon::schema_base::TicketStatus::Queued;
        }
        self
    }

    fn raise_dependency_count(mut self) -> Self {
        self.deps_count += 1;
        self.update_deps_done()
    }

    fn raise_dependency_quota(mut self, explosion_ub: usize) -> Self {
        self.deps_quota = self.deps_quota + explosion_ub - 1;
        self.update_deps_done()
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
